use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use guide_core::models::{ArcStatus, CreateEncounterRequest, SubplotStatus};
use guide_db::{encounters::EncounterRepository, story::StoryRepository};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/campaigns/{id}/story/arcs", get(list_arcs))
        .route(
            "/campaigns/{id}/story/arcs/{arc_id}/notes",
            patch(update_arc_notes),
        )
        .route(
            "/campaigns/{id}/story/arcs/{arc_id}/status",
            patch(update_arc_status),
        )
        .route("/campaigns/{id}/story/events", get(list_events))
        .route(
            "/campaigns/{id}/story/events/{eid}/notes",
            patch(update_event_notes),
        )
        .route("/campaigns/{id}/story/subplots", get(list_subplots))
        .route(
            "/campaigns/{id}/story/subplots/{sid}/notes",
            patch(update_subplot_notes),
        )
        .route(
            "/campaigns/{id}/story/subplots/{sid}/status",
            patch(update_subplot_status),
        )
        .route(
            "/campaigns/{id}/story/character-arcs",
            get(list_character_arcs),
        )
        .route(
            "/campaigns/{id}/story/character-arcs/{caid}/notes",
            patch(update_character_arc_notes),
        )
        .route(
            "/campaigns/{id}/story/encounters",
            get(list_story_encounters),
        )
        .route(
            "/campaigns/{id}/story/encounters/{eid}/notes",
            patch(update_story_encounter_notes),
        )
        .route(
            "/campaigns/{id}/story/encounters/{eid}/activate",
            post(activate_encounter),
        )
        .route("/campaigns/{id}/story/npcs", get(list_npcs))
        .route("/campaigns/{id}/story/locations", get(list_locations))
        .route("/campaigns/{id}/story/factions", get(list_factions))
}

#[derive(Deserialize)]
struct ArcIdFilter {
    arc_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct NotesBody {
    dm_notes: Option<String>,
}

#[derive(Deserialize)]
struct ArcStatusBody {
    status: ArcStatus,
}

#[derive(Deserialize)]
struct SubplotStatusBody {
    status: SubplotStatus,
}

async fn list_arcs(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    Ok(Json(repo.list_arcs(campaign_id).await?))
}

async fn update_arc_notes(
    State(state): State<AppState>,
    Path((_campaign_id, arc_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<NotesBody>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    repo.update_arc_notes(arc_id, body.dm_notes.as_deref())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_arc_status(
    State(state): State<AppState>,
    Path((_campaign_id, arc_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ArcStatusBody>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    repo.update_arc_status(arc_id, body.status).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_events(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Query(filter): Query<ArcIdFilter>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    let events = if let Some(arc_id) = filter.arc_id {
        repo.list_events_by_arc(arc_id).await?
    } else {
        repo.list_events(campaign_id).await?
    };
    Ok(Json(events))
}

async fn update_event_notes(
    State(state): State<AppState>,
    Path((_campaign_id, event_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<NotesBody>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    repo.update_event_notes(event_id, body.dm_notes.as_deref())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_subplots(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    Ok(Json(repo.list_subplots(campaign_id).await?))
}

async fn update_subplot_notes(
    State(state): State<AppState>,
    Path((_campaign_id, subplot_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<NotesBody>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    repo.update_subplot_notes(subplot_id, body.dm_notes.as_deref())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_subplot_status(
    State(state): State<AppState>,
    Path((_campaign_id, subplot_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<SubplotStatusBody>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    repo.update_subplot_status(subplot_id, body.status).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_character_arcs(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    Ok(Json(repo.list_character_arcs(campaign_id).await?))
}

async fn update_character_arc_notes(
    State(state): State<AppState>,
    Path((_campaign_id, arc_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<NotesBody>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    repo.update_character_arc_notes(arc_id, body.dm_notes.as_deref())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_story_encounters(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    Ok(Json(repo.list_prepopulated_encounters(campaign_id).await?))
}

async fn update_story_encounter_notes(
    State(state): State<AppState>,
    Path((_campaign_id, encounter_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<NotesBody>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    repo.update_encounter_notes(encounter_id, body.dm_notes.as_deref())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct ActivateResponse {
    encounter_id: Uuid,
}

async fn activate_encounter(
    State(state): State<AppState>,
    Path((campaign_id, prepop_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let story_repo = StoryRepository::new(&state.db);
    let prepop = story_repo.get_prepopulated_encounter(prepop_id).await?;

    let enc_repo = EncounterRepository::new(&state.db);
    let encounter = enc_repo
        .create(
            campaign_id,
            CreateEncounterRequest {
                session_id: None,
                name: Some(prepop.name),
                description: Some(prepop.description),
                participant_character_ids: vec![],
            },
        )
        .await?;

    Ok(Json(ActivateResponse {
        encounter_id: encounter.id,
    }))
}

async fn list_npcs(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    Ok(Json(repo.list_npcs(campaign_id).await?))
}

async fn list_locations(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    Ok(Json(repo.list_locations(campaign_id).await?))
}

async fn list_factions(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = StoryRepository::new(&state.db);
    Ok(Json(repo.list_factions(campaign_id).await?))
}
