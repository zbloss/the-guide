use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use guide_core::models::{CampaignDocument, DocumentKind, IngestionStatus};
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

    let max_bytes = state.config.upload.max_upload_bytes;
    let mut filename: Option<String> = None;
    let mut file_bytes: Vec<u8> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field.file_name().map(str::to_string);
            file_bytes = match field.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": format!("Failed to read file data: {e}") })),
                    )
                        .into_response()
                }
            };
        }
    }

    if file_bytes.len() as u64 > max_bytes {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": format!("File exceeds maximum allowed size of {} bytes", max_bytes)
            })),
        )
            .into_response();
    }

    let filename = match filename {
        Some(f) if !f.is_empty() => f,
        _ => {
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
    let doc_filename = doc.filename.clone();
    let ocr_model = state.config.llm.ocr_model.clone();
    let ingestion_cfg = state.config.ingestion.clone();

    tokio::spawn(async move {
        let path = std::path::PathBuf::from(&stored_path);
        let ingest_future = guide_pdf::ingest_document(
            &path,
            DocumentKind::Campaign,
            Some(campaign_id),
            doc_id,
            &doc_filename,
            true,
            llm,
            &ocr_model,
            &ingestion_cfg,
            &qdrant,
            &db,
        );

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(300),
            ingest_future,
        )
        .await;

        let repo = DocumentRepository::new(&db);
        match result {
            Ok(Ok(count)) => {
                tracing::info!(
                    "Ingested {count} chunks for doc {doc_id} in campaign {campaign_id}"
                );
            }
            Ok(Err(e)) => {
                tracing::error!("Ingestion failed for doc {doc_id}: {e}");
                let _ = repo
                    .update_status(doc_id, &IngestionStatus::Failed, Some(&e.to_string()))
                    .await;
            }
            Err(_elapsed) => {
                tracing::error!("Ingestion timed out for doc {doc_id} after 300 seconds");
                let _ = repo
                    .update_status(
                        doc_id,
                        &IngestionStatus::Failed,
                        Some("Ingestion timed out after 300 seconds"),
                    )
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
