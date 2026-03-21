use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use guide_core::{
    models::{CampaignDocument, DocumentKind, GlobalDocument, IngestionStatus, RankedChunk},
    GuideError,
};
use guide_db::{
    campaign_global_docs::CampaignGlobalDocRepository,
    documents::{parse_doc_kind, DocumentRepository, GlobalDocumentRepository},
    PageOcrRow,
};
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
            get(get_document).delete(delete_campaign_document),
        )
        .route(
            "/campaigns/{campaign_id}/documents/{doc_id}/ingest",
            post(ingest_document),
        )
        .route(
            "/campaigns/{campaign_id}/documents/{doc_id}/search",
            get(search_campaign_document),
        )
        .route(
            "/campaigns/{campaign_id}/documents/{doc_id}/pages",
            get(get_document_pages),
        )
        .route(
            "/campaigns/{campaign_id}/documents/{doc_id}/story-extraction-status",
            get(story_extraction_status),
        )
        // Campaign ↔ GlobalDoc associations
        .route(
            "/campaigns/{campaign_id}/global-docs",
            get(list_campaign_global_docs).post(associate_global_doc),
        )
        .route(
            "/campaigns/{campaign_id}/global-docs/{gdoc_id}",
            delete(remove_global_doc),
        )
        // Global (rulebook) documents
        .route("/documents", get(list_global).post(upload_global))
        .route("/documents/{doc_id}", get(get_global).delete(delete_global_document))
        .route("/documents/{doc_id}/ingest", post(ingest_global))
        // Rules search (global PageIndex)
        .route("/rules/search", get(search_rules))
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
    let mut filename = String::new();
    let mut data = Bytes::new();
    let mut kind_str: Option<String> = None;
    let mut description: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| GuideError::InvalidInput(e.to_string()))?
    {
        match field.name() {
            Some("file") => {
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "upload.pdf".to_string());
                data = field
                    .bytes()
                    .await
                    .map_err(|e| GuideError::InvalidInput(e.to_string()))?;
            }
            Some("kind") => {
                kind_str = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| GuideError::InvalidInput(e.to_string()))?,
                );
            }
            Some("description") => {
                description = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| GuideError::InvalidInput(e.to_string()))?,
                );
            }
            _ => {}
        }
    }

    if filename.is_empty() {
        return Err(GuideError::InvalidInput("No file provided".into()).into());
    }

    let document_kind = kind_str
        .as_deref()
        .map(parse_doc_kind)
        .unwrap_or(DocumentKind::Campaign);

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
        document_kind,
        description,
        ingestion_status: IngestionStatus::Pending,
        ingestion_error: None,
        story_extraction_status: "pending".to_string(),
        story_extraction_error: None,
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
    let mut filename = String::new();
    let mut data = Bytes::new();
    let mut kind_str: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| GuideError::InvalidInput(e.to_string()))?
    {
        match field.name() {
            Some("file") => {
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "upload.pdf".to_string());
                data = field
                    .bytes()
                    .await
                    .map_err(|e| GuideError::InvalidInput(e.to_string()))?;
            }
            Some("kind") => {
                kind_str = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| GuideError::InvalidInput(e.to_string()))?,
                );
            }
            _ => {}
        }
    }

    if filename.is_empty() {
        return Err(GuideError::InvalidInput("No file provided".into()).into());
    }

    let document_kind = kind_str
        .as_deref()
        .map(parse_doc_kind)
        .unwrap_or(DocumentKind::DmGuide);

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
        document_kind,
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

async fn delete_campaign_document(
    State(state): State<AppState>,
    Path((campaign_id, doc_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DocumentRepository::new(&state.db);
    let doc = repo.get_by_id(doc_id).await?;

    if let Some(q) = state.qdrant.as_deref() {
        let col = guide_db::qdrant::campaign_collection_name(&campaign_id.to_string());
        if let Err(e) = guide_db::qdrant::delete_document_vectors(q, &col, doc_id).await {
            tracing::warn!("Failed to delete Qdrant vectors for doc {doc_id}: {e}");
        }
    }

    if let Err(e) = tokio::fs::remove_file(format!("data/indexes/{}/{}.json", campaign_id, doc_id)).await {
        tracing::warn!("Failed to remove index file for doc {doc_id}: {e}");
    }

    if let Some(parent) = std::path::Path::new(&doc.stored_path).parent() {
        if let Err(e) = tokio::fs::remove_dir_all(parent).await {
            tracing::warn!("Failed to remove upload dir for doc {doc_id}: {e}");
        }
    }

    repo.delete(doc_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_global_document(
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = GlobalDocumentRepository::new(&state.db);
    let doc = repo.get_by_id(doc_id).await?;

    if let Some(q) = state.qdrant.as_deref() {
        let col = guide_db::qdrant::global_collection_name();
        if let Err(e) = guide_db::qdrant::delete_document_vectors(q, col, doc_id).await {
            tracing::warn!("Failed to delete Qdrant vectors for global doc {doc_id}: {e}");
        }
    }

    if let Err(e) = tokio::fs::remove_file(format!("data/indexes/global/{}.json", doc_id)).await {
        tracing::warn!("Failed to remove index file for global doc {doc_id}: {e}");
    }

    if let Some(parent) = std::path::Path::new(&doc.stored_path).parent() {
        if let Err(e) = tokio::fs::remove_dir_all(parent).await {
            tracing::warn!("Failed to remove upload dir for global doc {doc_id}: {e}");
        }
    }

    repo.delete(doc_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── OCR Page Preview ─────────────────────────────────────────────────────────

/// GET /campaigns/:campaign_id/documents/:doc_id/pages
///
/// Returns the raw OCR text for every page of a campaign document,
/// ordered by page number ascending.
async fn get_document_pages(
    State(state): State<AppState>,
    Path((_campaign_id, doc_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<PageOcrRow>>, crate::error::AppError> {
    let repo = DocumentRepository::new(&state.db);
    let rows = repo.list_page_ocr(doc_id).await?;
    Ok(Json(rows))
}

// ─── Story Extraction Status ──────────────────────────────────────────────────

#[derive(Serialize)]
struct StoryExtractionStatusResponse {
    status: String,
    error: Option<String>,
}

async fn story_extraction_status(
    State(state): State<AppState>,
    Path((_campaign_id, doc_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DocumentRepository::new(&state.db);
    let doc = repo.get_by_id(doc_id).await?;
    Ok(Json(StoryExtractionStatusResponse {
        status: doc.story_extraction_status,
        error: doc.story_extraction_error,
    }))
}

// ─── Campaign ↔ GlobalDoc Associations ────────────────────────────────────────

#[derive(Deserialize)]
struct AssociateGlobalDocRequest {
    global_doc_id: Uuid,
}

async fn list_campaign_global_docs(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignGlobalDocRepository::new(&state.db);
    Ok(Json(repo.list_for_campaign(campaign_id).await?))
}

async fn associate_global_doc(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(body): Json<AssociateGlobalDocRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignGlobalDocRepository::new(&state.db);
    repo.associate(campaign_id, body.global_doc_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_global_doc(
    State(state): State<AppState>,
    Path((campaign_id, gdoc_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignGlobalDocRepository::new(&state.db);
    repo.remove(campaign_id, gdoc_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

async fn save_file(path: &str, data: &[u8]) -> Result<(), GuideError> {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, data).await?;
    Ok(())
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search_rules(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_pdf::pipeline::query_indexes;
    if params.q.trim().is_empty() {
        return Ok(Json(Vec::<RankedChunk>::new()));
    }
    let chunks = query_indexes(
        &params.q,
        None,
        false,
        state.llm.as_ref(),
        &state.config,
        state.qdrant.as_deref(),
    )
    .await
    .unwrap_or_default();
    Ok(Json(chunks))
}

async fn search_campaign_document(
    State(state): State<AppState>,
    Path((campaign_id, _doc_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_pdf::pipeline::query_indexes;
    if params.q.trim().is_empty() {
        return Ok(Json(Vec::<RankedChunk>::new()));
    }
    let chunks = query_indexes(
        &params.q,
        Some(campaign_id),
        false,
        state.llm.as_ref(),
        &state.config,
        state.qdrant.as_deref(),
    )
    .await
    .unwrap_or_default();
    Ok(Json(chunks))
}
