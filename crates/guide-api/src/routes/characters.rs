use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use guide_core::{
    models::{
        Backstory, Character, CharacterType, CreateCharacterRequest, GenerateNpcRequest,
        HookPriority, PlotHook, RestoreSlotRequest, SpendSlotRequest, UpdateCharacterRequest,
    },
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
        .route(
            "/campaigns/{campaign_id}/characters/{id}/villain-profile",
            post(generate_villain_profile),
        )
        .route(
            "/campaigns/{campaign_id}/npcs/generate",
            post(generate_npc),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{id}/spell-slots/spend",
            post(spend_spell_slot),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{id}/spell-slots/restore",
            post(restore_spell_slots),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{id}/portrait",
            post(upload_portrait),
        )
        .route(
            "/campaigns/{campaign_id}/characters/{id}/level-up",
            post(level_up_assist),
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
    let raw = resp.content.trim();
    if raw.is_empty() {
        return Err(GuideError::Llm(
            "LLM returned an empty response. Ensure the model is running and configured correctly.".into()
        ).into());
    }
    let json_str = raw
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let parsed: LlmBackstory = serde_json::from_str(json_str)
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

async fn generate_villain_profile(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let repo = CharacterRepository::new(&state.db);
    let character = repo.get_by_id(id).await?;

    let backstory_text = character
        .backstory
        .as_ref()
        .map(|b| b.raw_text.as_str())
        .unwrap_or("No backstory provided.");

    let user_message = format!(
        "Character: {name}\nRace: {race}\nClass: {class}\nLevel: {level}\nBackstory: {backstory}",
        name = character.name,
        race = character.race.as_deref().unwrap_or("Unknown"),
        class = character.class.as_deref().unwrap_or("Unknown"),
        level = character.level,
        backstory = backstory_text,
    );

    let req = CompletionRequest {
        task: LlmTask::General,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: "You are a D&D storytelling expert. Given a character, generate a compelling villain profile with these sections: **Motivation** (what drives them), **Core Flaw** (their fatal weakness), **Lair** (where they operate and its atmosphere), **Signature Move** (their most feared combat/social tactic), **Secret** (something that could humanize or redeem them). Be creative, specific, and dark. Output as plain text with markdown headers.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: user_message,
            },
        ],
        model_override: None,
        temperature: Some(0.8),
        max_tokens: Some(1024),
    };

    let resp = state.llm.complete(req).await?;
    let content = resp.content.trim().to_string();
    if content.is_empty() {
        return Err(GuideError::Llm(
            "LLM returned an empty response for villain profile generation.".into(),
        )
        .into());
    }

    Ok(Json(serde_json::json!({ "villain_profile": content })))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/npcs/generate",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = GenerateNpcRequest,
    responses(
        (status = 201, description = "NPC generated and created", body = Character)
    )
)]
async fn generate_npc(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<GenerateNpcRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let llm_req = CompletionRequest {
        task: LlmTask::General,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: "You are a D&D 5e NPC generator. Given a prompt, respond with a JSON object with fields: name (string), race (string), class (string), level (integer 1-20), max_hp (integer), armor_class (integer), backstory_text (string). Output only valid JSON.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: req.prompt.clone(),
            },
        ],
        model_override: None,
        temperature: Some(0.8),
        max_tokens: Some(512),
    };

    let resp = state.llm.complete(llm_req).await?;
    let raw = resp.content.trim();
    let json_str = raw
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(serde::Deserialize)]
    struct NpcDraft {
        name: String,
        race: Option<String>,
        class: Option<String>,
        level: Option<i32>,
        max_hp: Option<i32>,
        armor_class: Option<i32>,
        backstory_text: Option<String>,
    }

    let draft: NpcDraft = serde_json::from_str(json_str)
        .map_err(|e| GuideError::Llm(format!("Failed to parse NPC JSON: {e}")))?;

    let create_req = CreateCharacterRequest {
        name: draft.name,
        character_type: CharacterType::Npc,
        class: draft.class,
        race: draft.race,
        level: draft.level,
        max_hp: draft.max_hp.unwrap_or(10),
        armor_class: draft.armor_class.unwrap_or(10),
        speed: None,
        ability_scores: None,
        backstory_text: draft.backstory_text,
        spell_slots: None,
    };

    let repo = CharacterRepository::new(&state.db);
    let character = repo.create(campaign_id, create_req).await?;
    Ok((StatusCode::CREATED, Json(character)))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/characters/{id}/spell-slots/spend",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Character ID")
    ),
    request_body = SpendSlotRequest,
    responses(
        (status = 200, description = "Spell slot spent", body = Character),
        (status = 404, description = "Character not found")
    )
)]
async fn spend_spell_slot(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SpendSlotRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CharacterRepository::new(&state.db);
    let character = repo.spend_spell_slot(id, req.level).await?;
    Ok(Json(character))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/characters/{id}/spell-slots/restore",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Character ID")
    ),
    request_body = RestoreSlotRequest,
    responses(
        (status = 200, description = "Spell slots restored", body = Character),
        (status = 404, description = "Character not found")
    )
)]
async fn restore_spell_slots(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<RestoreSlotRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = CharacterRepository::new(&state.db);
    let character = repo.restore_spell_slots(id, req.level).await?;
    Ok(Json(character))
}

async fn upload_portrait(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use tokio::io::AsyncWriteExt;

    let portraits_dir = std::path::PathBuf::from("data/portraits");
    tokio::fs::create_dir_all(&portraits_dir)
        .await
        .map_err(|e| GuideError::Internal(format!("Failed to create portraits dir: {e}")))?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| GuideError::InvalidInput(format!("Multipart error: {e}")))?
    {
        let file_name = field
            .file_name()
            .unwrap_or("portrait.png")
            .to_string();
        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let dest_path = portraits_dir.join(format!("{id}.{ext}"));
        let data = field
            .bytes()
            .await
            .map_err(|e| GuideError::InvalidInput(format!("Failed to read field bytes: {e}")))?;

        let mut file = tokio::fs::File::create(&dest_path)
            .await
            .map_err(|e| GuideError::Internal(format!("Failed to create file: {e}")))?;
        file.write_all(&data)
            .await
            .map_err(|e| GuideError::Internal(format!("Failed to write file: {e}")))?;

        let url = format!("/portraits/{id}.{ext}");
        let repo = CharacterRepository::new(&state.db);
        repo.update_portrait_url(id, &url).await?;
        let character = repo.get_by_id(id).await?;
        return Ok(Json(character));
    }

    Err(GuideError::InvalidInput("No file field found in multipart request".into()).into())
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/characters/{id}/level-up",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Character ID")
    ),
    responses(
        (status = 200, description = "Level-up advice generated")
    )
)]
async fn level_up_assist(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let repo = CharacterRepository::new(&state.db);
    let character = repo.get_by_id(id).await?;

    let prompt = format!(
        "A D&D 5e character named {} (class: {}, race: {}, current level: {}) is leveling up to level {}. \
         Provide concise, rules-accurate advice on what features, HP, and options they gain at the new level.",
        character.name,
        character.class.as_deref().unwrap_or("Unknown"),
        character.race.as_deref().unwrap_or("Unknown"),
        character.level,
        character.level + 1,
    );

    let llm_req = CompletionRequest {
        task: LlmTask::General,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: "You are a D&D 5e rules expert assisting a Dungeon Master with level-up guidance. Be concise and accurate.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: prompt,
            },
        ],
        model_override: None,
        temperature: Some(0.7),
        max_tokens: Some(512),
    };

    let resp = state.llm.complete(llm_req).await?;
    Ok(Json(serde_json::json!({ "advice": resp.content })))
}
