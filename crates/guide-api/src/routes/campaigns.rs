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

    if let Some(q) = state.qdrant.as_deref() {
        let col = guide_db::qdrant::campaign_collection_name(&id.to_string());
        if let Err(e) = guide_db::qdrant::delete_collection(q, &col).await {
            tracing::warn!("Qdrant collection deletion failed for campaign {id}: {e}");
        }
    }

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
        json_mode: false,
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

