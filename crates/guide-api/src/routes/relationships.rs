use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Json, Router,
};
use guide_core::models::CreateRelationshipRequest;
use guide_db::relationships::RelationshipRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/relationships",
            get(list_relationships).post(create_relationship),
        )
        .route(
            "/campaigns/{campaign_id}/relationships/{rel_id}",
            delete(delete_relationship),
        )
}

async fn list_relationships(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = RelationshipRepository::new(&state.db);
    Ok(Json(repo.list_by_campaign(campaign_id).await?))
}

async fn create_relationship(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateRelationshipRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = RelationshipRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(campaign_id, req).await?)))
}

async fn delete_relationship(
    State(state): State<AppState>,
    Path((_campaign_id, rel_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = RelationshipRepository::new(&state.db);
    repo.delete(rel_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
