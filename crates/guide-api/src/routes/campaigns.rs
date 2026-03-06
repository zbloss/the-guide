use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use guide_core::models::{CreateCampaignRequest, UpdateCampaignRequest};
use guide_db::campaigns::CampaignRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/campaigns", get(list_campaigns).post(create_campaign))
        .route(
            "/campaigns/{id}",
            get(get_campaign).put(update_campaign).delete(delete_campaign),
        )
}

async fn list_campaigns(State(state): State<AppState>) -> impl IntoResponse {
    let repo = CampaignRepository::new(&state.db);
    match repo.list().await {
        Ok(campaigns) => (StatusCode::OK, Json(campaigns)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn create_campaign(
    State(state): State<AppState>,
    Json(req): Json<CreateCampaignRequest>,
) -> impl IntoResponse {
    let repo = CampaignRepository::new(&state.db);
    match repo.create(req).await {
        Ok(campaign) => {
            // Phase 1: also create Qdrant collection
            if let Some(qdrant) = &state.qdrant {
                let vector_size = state.config.qdrant.vector_size;
                let campaign_id = campaign.id.to_string();
                let qdrant = qdrant.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        guide_db::qdrant::create_campaign_collection(&qdrant, &campaign_id, vector_size).await
                    {
                        tracing::warn!("Failed to create Qdrant collection: {e}");
                    }
                });
            }
            (StatusCode::CREATED, Json(campaign)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_campaign(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = CampaignRepository::new(&state.db);
    match repo.get_by_id(id).await {
        Ok(campaign) => (StatusCode::OK, Json(campaign)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn update_campaign(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCampaignRequest>,
) -> impl IntoResponse {
    let repo = CampaignRepository::new(&state.db);
    match repo.update(id, req).await {
        Ok(campaign) => (StatusCode::OK, Json(campaign)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn delete_campaign(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = CampaignRepository::new(&state.db);
    match repo.delete(id).await {
        Ok(()) => {
            // Drop Qdrant collection
            if let Some(qdrant) = &state.qdrant {
                let campaign_id = id.to_string();
                let qdrant = qdrant.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        guide_db::qdrant::delete_campaign_collection(&qdrant, &campaign_id).await
                    {
                        tracing::warn!("Failed to delete Qdrant collection: {e}");
                    }
                });
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(guide_core::GuideError::NotFound(msg)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
