use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use guide_core::{models::GeneratedEncounter, GuideError};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/campaigns/{campaign_id}/encounters/generate",
        post(generate_encounter),
    )
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GenerateRequest {
    pub context: Option<String>,
    pub party_level: Option<u32>,
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/encounters/generate",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = GenerateRequest,
    responses(
        (status = 200, description = "Encounter generated successfully", body = GeneratedEncounter)
    )
)]
async fn generate_encounter(
    State(state): State<AppState>,
    Path(_campaign_id): Path<Uuid>,
    Json(req): Json<GenerateRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let context = req.context.unwrap_or_else(|| "A standard encounter".to_string());
    let party_level = req.party_level.unwrap_or(1);

    let system_prompt = "You are a D&D 5e encounter designer. \
        Respond with ONLY a valid JSON object — no markdown, no code fences, no explanation, no preamble. \
        The JSON must exactly match the schema provided by the user.";

    let user_prompt = format!(
        "/no_think\n\
         Generate a D&D 5e encounter for a party of level {party_level}.\n\
         Context: {context}\n\n\
         Return ONLY valid JSON matching this schema:\n\
         {{\n\
           \"title\": \"<encounter name>\",\n\
           \"description\": \"<vivid scene description>\",\n\
           \"encounter_type\": \"combat|social|exploration|puzzle|mixed\",\n\
           \"challenge_rating\": <number or null>,\n\
           \"suggested_enemies\": [{{\n\
             \"name\": \"<enemy name>\",\n\
             \"count\": <number>,\n\
             \"cr\": <number or null>\n\
           }}],\n\
           \"narrative_hook\": \"<why this encounter matters to the story>\",\n\
           \"alternative\": \"<optional alternative approach or null>\"\n\
         }}"
    );

    let llm_req = CompletionRequest {
        task: LlmTask::EncounterGeneration,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: system_prompt.to_string(),
            },
            Message {
                role: MessageRole::User,
                content: user_prompt,
            },
        ],
        model_override: None,
        temperature: Some(0.8),
        max_tokens: Some(4096),
    };

    let resp = state.llm.complete(llm_req).await?;

    // Strip markdown code fences that some models add despite instructions
    let raw = resp.content.trim();
    let json_str = raw
        .strip_prefix("```json")
        .or_else(|| raw.strip_prefix("```"))
        .map(|s| s.trim_end_matches("```").trim())
        .unwrap_or(raw);

    if json_str.is_empty() {
        return Err(GuideError::Llm(
            "LLM returned empty response for encounter generation".into(),
        )
        .into());
    }

    let encounter: GeneratedEncounter = serde_json::from_str(json_str)
        .map_err(|e| {
            tracing::warn!("Encounter parse failed. Raw content: {:?}", resp.content);
            GuideError::Llm(format!("Failed to parse generated encounter: {e}"))
        })?;

    Ok(Json(encounter))
}
