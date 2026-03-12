use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use guide_core::models::{EncounterTemplate, SaveTemplateRequest, TemplateParticipant};
use guide_db::{encounters::EncounterRepository, templates::TemplateRepository};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/encounters/{enc_id}/save-as-template",
            post(save_as_template),
        )
        .route("/encounter-templates", get(list_templates))
        .route(
            "/encounter-templates/{id}",
            get(get_template).delete(delete_template),
        )
}

async fn save_as_template(
    State(state): State<AppState>,
    Path((_campaign_id, enc_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SaveTemplateRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let enc_repo = EncounterRepository::new(&state.db);
    let encounter = enc_repo.get_by_id(enc_id).await?;

    let participants: Vec<TemplateParticipant> = encounter
        .participants
        .iter()
        .map(|p| TemplateParticipant {
            name: p.name.clone(),
            max_hp: p.max_hp,
            armor_class: p.armor_class,
            speed: p.action_budget.movement_remaining,
        })
        .collect();

    let template = EncounterTemplate {
        id: Uuid::new_v4(),
        name: req
            .name
            .unwrap_or_else(|| encounter.name.clone().unwrap_or_else(|| "Unnamed Encounter".to_string())),
        description: encounter.description.clone(),
        participants,
        created_at: Utc::now(),
    };

    let tmpl_repo = TemplateRepository::new(&state.db);
    tmpl_repo.save_template(&template).await?;

    Ok((StatusCode::CREATED, Json(template)))
}

async fn list_templates(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = TemplateRepository::new(&state.db);
    Ok(Json(repo.list_templates().await?))
}

async fn get_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = TemplateRepository::new(&state.db);
    Ok(Json(repo.get_template(id).await?))
}

async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = TemplateRepository::new(&state.db);
    repo.delete_template(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
