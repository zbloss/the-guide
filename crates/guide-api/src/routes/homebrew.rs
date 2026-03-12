use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::models::{CreateHomebrewRuleRequest, HomebrewRule};
use guide_db::homebrew::HomebrewRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/homebrew",
            get(list_rules).post(create_rule),
        )
        .route(
            "/campaigns/{campaign_id}/homebrew/{id}",
            axum::routing::delete(delete_rule),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/homebrew",
    params(("campaign_id" = Uuid, Path, description = "Campaign ID")),
    responses((status = 200, description = "List all homebrew rules", body = [HomebrewRule]))
)]
async fn list_rules(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = HomebrewRepository::new(&state.db);
    Ok(Json(repo.list_by_campaign(campaign_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/homebrew",
    params(("campaign_id" = Uuid, Path, description = "Campaign ID")),
    request_body = CreateHomebrewRuleRequest,
    responses((status = 201, description = "Homebrew rule created", body = HomebrewRule))
)]
async fn create_rule(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateHomebrewRuleRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = HomebrewRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(campaign_id, req).await?)))
}

async fn delete_rule(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = HomebrewRepository::new(&state.db);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
