use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use guide_core::models::{CampaignDocument, IngestionStatus};
use guide_db::{campaigns::CampaignRepository, documents::DocumentRepository};
use guide_llm::LlmClient;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
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
}

async fn list_documents(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> impl IntoResponse {
    let campaign_repo = CampaignRepository::new(&state.db);
    if let Err(e) = campaign_repo.get_by_id(campaign_id).await {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    let doc_repo = DocumentRepository::new(&state.db);
    match doc_repo.list_by_campaign(campaign_id).await {
        Ok(docs) => (StatusCode::OK, Json(docs)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn upload_document(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let campaign_repo = CampaignRepository::new(&state.db);
    if let Err(e) = campaign_repo.get_by_id(campaign_id).await {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    let mut filename: Option<String> = None;
    let mut file_bytes: Vec<u8> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field.file_name().map(str::to_string);
            file_bytes = field.bytes().await.unwrap_or_default().to_vec();
        }
    }

    let filename = match filename {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Missing 'file' field in multipart form" })),
            )
                .into_response()
        }
    };

    let doc_id = Uuid::new_v4();
    let dir = format!("data/documents/{campaign_id}");
    let stored_path = format!("{dir}/{doc_id}.pdf");

    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to create directory: {e}") })),
        )
            .into_response();
    }

    if let Err(e) = tokio::fs::write(&stored_path, &file_bytes).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to write file: {e}") })),
        )
            .into_response();
    }

    let doc = CampaignDocument {
        id: doc_id,
        campaign_id,
        filename,
        file_size_bytes: file_bytes.len() as i64,
        stored_path,
        page_count: None,
        ingestion_status: IngestionStatus::Pending,
        ingestion_error: None,
        uploaded_at: Utc::now(),
        ingested_at: None,
    };

    let doc_repo = DocumentRepository::new(&state.db);
    match doc_repo.insert(&doc).await {
        Ok(saved) => (StatusCode::CREATED, Json(saved)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_document(
    State(state): State<AppState>,
    Path((campaign_id, doc_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let doc_repo = DocumentRepository::new(&state.db);
    match doc_repo.get_by_id(doc_id).await {
        Ok(doc) if doc.campaign_id == campaign_id => (StatusCode::OK, Json(doc)).into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Document not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Trigger async OCR ingestion for an uploaded document.
/// Returns 202 Accepted; ingestion runs in the background.
async fn ingest_document(
    State(state): State<AppState>,
    Path((campaign_id, doc_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let qdrant = match &state.qdrant {
        Some(q) => Arc::clone(q),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "Qdrant is not available" })),
            )
                .into_response()
        }
    };

    // Load and validate document belongs to campaign
    let doc_repo = DocumentRepository::new(&state.db);
    let doc = match doc_repo.get_by_id(doc_id).await {
        Ok(d) if d.campaign_id == campaign_id => d,
        Ok(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Document not found" })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    // Mark as processing
    if let Err(e) = doc_repo.update_status(doc_id, &IngestionStatus::Processing, None).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    // Ensure Qdrant collection exists
    if let Err(e) =
        guide_db::qdrant::create_campaign_collection(&qdrant, &campaign_id.to_string(), 768).await
    {
        let _ = doc_repo
            .update_status(doc_id, &IngestionStatus::Failed, Some(&e.to_string()))
            .await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    // Clone everything needed for the background task
    let db = state.db.clone();
    let llm: Arc<dyn LlmClient> = state.llm.clone();
    let stored_path = doc.stored_path.clone();

    tokio::spawn(async move {
        let path = std::path::PathBuf::from(&stored_path);
        match guide_pdf::ingest_document(&path, campaign_id, doc_id, true, llm, &qdrant, &db)
            .await
        {
            Ok(count) => {
                tracing::info!("Ingested {count} chunks for doc {doc_id} in campaign {campaign_id}");
            }
            Err(e) => {
                tracing::error!("Ingestion failed for doc {doc_id}: {e}");
                let repo = DocumentRepository::new(&db);
                let _ = repo
                    .update_status(doc_id, &IngestionStatus::Failed, Some(&e.to_string()))
                    .await;
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "processing",
            "doc_id": doc_id,
        })),
    )
        .into_response()
}
