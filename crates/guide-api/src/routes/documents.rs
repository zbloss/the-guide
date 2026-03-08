use axum::{
    body::Bytes,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use guide_core::{
    models::{CampaignDocument, DocumentKind, GlobalDocument, IngestionStatus},
    GuideError,
};
use guide_db::documents::{DocumentRepository, GlobalDocumentRepository};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Campaign documents
        .route(
            "/campaigns/{campaign_id}/documents",
            get(list_documents).post(upload_document),
        )
        .route(
            "/campaigns/{campaign_id}/documents/{doc_id}",
            get(get_document),
        )
        .route(
            "/campaigns/{campaign_id}/documents/{doc_id}/ingest",
            post(ingest_document),
        )
        // Global (rulebook) documents
        .route("/documents", get(list_global).post(upload_global))
        .route("/documents/{doc_id}", get(get_global))
        .route("/documents/{doc_id}/ingest", post(ingest_global))
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/documents",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    responses(
        (status = 200, description = "List all documents in a campaign", body = [CampaignDocument])
    )
)]
async fn list_documents(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DocumentRepository::new(&state.db);
    Ok(Json(repo.list_by_campaign(campaign_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/documents",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Document uploaded successfully", body = CampaignDocument)
    )
)]
async fn upload_document(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let (filename, data) = extract_file_from_multipart(&mut multipart).await?;
    let doc_id = Uuid::new_v4();
    let stored_path = format!("data/uploads/{doc_id}/{filename}");

    save_file(&stored_path, &data).await?;

    let doc = CampaignDocument {
        id: doc_id,
        campaign_id,
        filename: filename.clone(),
        file_size_bytes: data.len() as i64,
        stored_path,
        page_count: None,
        document_kind: DocumentKind::Campaign,
        ingestion_status: IngestionStatus::Pending,
        ingestion_error: None,
        uploaded_at: Utc::now(),
        ingested_at: None,
    };

    let repo = DocumentRepository::new(&state.db);
    repo.insert(&doc).await?;
    Ok((StatusCode::CREATED, Json(doc)))
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/documents/{doc_id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("doc_id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Found document", body = CampaignDocument),
        (status = 404, description = "Document not found")
    )
)]
async fn get_document(
    State(state): State<AppState>,
    Path((_campaign_id, doc_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DocumentRepository::new(&state.db);
    Ok(Json(repo.get_by_id(doc_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/documents/{doc_id}/ingest",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("doc_id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 202, description = "Ingestion started")
    )
)]
async fn ingest_document(
    State(state): State<AppState>,
    Path((_campaign_id, doc_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DocumentRepository::new(&state.db);
    let doc = repo.get_by_id(doc_id).await?;
    let stored_path = doc.stored_path.clone();

    let llm = state.llm.clone();
    let config = state.config.as_ref().clone();
    let qdrant = state.qdrant.clone();
    let db = state.db.clone();

    tokio::spawn(async move {
        let path = std::path::Path::new(&stored_path);
        let result = guide_pdf::pipeline::ingest_campaign_document(
            path,
            &doc,
            llm,
            &config,
            qdrant.as_deref(),
            &db,
        )
        .await;

        if let Err(e) = result {
            tracing::error!("Ingestion failed for doc {doc_id}: {e}");
            let repo = DocumentRepository::new(&db);
            let _ = repo
                .update_status(doc_id, &IngestionStatus::Failed, Some(&e.to_string()))
                .await;
        }
    });

    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    get,
    path = "/documents",
    responses(
        (status = 200, description = "List all global documents", body = [GlobalDocument])
    )
)]
async fn list_global(State(state): State<AppState>) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = GlobalDocumentRepository::new(&state.db);
    Ok(Json(repo.list_all().await?))
}

#[utoipa::path(
    post,
    path = "/documents",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Global document uploaded successfully", body = GlobalDocument)
    )
)]
async fn upload_global(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let (filename, data) = extract_file_from_multipart(&mut multipart).await?;
    let doc_id = Uuid::new_v4();
    let stored_path = format!("data/uploads/global/{doc_id}/{filename}");

    save_file(&stored_path, &data).await?;

    let title = filename
        .strip_suffix(".pdf")
        .unwrap_or(&filename)
        .to_string();
    let doc = GlobalDocument {
        id: doc_id,
        title,
        filename: filename.clone(),
        file_size_bytes: data.len() as i64,
        stored_path,
        page_count: None,
        ingestion_status: IngestionStatus::Pending,
        ingestion_error: None,
        uploaded_at: Utc::now(),
        ingested_at: None,
    };

    let repo = GlobalDocumentRepository::new(&state.db);
    repo.insert(&doc).await?;
    Ok((StatusCode::CREATED, Json(doc)))
}

#[utoipa::path(
    get,
    path = "/documents/{doc_id}",
    params(
        ("doc_id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Found global document", body = GlobalDocument),
        (status = 404, description = "Document not found")
    )
)]
async fn get_global(
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = GlobalDocumentRepository::new(&state.db);
    Ok(Json(repo.get_by_id(doc_id).await?))
}

#[utoipa::path(
    post,
    path = "/documents/{doc_id}/ingest",
    params(
        ("doc_id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 202, description = "Ingestion started")
    )
)]
async fn ingest_global(
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = GlobalDocumentRepository::new(&state.db);
    let doc = repo.get_by_id(doc_id).await?;
    let stored_path = doc.stored_path.clone();

    let llm = state.llm.clone();
    let config = state.config.as_ref().clone();
    let qdrant = state.qdrant.clone();
    let db = state.db.clone();

    tokio::spawn(async move {
        let path = std::path::Path::new(&stored_path);
        let result = guide_pdf::pipeline::ingest_global_document(
            path,
            &doc,
            llm,
            &config,
            qdrant.as_deref(),
            &db,
        )
        .await;

        if let Err(e) = result {
            tracing::error!("Global ingestion failed for doc {doc_id}: {e}");
            let repo = GlobalDocumentRepository::new(&db);
            let _ = repo
                .update_status(doc_id, &IngestionStatus::Failed, Some(&e.to_string()))
                .await;
        }
    });

    Ok(StatusCode::ACCEPTED)
}

async fn extract_file_from_multipart(
    multipart: &mut Multipart,
) -> Result<(String, Bytes), GuideError> {
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| GuideError::InvalidInput(e.to_string()))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload.pdf".to_string());
        let data = field
            .bytes()
            .await
            .map_err(|e| GuideError::InvalidInput(e.to_string()))?;
        return Ok((filename, data));
    }
    Err(GuideError::InvalidInput("No file provided".into()))
}

async fn save_file(path: &str, data: &[u8]) -> Result<(), GuideError> {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, data).await?;
    Ok(())
}
