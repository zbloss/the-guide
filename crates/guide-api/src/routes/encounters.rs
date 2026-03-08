use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use guide_combat::{build_participant, initiative::roll_initiative, CombatEngine};
use guide_core::{
    models::{
        CombatParticipant, CreateEncounterRequest, Encounter, EncounterSummary,
        UpdateParticipantRequest,
    },
    GuideError,
};
use guide_db::{characters::CharacterRepository, encounters::EncounterRepository};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/encounters",
            get(list_encounters).post(create_encounter),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{id}",
            get(get_encounter).delete(delete_encounter),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{id}/start",
            post(start_encounter),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{id}/next-turn",
            post(next_turn),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{id}/end",
            post(end_encounter),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{id}/participants/{pid}",
            put(update_participant),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/encounters",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("session_id" = Uuid, Query, description = "Session ID (optional filtering)")
    ),
    responses(
        (status = 200, description = "List all encounters in a session/campaign", body = [Encounter])
    )
)]
async fn list_encounters(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // Note: the path uses campaign_id but encounters are per-session in the DB
    // We re-route by querying all encounters for sessions in this campaign.
    // For simplicity, we accept a session_id query param or use campaign_id as session_id.
    let repo = EncounterRepository::new(&state.db);
    Ok(Json(repo.list_by_session(session_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/encounters",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = CreateEncounterRequest,
    responses(
        (status = 201, description = "Encounter created successfully", body = Encounter)
    )
)]
async fn create_encounter(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateEncounterRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let enc_repo = EncounterRepository::new(&state.db);
    let char_repo = CharacterRepository::new(&state.db);

    let mut encounter = enc_repo.create(campaign_id, req.clone()).await?;

    // Add participants from requested character IDs
    for char_id in &req.participant_character_ids {
        let character = char_repo.get_by_id(*char_id).await?;
        let entry = roll_initiative(character.ability_scores.initiative_modifier());
        let participant = build_participant(
            character.id,
            encounter.id,
            &character.name,
            entry.roll,
            entry.modifier,
            character.max_hp,
            character.current_hp,
            character.armor_class,
            character.speed,
        );
        enc_repo.add_participant(&participant).await?;
        encounter.participants.push(participant);
    }

    Ok((StatusCode::CREATED, Json(encounter)))
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/encounters/{id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Encounter ID")
    ),
    responses(
        (status = 200, description = "Found encounter", body = Encounter),
        (status = 404, description = "Encounter not found")
    )
)]
async fn get_encounter(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = EncounterRepository::new(&state.db);
    Ok(Json(repo.get_by_id(id).await?))
}

#[utoipa::path(
    delete,
    path = "/campaigns/{campaign_id}/encounters/{id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Encounter ID")
    ),
    responses(
        (status = 204, description = "Encounter deleted successfully"),
        (status = 404, description = "Encounter not found")
    )
)]
async fn delete_encounter(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = EncounterRepository::new(&state.db);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/encounters/{id}/start",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Encounter ID")
    ),
    responses(
        (status = 200, description = "Encounter started", body = EncounterSummary),
        (status = 404, description = "Encounter not found")
    )
)]
async fn start_encounter(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = EncounterRepository::new(&state.db);
    let encounter = repo.get_by_id(id).await?;
    let mut engine = CombatEngine::new(encounter);
    engine.start()?;
    repo.save_state(&engine.encounter).await?;
    let round = engine.encounter.round;
    Ok(Json(serde_json::json!({
        "encounter": engine.encounter,
        "current_participant": engine.current_participant().cloned(),
        "round": round,
    })))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/encounters/{id}/next-turn",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Encounter ID")
    ),
    responses(
        (status = 200, description = "Turn advanced", body = EncounterSummary),
        (status = 404, description = "Encounter not found")
    )
)]
async fn next_turn(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = EncounterRepository::new(&state.db);
    let encounter = repo.get_by_id(id).await?;
    let mut engine = CombatEngine::new(encounter);
    engine.next_turn()?;
    repo.save_state(&engine.encounter).await?;
    let current = engine.current_participant().cloned();
    Ok(Json(serde_json::json!({
        "encounter": engine.encounter,
        "current_participant": current,
        "round": engine.encounter.round,
    })))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/encounters/{id}/end",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Encounter ID")
    ),
    responses(
        (status = 200, description = "Encounter ended", body = Encounter),
        (status = 404, description = "Encounter not found")
    )
)]
async fn end_encounter(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = EncounterRepository::new(&state.db);
    let encounter = repo.get_by_id(id).await?;
    let mut engine = CombatEngine::new(encounter);
    engine.end()?;
    repo.save_state(&engine.encounter).await?;
    Ok(Json(engine.encounter))
}

#[utoipa::path(
    put,
    path = "/campaigns/{campaign_id}/encounters/{id}/participants/{pid}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Encounter ID"),
        ("pid" = Uuid, Path, description = "Participant ID")
    ),
    request_body = UpdateParticipantRequest,
    responses(
        (status = 200, description = "Participant updated", body = CombatParticipant),
        (status = 404, description = "Encounter or participant not found")
    )
)]
async fn update_participant(
    State(state): State<AppState>,
    Path((_campaign_id, enc_id, pid)): Path<(Uuid, Uuid, Uuid)>,
    Json(req): Json<UpdateParticipantRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = EncounterRepository::new(&state.db);
    let encounter = repo.get_by_id(enc_id).await?;
    let mut engine = CombatEngine::new(encounter);

    if let Some(delta) = req.hp_delta {
        engine.apply_hp_change(pid, delta)?;
    }
    if let Some(hp) = req.set_hp {
        engine.set_hp(pid, hp)?;
    }
    if let Some(condition) = req.add_condition {
        engine.add_condition(pid, condition)?;
    }
    if let Some(condition) = &req.remove_condition {
        engine.remove_condition(pid, condition)?;
    }

    repo.save_state(&engine.encounter).await?;

    let participant = engine
        .encounter
        .participants
        .iter()
        .find(|p| p.id == pid)
        .cloned()
        .ok_or_else(|| GuideError::NotFound(format!("Participant {pid}")))?;

    Ok(Json(participant))
}
