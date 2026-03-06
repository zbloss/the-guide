use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use guide_core::models::{DocumentKind, GlobalDocument, IngestionStatus};
use guide_db::documents::GlobalDocumentRepository;
use guide_llm::LlmClient;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/documents/global", get(list_global_documents).post(upload_global_document))
        .route("/documents/global/{doc_id}", get(get_global_document))
        .route("/documents/global/{doc_id}/ingest", post(ingest_global_document))
}

async fn list_global_documents(State(state): State<AppState>) -> impl IntoResponse {
    let repo = GlobalDocumentRepository::new(&state.db);
    match repo.list_all().await {
        Ok(docs) => (StatusCode::OK, Json(docs)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn upload_global_document(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let max_bytes = state.config.upload.max_upload_bytes;
    let mut filename: Option<String> = None;
    let mut title: Option<String> = None;
    let mut file_bytes: Vec<u8> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                filename = field.file_name().map(str::to_string);
                file_bytes = match field.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({ "error": format!("Failed to read file: {e}") })),
                        )
                            .into_response()
                    }
                };
            }
            Some("title") => {
                title = field.text().await.ok();
            }
            _ => {}
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

    let doc_title = title.unwrap_or_else(|| filename.clone());
    let doc_id = Uuid::new_v4();
    let dir = "data/documents/global".to_string();
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

    let doc = GlobalDocument {
        id: doc_id,
        title: doc_title,
        filename,
        file_size_bytes: file_bytes.len() as i64,
        stored_path,
        page_count: None,
        ingestion_status: IngestionStatus::Pending,
        ingestion_error: None,
        uploaded_at: Utc::now(),
        ingested_at: None,
    };

    let repo = GlobalDocumentRepository::new(&state.db);
    match repo.insert(&doc).await {
        Ok(saved) => (StatusCode::CREATED, Json(saved)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_global_document(
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = GlobalDocumentRepository::new(&state.db);
    match repo.get_by_id(doc_id).await {
        Ok(doc) => (StatusCode::OK, Json(doc)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Trigger async OCR ingestion into the global_rules Qdrant collection.
/// Returns 202 Accepted; ingestion runs in the background.
async fn ingest_global_document(
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
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

    let repo = GlobalDocumentRepository::new(&state.db);
    let doc = match repo.get_by_id(doc_id).await {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    if let Err(e) = repo.update_status(doc_id, &IngestionStatus::Processing, None).await {
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
    let doc_title = doc.title.clone();
    let ocr_model = state.config.llm.ocr_model.clone();
    let ingestion_cfg = state.config.ingestion.clone();

    tokio::spawn(async move {
        let path = std::path::PathBuf::from(&stored_path);
        let ingest_future = guide_pdf::ingest_document(
            &path,
            DocumentKind::Rulebook,
            None,
            doc_id,
            &doc_title,
            true,
            llm,
            &ocr_model,
            &ingestion_cfg,
            &qdrant,
            &db,
        );

        let result = tokio::time::timeout(std::time::Duration::from_secs(3600), ingest_future).await;

        let repo = GlobalDocumentRepository::new(&db);
        match result {
            Ok(Ok(count)) => {
                tracing::info!("Ingested {count} chunks for global doc {doc_id}");
            }
            Ok(Err(e)) => {
                tracing::error!("Ingestion failed for global doc {doc_id}: {e}");
                let _ = repo
                    .update_status(doc_id, &IngestionStatus::Failed, Some(&e.to_string()))
                    .await;
            }
            Err(_elapsed) => {
                tracing::error!("Ingestion timed out for global doc {doc_id}");
                let _ = repo
                    .update_status(
                        doc_id,
                        &IngestionStatus::Failed,
                        Some("Ingestion timed out after 3600 seconds"),
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
