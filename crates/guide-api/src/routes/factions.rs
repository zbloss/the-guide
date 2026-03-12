use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::models::{CreateFactionRequest, Faction, UpdateFactionReputationRequest};
use guide_db::factions::FactionRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{id}/factions",
            get(list_factions).post(create_faction),
        )
        .route(
            "/campaigns/{id}/factions/{faction_id}/reputation",
            axum::routing::put(update_reputation),
        )
        .route(
            "/campaigns/{id}/factions/{faction_id}",
            axum::routing::delete(delete_faction),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{id}/factions",
    params(("id" = Uuid, Path, description = "Campaign ID")),
    responses((status = 200, description = "List all factions", body = [Faction]))
)]
async fn list_factions(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = FactionRepository::new(&state.db);
    Ok(Json(repo.list_by_campaign(campaign_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{id}/factions",
    params(("id" = Uuid, Path, description = "Campaign ID")),
    request_body = CreateFactionRequest,
    responses((status = 201, description = "Faction created", body = Faction))
)]
async fn create_faction(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateFactionRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = FactionRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(campaign_id, req).await?)))
}

#[utoipa::path(
    put,
    path = "/campaigns/{id}/factions/{faction_id}/reputation",
    params(
        ("id" = Uuid, Path, description = "Campaign ID"),
        ("faction_id" = Uuid, Path, description = "Faction ID"),
    ),
    request_body = UpdateFactionReputationRequest,
    responses((status = 200, description = "Reputation updated", body = Faction))
)]
async fn update_reputation(
    State(state): State<AppState>,
    Path((_campaign_id, faction_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateFactionReputationRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = FactionRepository::new(&state.db);
    Ok(Json(repo.update_reputation(faction_id, req).await?))
}

async fn delete_faction(
    State(state): State<AppState>,
    Path((_campaign_id, faction_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = FactionRepository::new(&state.db);
    repo.delete(faction_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
