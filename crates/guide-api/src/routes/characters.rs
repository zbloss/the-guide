use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use guide_core::models::{Backstory, CreateCharacterRequest, PlotHook};
use guide_db::characters::CharacterRepository;
use guide_llm::{
    client::{CompletionRequest, LlmTask, Message, MessageRole},
    prompts,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/characters",
            get(list_characters).post(create_character),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{char_id}",
            get(get_character).delete(delete_character),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{char_id}/analyze-backstory",
            post(analyze_backstory),
        )
}

async fn list_characters(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = CharacterRepository::new(&state.db);
    match repo.list_by_campaign(campaign_id).await {
        Ok(chars) => (StatusCode::OK, Json(chars)).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn create_character(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateCharacterRequest>,
) -> impl IntoResponse {
    let repo = CharacterRepository::new(&state.db);
    match repo.create(campaign_id, req).await {
        Ok(character) => (StatusCode::CREATED, Json(character)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn get_character(
    State(state): State<AppState>,
    Path((_campaign_id, char_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = CharacterRepository::new(&state.db);
    match repo.get_by_id(char_id).await {
        Ok(character) => (StatusCode::OK, Json(character)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn delete_character(
    State(state): State<AppState>,
    Path((_campaign_id, char_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = CharacterRepository::new(&state.db);
    match repo.delete(char_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// ── Backstory Analysis ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AnalyzeBackstoryRequest {
    /// Raw backstory text. If omitted, uses existing character.backstory.raw_text.
    backstory_text: Option<String>,
}

#[derive(Debug, Serialize)]
struct AnalyzeBackstoryResponse {
    character_id: Uuid,
    backstory: Backstory,
}

/// LLM-powered endpoint: extract plot hooks, motivations, secrets from backstory text.
async fn analyze_backstory(
    State(state): State<AppState>,
    Path((_campaign_id, char_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AnalyzeBackstoryRequest>,
) -> impl IntoResponse {
    let repo = CharacterRepository::new(&state.db);

    let character = match repo.get_by_id(char_id).await {
        Ok(c) => c,
        Err(guide_core::GuideError::NotFound(msg)) => return error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    // Resolve the backstory text to analyse
    let raw_text = match req.backstory_text.or_else(|| {
        character.backstory.as_ref().map(|b| b.raw_text.clone())
    }) {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "No backstory text provided and character has no existing backstory",
            )
        }
    };

    // Ask the LLM to extract structured data
    let completion = state
        .llm
        .complete(CompletionRequest {
            task: LlmTask::BackstoryAnalysis,
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: prompts::backstory_analysis_system().to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: format!(
                        "Character: {}\n\nBackstory:\n{}",
                        character.name, raw_text
                    ),
                },
            ],
            model_override: None,
            temperature: Some(0.3), // low temp for structured extraction
            max_tokens: Some(1024),
        })
        .await;

    let llm_text = match completion {
        Ok(r) => r.content,
        Err(e) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("LLM unavailable: {e}"),
            )
        }
    };

    // Parse the JSON the LLM returned
    let extracted: ExtractedBackstory = match serde_json::from_str(llm_text.trim()) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("LLM returned non-JSON backstory extraction: {e}\n{llm_text}");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("LLM response was not valid JSON: {e}"),
            );
        }
    };

    // Build the Backstory struct
    let backstory = Backstory {
        raw_text: raw_text.clone(),
        extracted_hooks: extracted
            .plot_hooks
            .into_iter()
            .map(|h| PlotHook {
                id: Uuid::new_v4(),
                character_id: char_id,
                description: h.description,
                priority: parse_priority(&h.priority),
                is_active: true,
                llm_extracted: true,
            })
            .collect(),
        motivations: extracted.motivations,
        key_relationships: extracted.key_relationships,
        secrets: extracted.secrets,
    };

    // Persist
    if let Err(e) = repo.set_backstory(char_id, &backstory).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }

    (
        StatusCode::OK,
        Json(AnalyzeBackstoryResponse {
            character_id: char_id,
            backstory,
        }),
    )
        .into_response()
}

// ── LLM response deserialization ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ExtractedBackstory {
    #[serde(default)]
    motivations: Vec<String>,
    #[serde(default)]
    key_relationships: Vec<String>,
    #[serde(default)]
    secrets: Vec<String>,
    #[serde(default)]
    plot_hooks: Vec<ExtractedHook>,
}

#[derive(Debug, Deserialize)]
struct ExtractedHook {
    description: String,
    #[serde(default = "default_priority")]
    priority: String,
}

fn default_priority() -> String {
    "medium".to_string()
}

fn parse_priority(s: &str) -> guide_core::models::HookPriority {
    use guide_core::models::HookPriority;
    match s.to_lowercase().as_str() {
        "low" => HookPriority::Low,
        "high" => HookPriority::High,
        "critical" => HookPriority::Critical,
        _ => HookPriority::Medium,
    }
}

fn error_response(status: StatusCode, msg: &str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": msg }))).into_response()
}
