use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::models::{Campaign, CreateCampaignRequest, UpdateCampaignRequest};
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

#[utoipa::path(
    get,
    path = "/campaigns",
    responses(
        (status = 200, description = "List all campaigns", body = [Campaign])
    )
)]
async fn list_campaigns(State(state): State<AppState>) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignRepository::new(&state.db);
    let campaigns = repo.list().await?;
    Ok(Json(campaigns))
}

#[utoipa::path(
    post,
    path = "/campaigns",
    request_body = CreateCampaignRequest,
    responses(
        (status = 201, description = "Campaign created successfully", body = Campaign)
    )
)]
async fn create_campaign(
    State(state): State<AppState>,
    Json(req): Json<CreateCampaignRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignRepository::new(&state.db);
    let campaign = repo.create(req).await?;
    Ok((StatusCode::CREATED, Json(campaign)))
}

#[utoipa::path(
    get,
    path = "/campaigns/{id}",
    params(
        ("id" = Uuid, Path, description = "Campaign ID")
    ),
    responses(
        (status = 200, description = "Found campaign", body = Campaign),
        (status = 404, description = "Campaign not found")
    )
)]
async fn get_campaign(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignRepository::new(&state.db);
    let campaign = repo.get_by_id(id).await?;
    Ok(Json(campaign))
}

#[utoipa::path(
    put,
    path = "/campaigns/{id}",
    params(
        ("id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = UpdateCampaignRequest,
    responses(
        (status = 200, description = "Campaign updated successfully", body = Campaign),
        (status = 404, description = "Campaign not found")
    )
)]
async fn update_campaign(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCampaignRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignRepository::new(&state.db);
    let campaign = repo.update(id, req).await?;
    Ok(Json(campaign))
}

#[utoipa::path(
    delete,
    path = "/campaigns/{id}",
    params(
        ("id" = Uuid, Path, description = "Campaign ID")
    ),
    responses(
        (status = 204, description = "Campaign deleted successfully"),
        (status = 404, description = "Campaign not found")
    )
)]
async fn delete_campaign(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignRepository::new(&state.db);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
