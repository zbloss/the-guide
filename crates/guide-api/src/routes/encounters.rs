use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use guide_combat::{build_participant, CombatEngine};
use guide_core::models::{CreateEncounterRequest, EncounterSummary, UpdateParticipantRequest};
use guide_db::{
    characters::CharacterRepository,
    encounters::EncounterRepository,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Encounter lifecycle
        .route(
            "/campaigns/{campaign_id}/sessions/{session_id}/encounters",
            get(list_encounters).post(create_encounter),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{enc_id}",
            get(get_encounter),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{enc_id}/start",
            post(start_encounter),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{enc_id}/next-turn",
            post(next_turn),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{enc_id}/participants/{char_id}",
            put(update_participant),
        )
        .route(
            "/campaigns/{campaign_id}/encounters/{enc_id}/end",
            post(end_encounter),
        )
}

async fn list_encounters(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = EncounterRepository::new(&state.db);
    match repo.list_by_session(session_id).await {
        Ok(encounters) => (StatusCode::OK, Json(encounters)).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn create_encounter(
    State(state): State<AppState>,
    Path((campaign_id, _session_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateEncounterRequest>,
) -> impl IntoResponse {
    let enc_repo = EncounterRepository::new(&state.db);
    let char_repo = CharacterRepository::new(&state.db);

    // Collect participant character IDs from the request
    let char_ids = req.participant_character_ids.clone();

    // Validate all requested characters exist before creating the encounter
    let mut characters = Vec::new();
    let mut missing_ids = Vec::new();
    for char_id in &char_ids {
        match char_repo.get_by_id(*char_id).await {
            Ok(c) => characters.push(c),
            Err(_) => missing_ids.push(char_id.to_string()),
        }
    }
    if !missing_ids.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("Unknown character IDs: {}", missing_ids.join(", ")),
        );
    }

    let encounter = match enc_repo.create(campaign_id, req).await {
        Ok(e) => e,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    // Add each validated character as a participant with rolled initiative
    for character in characters {

        let dex_mod = guide_core::models::AbilityScores::modifier(
            character.ability_scores.dexterity,
        );
        let roll = guide_combat::initiative::roll_d20();

        let participant = build_participant(
            character.id,
            encounter.id,
            &character.name,
            roll,
            dex_mod,
            character.max_hp,
            character.current_hp,
            character.armor_class,
            character.speed,
        );

        if let Err(e) = enc_repo.add_participant(&participant).await {
            tracing::warn!("Failed to add participant {}: {e}", character.id);
        }
    }

    // Return fresh encounter with participants
    match enc_repo.get_by_id(encounter.id).await {
        Ok(enc) => (StatusCode::CREATED, Json(enc)).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn get_encounter(
    State(state): State<AppState>,
    Path((_campaign_id, enc_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = EncounterRepository::new(&state.db);
    match repo.get_by_id(enc_id).await {
        Ok(enc) => (StatusCode::OK, Json(enc)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn start_encounter(
    State(state): State<AppState>,
    Path((_campaign_id, enc_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = EncounterRepository::new(&state.db);

    let encounter = match repo.get_by_id(enc_id).await {
        Ok(e) => e,
        Err(guide_core::GuideError::NotFound(msg)) => return error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let mut engine = CombatEngine::new(encounter);

    if let Err(e) = engine.start() {
        return error_response(StatusCode::CONFLICT, &e.to_string());
    }

    if let Err(e) = repo.save_state(&engine.encounter).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }

    let current = engine.current_participant().cloned();
    (
        StatusCode::OK,
        Json(EncounterSummary {
            round: engine.encounter.round,
            current_participant: current,
            encounter: engine.encounter,
        }),
    )
        .into_response()
}

async fn next_turn(
    State(state): State<AppState>,
    Path((_campaign_id, enc_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = EncounterRepository::new(&state.db);

    let encounter = match repo.get_by_id(enc_id).await {
        Ok(e) => e,
        Err(guide_core::GuideError::NotFound(msg)) => return error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let mut engine = CombatEngine::new(encounter);

    let next_participant = match engine.next_turn() {
        Ok(p) => p.clone(),
        Err(e) => return error_response(StatusCode::CONFLICT, &e.to_string()),
    };

    if let Err(e) = repo.save_state(&engine.encounter).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }

    (
        StatusCode::OK,
        Json(EncounterSummary {
            round: engine.encounter.round,
            current_participant: Some(next_participant),
            encounter: engine.encounter,
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct UpdateParticipantPath {
    campaign_id: Uuid,
    enc_id: Uuid,
    char_id: Uuid,
}

async fn update_participant(
    State(state): State<AppState>,
    Path(path): Path<UpdateParticipantPath>,
    Json(req): Json<UpdateParticipantRequest>,
) -> impl IntoResponse {
    let repo = EncounterRepository::new(&state.db);

    let encounter = match repo.get_by_id(path.enc_id).await {
        Ok(e) => e,
        Err(guide_core::GuideError::NotFound(msg)) => return error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let mut engine = CombatEngine::new(encounter);

    // Find participant by character_id (the path param is a character id)
    let participant_id = match engine
        .encounter
        .participants
        .iter()
        .find(|p| p.character_id == path.char_id)
        .map(|p| p.id)
    {
        Some(id) => id,
        None => return error_response(StatusCode::NOT_FOUND, "Participant not found in encounter"),
    };

    // Apply HP changes
    if let Some(delta) = req.hp_delta {
        if let Err(e) = engine.apply_hp_change(participant_id, delta) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    }
    if let Some(hp) = req.set_hp {
        if let Err(e) = engine.set_hp(participant_id, hp) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    }

    // Apply condition changes
    if let Some(cond) = req.add_condition {
        if let Err(e) = engine.add_condition(participant_id, cond) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    }
    if let Some(cond) = req.remove_condition {
        if let Err(e) = engine.remove_condition(participant_id, &cond) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    }

    // Apply action budget changes
    if let Some(participant) = engine.encounter.participants.iter_mut().find(|p| p.id == participant_id) {
        if req.spend_action.unwrap_or(false) {
            participant.action_budget.has_action = false;
        }
        if req.spend_bonus_action.unwrap_or(false) {
            participant.action_budget.has_bonus_action = false;
        }
        if req.spend_reaction.unwrap_or(false) {
            participant.action_budget.has_reaction = false;
        }
        if let Some(mv) = req.spend_movement {
            participant.action_budget.movement_remaining =
                (participant.action_budget.movement_remaining - mv).max(0);
        }
    }

    if let Err(e) = repo.save_state(&engine.encounter).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }

    let updated = engine
        .encounter
        .participants
        .iter()
        .find(|p| p.id == participant_id)
        .cloned();

    (StatusCode::OK, Json(updated)).into_response()
}

async fn end_encounter(
    State(state): State<AppState>,
    Path((_campaign_id, enc_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = EncounterRepository::new(&state.db);

    let encounter = match repo.get_by_id(enc_id).await {
        Ok(e) => e,
        Err(guide_core::GuideError::NotFound(msg)) => return error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let mut engine = CombatEngine::new(encounter);

    if let Err(e) = engine.end() {
        return error_response(StatusCode::CONFLICT, &e.to_string());
    }

    if let Err(e) = repo.save_state(&engine.encounter).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }

    (StatusCode::OK, Json(engine.encounter)).into_response()
}

fn error_response(status: StatusCode, msg: &str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": msg }))).into_response()
}
