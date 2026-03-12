use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::models::{CalendarEntry, CreateCalendarEntryRequest};
use guide_db::calendar::CalendarRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/calendar",
            get(list_entries).post(create_entry),
        )
        .route(
            "/campaigns/{campaign_id}/calendar/{id}",
            axum::routing::delete(delete_entry),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/calendar",
    params(("campaign_id" = Uuid, Path, description = "Campaign ID")),
    responses((status = 200, description = "List all calendar entries", body = [CalendarEntry]))
)]
async fn list_entries(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CalendarRepository::new(&state.db);
    Ok(Json(repo.list_by_campaign(campaign_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/calendar",
    params(("campaign_id" = Uuid, Path, description = "Campaign ID")),
    request_body = CreateCalendarEntryRequest,
    responses((status = 201, description = "Calendar entry created", body = CalendarEntry))
)]
async fn create_entry(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateCalendarEntryRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CalendarRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(campaign_id, req).await?)))
}

async fn delete_entry(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CalendarRepository::new(&state.db);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
