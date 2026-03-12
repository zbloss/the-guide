use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::models::{
    CreateTrackedPlotHookRequest, TrackedPlotHook, UpdateTrackedPlotHookRequest,
};
use guide_db::PlotHookRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/characters/{char_id}/plot-hooks",
            get(list_plot_hooks).post(create_plot_hook),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{char_id}/plot-hooks/{hook_id}",
            axum::routing::put(update_plot_hook).delete(delete_plot_hook),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/characters/{char_id}/plot-hooks",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("char_id" = Uuid, Path, description = "Character ID")
    ),
    responses(
        (status = 200, description = "List all plot hooks for a character", body = [TrackedPlotHook])
    )
)]
pub async fn list_plot_hooks(
    State(state): State<AppState>,
    Path((_campaign_id, char_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = PlotHookRepository::new(&state.db);
    Ok(Json(repo.list_by_character(char_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/characters/{char_id}/plot-hooks",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("char_id" = Uuid, Path, description = "Character ID")
    ),
    request_body = CreateTrackedPlotHookRequest,
    responses(
        (status = 201, description = "Plot hook created", body = TrackedPlotHook)
    )
)]
pub async fn create_plot_hook(
    State(state): State<AppState>,
    Path((_campaign_id, char_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateTrackedPlotHookRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = PlotHookRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(char_id, &req).await?)))
}

#[utoipa::path(
    put,
    path = "/campaigns/{campaign_id}/characters/{char_id}/plot-hooks/{hook_id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("char_id" = Uuid, Path, description = "Character ID"),
        ("hook_id" = Uuid, Path, description = "Plot Hook ID")
    ),
    request_body = UpdateTrackedPlotHookRequest,
    responses(
        (status = 200, description = "Plot hook updated", body = TrackedPlotHook),
        (status = 404, description = "Plot hook not found")
    )
)]
pub async fn update_plot_hook(
    State(state): State<AppState>,
    Path((_campaign_id, _char_id, hook_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(req): Json<UpdateTrackedPlotHookRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = PlotHookRepository::new(&state.db);
    Ok(Json(repo.update(hook_id, &req).await?))
}

#[utoipa::path(
    delete,
    path = "/campaigns/{campaign_id}/characters/{char_id}/plot-hooks/{hook_id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("char_id" = Uuid, Path, description = "Character ID"),
        ("hook_id" = Uuid, Path, description = "Plot Hook ID")
    ),
    responses(
        (status = 204, description = "Plot hook deleted"),
        (status = 404, description = "Plot hook not found")
    )
)]
pub async fn delete_plot_hook(
    State(state): State<AppState>,
    Path((_campaign_id, _char_id, hook_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = PlotHookRepository::new(&state.db);
    repo.delete(hook_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
