use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::models::{CreateLootItemRequest, LootItem};
use guide_db::loot::LootRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/sessions/{session_id}/loot",
            get(list_loot).post(create_loot_item),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{session_id}/loot/{id}",
            axum::routing::delete(delete_loot_item),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/sessions/{session_id}/loot",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("session_id" = Uuid, Path, description = "Session ID"),
    ),
    responses((status = 200, description = "List all loot for session", body = [LootItem]))
)]
async fn list_loot(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = LootRepository::new(&state.db);
    Ok(Json(repo.list_by_session(session_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/sessions/{session_id}/loot",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("session_id" = Uuid, Path, description = "Session ID"),
    ),
    request_body = CreateLootItemRequest,
    responses((status = 201, description = "Loot item created", body = LootItem))
)]
async fn create_loot_item(
    State(state): State<AppState>,
    Path((campaign_id, session_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateLootItemRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = LootRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(session_id, campaign_id, req).await?)))
}

async fn delete_loot_item(
    State(state): State<AppState>,
    Path((_campaign_id, _session_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = LootRepository::new(&state.db);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
