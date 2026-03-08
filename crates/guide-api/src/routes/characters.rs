use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use guide_core::{
    models::{Character, CreateCharacterRequest, UpdateCharacterRequest},
    GuideError,
};
use guide_db::characters::CharacterRepository;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/characters",
            get(list_characters).post(create_character),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{id}",
            get(get_character).put(update_character).delete(delete_character),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{id}/analyze-backstory",
            post(analyze_backstory),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/characters",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    responses(
        (status = 200, description = "List all characters in a campaign", body = [Character])
    )
)]
async fn list_characters(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CharacterRepository::new(&state.db);
    let characters = repo.list_by_campaign(campaign_id).await?;
    Ok(Json(characters))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/characters",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = CreateCharacterRequest,
    responses(
        (status = 201, description = "Character created successfully", body = Character)
    )
)]
async fn create_character(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateCharacterRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CharacterRepository::new(&state.db);
    let character = repo.create(campaign_id, req).await?;
    Ok((StatusCode::CREATED, Json(character)))
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/characters/{id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Character ID")
    ),
    responses(
        (status = 200, description = "Found character", body = Character),
        (status = 404, description = "Character not found")
    )
)]
async fn get_character(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CharacterRepository::new(&state.db);
    let character = repo.get_by_id(id).await?;
    Ok(Json(character))
}

#[utoipa::path(
    put,
    path = "/campaigns/{campaign_id}/characters/{id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Character ID")
    ),
    request_body = UpdateCharacterRequest,
    responses(
        (status = 200, description = "Character updated successfully", body = Character),
        (status = 404, description = "Character not found")
    )
)]
async fn update_character(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateCharacterRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CharacterRepository::new(&state.db);
    let character = repo.update(id, req).await?;
    Ok(Json(character))
}

#[utoipa::path(
    delete,
    path = "/campaigns/{campaign_id}/characters/{id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Character ID")
    ),
    responses(
        (status = 204, description = "Character deleted successfully"),
        (status = 404, description = "Character not found")
    )
)]
async fn delete_character(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CharacterRepository::new(&state.db);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/characters/{id}/analyze-backstory",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Character ID")
    ),
    responses(
        (status = 200, description = "Backstory analyzed and updated", body = Character),
        (status = 404, description = "Character not found")
    )
)]
async fn analyze_backstory(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_core::models::{Backstory, PlotHook, HookPriority};
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole, prompts};
    use serde::Deserialize;

    let repo = CharacterRepository::new(&state.db);
    let character = repo.get_by_id(id).await?;

    let backstory_text = character
        .backstory
        .as_ref()
        .map(|b| b.raw_text.clone())
        .ok_or_else(|| GuideError::InvalidInput("Character has no backstory text".into()))?;

    #[derive(Deserialize)]
    struct LlmHook {
        description: String,
        priority: String,
    }
    #[derive(Deserialize)]
    struct LlmBackstory {
        motivations: Vec<String>,
        key_relationships: Vec<String>,
        secrets: Vec<String>,
        plot_hooks: Vec<LlmHook>,
    }

    let req = CompletionRequest {
        task: LlmTask::BackstoryAnalysis,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: prompts::backstory_analysis_system().to_string(),
            },
            Message {
                role: MessageRole::User,
                content: backstory_text,
            },
        ],
        model_override: None,
        temperature: Some(0.7),
        max_tokens: Some(1024),
    };

    let resp = state.llm.complete(req).await?;
    let parsed: LlmBackstory = serde_json::from_str(resp.content.trim())
        .map_err(|e| GuideError::Llm(format!("Failed to parse backstory JSON: {e}")))?;

    let hooks: Vec<PlotHook> = parsed
        .plot_hooks
        .into_iter()
        .map(|h| PlotHook {
            id: uuid::Uuid::new_v4(),
            character_id: id,
            description: h.description,
            priority: match h.priority.as_str() {
                "critical" => HookPriority::Critical,
                "high" => HookPriority::High,
                "medium" => HookPriority::Medium,
                _ => HookPriority::Low,
            },
            is_active: true,
            llm_extracted: true,
        })
        .collect();

    let backstory = Backstory {
        raw_text: character
            .backstory
            .map(|b| b.raw_text)
            .unwrap_or_default(),
        extracted_hooks: hooks,
        motivations: parsed.motivations,
        key_relationships: parsed.key_relationships,
        secrets: parsed.secrets,
    };

    let updated = repo.update_backstory(id, &backstory).await?;
    Ok(Json(updated))
}
