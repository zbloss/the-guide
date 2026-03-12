use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use guide_core::models::{Campaign, CreateCampaignRequest, UpdateCampaignRequest};
use guide_db::campaigns::CampaignRepository;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/campaigns", get(list_campaigns).post(create_campaign))
        .route(
            "/campaigns/{id}",
            get(get_campaign).put(update_campaign).delete(delete_campaign),
        )
        .route(
            "/campaigns/{id}/plot-twist",
            post(generate_plot_twist),
        )
        .route("/campaigns/{id}/analytics", get(get_analytics))
        .route("/campaigns/{id}/share", post(generate_share_token))
        .route("/view/{token}", get(get_shared_view))
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

#[derive(Deserialize)]
struct PlotTwistRequest {
    tone: Option<String>,
}

async fn generate_plot_twist(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Option<Json<PlotTwistRequest>>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let repo = CampaignRepository::new(&state.db);
    let campaign = repo.get_by_id(id).await?;

    let tone = body
        .as_ref()
        .and_then(|b| b.tone.clone())
        .unwrap_or_else(|| "dramatic".to_string());

    let system_prompt = concat!(
        "You are a master D&D storyteller. Generate a single surprising plot twist for this campaign. ",
        "The twist should be unexpected but feel earned, involving existing elements. ",
        "Output: one paragraph description of the twist, written as a dramatic reveal for the DM."
    );

    let description = campaign.description.unwrap_or_default();
    let user_message = format!(
        "Campaign: {}\nDescription: {}\nTone: {}",
        campaign.name, description, tone
    );

    let req = CompletionRequest {
        task: LlmTask::General,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: system_prompt.to_string(),
            },
            Message {
                role: MessageRole::User,
                content: user_message,
            },
        ],
        model_override: None,
        temperature: Some(0.9),
        max_tokens: Some(512),
    };

    let resp = state.llm.complete(req).await?;

    Ok(Json(serde_json::json!({
        "twist": resp.content.trim(),
        "tone": tone,
    })))
}

async fn get_analytics(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignRepository::new(&state.db);
    let data = repo.analytics(campaign_id).await?;
    Ok(Json(data))
}

async fn generate_share_token(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CampaignRepository::new(&state.db);
    let campaign = repo.generate_share_token(campaign_id).await?;
    Ok(Json(campaign))
}

async fn get_shared_view(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_db::{characters::CharacterRepository, sessions::SessionRepository};
    let repo = CampaignRepository::new(&state.db);
    let campaign = repo.get_by_share_token(&token).await?;

    let char_repo = CharacterRepository::new(&state.db);
    let characters = char_repo.list_by_campaign(campaign.id).await.unwrap_or_default();
    let pc_characters: Vec<_> = characters
        .into_iter()
        .filter(|c| matches!(c.character_type, guide_core::models::CharacterType::Pc))
        .map(|c| serde_json::json!({
            "name": c.name,
            "class": c.class,
            "race": c.race,
            "level": c.level,
            "portrait_url": c.portrait_url,
        }))
        .collect();

    let session_repo = SessionRepository::new(&state.db);
    let sessions = session_repo.list_by_campaign(campaign.id).await.unwrap_or_default();
    let last_session = sessions.first().map(|s| serde_json::json!({
        "session_number": s.session_number,
        "title": s.title,
        "created_at": s.created_at,
    }));

    Ok(Json(serde_json::json!({
        "campaign": {
            "id": campaign.id,
            "name": campaign.name,
            "description": campaign.description,
            "game_system": campaign.game_system,
            "world_state": campaign.world_state,
        },
        "party": pc_characters,
        "last_session": last_session,
        "session_count": sessions.len(),
    })))
}
