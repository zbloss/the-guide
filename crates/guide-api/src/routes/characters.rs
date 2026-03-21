use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use guide_core::{
    models::{
        AbilityScores, Backstory, Character, CharacterType, CreateCharacterRequest,
        CreateTrackedPlotHookRequest, GenerateNpcRequest, HookPriority, ParsedSheetResult,
        PlotHook, PlotHookStatus, RestoreSlotRequest, SavingThrows, SkillScores, SpellSlot,
        SpendSlotRequest, UpdateCharacterRequest,
    },
    GuideError,
};
use guide_db::{characters::CharacterRepository, PlotHookRepository};
use uuid::Uuid;

use crate::state::AppState;

// ── Type-coercing helpers for LLM JSON output ────────────────────────────────

fn jv_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn jv_str_opt(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) if s.is_empty() => None,
        other => {
            let s = jv_str(other);
            if s.is_empty() { None } else { Some(s) }
        }
    }
}

fn jv_i32(v: &serde_json::Value) -> i32 {
    match v {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) as i32,
        serde_json::Value::String(s) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

fn jv_i32_opt(v: &serde_json::Value) -> Option<i32> {
    match v {
        serde_json::Value::Null => None,
        other => Some(jv_i32(other)),
    }
}

fn jv_f32(v: &serde_json::Value) -> f32 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) as f32,
        serde_json::Value::String(s) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn jv_str_vec(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| match item {
                serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn jv_ability_scores(v: &serde_json::Value) -> AbilityScores {
    let null = serde_json::Value::Null;
    if let serde_json::Value::Object(m) = v {
        AbilityScores {
            strength:     jv_i32(m.get("strength").unwrap_or(&null)),
            dexterity:    jv_i32(m.get("dexterity").unwrap_or(&null)),
            constitution: jv_i32(m.get("constitution").unwrap_or(&null)),
            intelligence: jv_i32(m.get("intelligence").unwrap_or(&null)),
            wisdom:       jv_i32(m.get("wisdom").unwrap_or(&null)),
            charisma:     jv_i32(m.get("charisma").unwrap_or(&null)),
        }
    } else {
        AbilityScores::default()
    }
}

fn jv_saving_throws(v: &serde_json::Value) -> Option<SavingThrows> {
    if let serde_json::Value::Object(m) = v {
        let null = serde_json::Value::Null;
        Some(SavingThrows {
            strength:     jv_i32_opt(m.get("strength").unwrap_or(&null)),
            dexterity:    jv_i32_opt(m.get("dexterity").unwrap_or(&null)),
            constitution: jv_i32_opt(m.get("constitution").unwrap_or(&null)),
            intelligence: jv_i32_opt(m.get("intelligence").unwrap_or(&null)),
            wisdom:       jv_i32_opt(m.get("wisdom").unwrap_or(&null)),
            charisma:     jv_i32_opt(m.get("charisma").unwrap_or(&null)),
        })
    } else {
        None
    }
}

fn jv_skill_scores(v: &serde_json::Value) -> Option<SkillScores> {
    if let serde_json::Value::Object(m) = v {
        let null = serde_json::Value::Null;
        Some(SkillScores {
            acrobatics:      jv_i32_opt(m.get("acrobatics").unwrap_or(&null)),
            animal_handling: jv_i32_opt(m.get("animal_handling").unwrap_or(&null)),
            arcana:          jv_i32_opt(m.get("arcana").unwrap_or(&null)),
            athletics:       jv_i32_opt(m.get("athletics").unwrap_or(&null)),
            deception:       jv_i32_opt(m.get("deception").unwrap_or(&null)),
            history:         jv_i32_opt(m.get("history").unwrap_or(&null)),
            insight:         jv_i32_opt(m.get("insight").unwrap_or(&null)),
            intimidation:    jv_i32_opt(m.get("intimidation").unwrap_or(&null)),
            investigation:   jv_i32_opt(m.get("investigation").unwrap_or(&null)),
            medicine:        jv_i32_opt(m.get("medicine").unwrap_or(&null)),
            nature:          jv_i32_opt(m.get("nature").unwrap_or(&null)),
            perception:      jv_i32_opt(m.get("perception").unwrap_or(&null)),
            performance:     jv_i32_opt(m.get("performance").unwrap_or(&null)),
            persuasion:      jv_i32_opt(m.get("persuasion").unwrap_or(&null)),
            religion:        jv_i32_opt(m.get("religion").unwrap_or(&null)),
            sleight_of_hand: jv_i32_opt(m.get("sleight_of_hand").unwrap_or(&null)),
            stealth:         jv_i32_opt(m.get("stealth").unwrap_or(&null)),
            survival:        jv_i32_opt(m.get("survival").unwrap_or(&null)),
        })
    } else {
        None
    }
}

fn jv_spell_slots(v: &serde_json::Value) -> Vec<SpellSlot> {
    let null = serde_json::Value::Null;
    match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                if let serde_json::Value::Object(m) = item {
                    Some(SpellSlot {
                        level:     jv_i32(m.get("level").unwrap_or(&null)),
                        total:     jv_i32(m.get("total").unwrap_or(&null)),
                        remaining: jv_i32(m.get("remaining").unwrap_or(&null)),
                    })
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn build_parsed_sheet_result(val: serde_json::Value, raw_extracted_text: String) -> ParsedSheetResult {
    let null = serde_json::Value::Null;

    let name = jv_str(val.get("name").unwrap_or(&null));
    let name = if name.is_empty() { "Unknown".into() } else { name };

    let level = jv_i32(val.get("level").unwrap_or(&null));
    let level = if level <= 0 { 1 } else { level };

    let max_hp = jv_i32(val.get("max_hp").unwrap_or(&null));
    let max_hp = if max_hp <= 0 { 10 } else { max_hp };

    let armor_class = jv_i32(val.get("armor_class").unwrap_or(&null));
    let armor_class = if armor_class <= 0 { 10 } else { armor_class };

    let speed = jv_i32(val.get("speed").unwrap_or(&null));
    let speed = if speed <= 0 { 30 } else { speed };

    ParsedSheetResult {
        name,
        class: jv_str_opt(val.get("class").unwrap_or(&null)),
        race: jv_str_opt(val.get("race").unwrap_or(&null)),
        level,
        max_hp,
        armor_class,
        speed,
        ability_scores: jv_ability_scores(val.get("ability_scores").unwrap_or(&null)),
        backstory_text: jv_str_opt(val.get("backstory_text").unwrap_or(&null)),
        raw_extracted_text,
        parse_confidence: jv_f32(val.get("parse_confidence").unwrap_or(&null)),
        background: jv_str_opt(val.get("background").unwrap_or(&null)),
        experience_points: {
            let xp = jv_i32(val.get("experience_points").unwrap_or(&null));
            if xp == 0 { None } else { Some(xp) }
        },
        hit_dice: jv_str_opt(val.get("hit_dice").unwrap_or(&null)),
        saving_throws: jv_saving_throws(val.get("saving_throws").unwrap_or(&null)),
        skills: jv_skill_scores(val.get("skills").unwrap_or(&null)),
        proficiencies: jv_str_vec(val.get("proficiencies").unwrap_or(&null)),
        languages: jv_str_vec(val.get("languages").unwrap_or(&null)),
        features_and_traits: jv_str_vec(val.get("features_and_traits").unwrap_or(&null)),
        equipment: jv_str_vec(val.get("equipment").unwrap_or(&null)),
        personality_traits: jv_str_opt(val.get("personality_traits").unwrap_or(&null)),
        ideals: jv_str_opt(val.get("ideals").unwrap_or(&null)),
        bonds: jv_str_opt(val.get("bonds").unwrap_or(&null)),
        flaws: jv_str_opt(val.get("flaws").unwrap_or(&null)),
        spell_slots: jv_spell_slots(val.get("spell_slots").unwrap_or(&null)),
    }
}

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
        .route(
            "/campaigns/{campaign_id}/characters/import-csv",
            post(import_csv),
        )
        .route(
            "/campaigns/{campaign_id}/characters/import-dndbeyond",
            post(import_dndbeyond),
        )
        .route(
            "/campaigns/{campaign_id}/characters/parse-sheet",
            post(parse_sheet),
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
        json_mode: false,
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

    // Auto-populate tracked plot hooks in the plot_hooks table
    let hook_repo = PlotHookRepository::new(&state.db);
    for hook in &backstory.extracted_hooks {
        let create_req = CreateTrackedPlotHookRequest {
            hook_text: hook.description.clone(),
            status: Some(PlotHookStatus::Open),
        };
        if let Err(e) = hook_repo.create(id, &create_req).await {
            tracing::warn!("Failed to create tracked plot hook: {e}");
        }
    }

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
        json_mode: false,
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
        json_mode: false,
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

    let field = multipart
        .next_field()
        .await
        .map_err(|e| GuideError::InvalidInput(format!("Multipart error: {e}")))?
        .ok_or_else(|| GuideError::InvalidInput("No file field found in multipart request".into()))?;

    let file_name = field.file_name().unwrap_or("portrait.png").to_string();
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
    Ok(Json(character))
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
        json_mode: false,
    };

    let resp = state.llm.complete(llm_req).await?;
    Ok(Json(serde_json::json!({ "advice": resp.content })))
}

async fn import_csv(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| GuideError::InvalidInput(format!("Multipart error: {e}")))?
        .ok_or_else(|| GuideError::InvalidInput("No CSV file provided".into()))?;

    let csv_bytes = field
        .bytes()
        .await
        .map_err(|e| GuideError::InvalidInput(format!("Failed to read CSV: {e}")))?;
    let csv_text = std::str::from_utf8(&csv_bytes)
        .map_err(|_| GuideError::InvalidInput("CSV file must be UTF-8 encoded".into()))?;

    let repo = CharacterRepository::new(&state.db);
    let mut created = Vec::new();

    for (line_idx, line) in csv_text.lines().enumerate() {
        if line_idx == 0 {
            continue; // skip header
        }
        let cols: Vec<&str> = line.splitn(10, ',').collect();
        if cols.is_empty() || cols[0].trim().is_empty() {
            continue;
        }
        let name = cols[0].trim().to_string();
        let char_type_str = cols.get(1).map(|s| s.trim()).unwrap_or("pc");
        let char_type = match char_type_str.to_lowercase().as_str() {
            "npc" => guide_core::models::CharacterType::Npc,
            "monster" => guide_core::models::CharacterType::Monster,
            _ => guide_core::models::CharacterType::Pc,
        };
        let class = cols.get(2).filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string());
        let race = cols.get(3).filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string());
        let level = cols.get(4).and_then(|s| s.trim().parse::<i32>().ok());
        let max_hp = cols.get(5).and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(10);
        let armor_class = cols.get(6).and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(10);
        let speed = cols.get(7).and_then(|s| s.trim().parse::<i32>().ok());

        let req = CreateCharacterRequest {
            name,
            character_type: char_type,
            class,
            race,
            level,
            max_hp,
            armor_class,
            speed,
            ability_scores: None,
            backstory_text: None,
            spell_slots: None,
        };
        let character = repo.create(campaign_id, req).await?;
        created.push(character);
    }

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "imported": created.len(),
        "characters": created,
    }))))
}

async fn parse_sheet(
    State(state): State<AppState>,
    Path(_campaign_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let mut pdf_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        crate::error::AppError(guide_core::GuideError::InvalidInput(e.to_string()))
    })? {
        if field.name() == Some("file") {
            pdf_bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        crate::error::AppError(guide_core::GuideError::InvalidInput(e.to_string()))
                    })?
                    .to_vec(),
            );
        }
    }

    let bytes = pdf_bytes.ok_or_else(|| {
        crate::error::AppError(guide_core::GuideError::InvalidInput("No file field".into()))
    })?;

    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    // Strategy 1: Direct AcroForm field parser (no LLM) — works for standard D&D Beyond sheets.
    if let Some(result) = guide_pdf::extractor::parse_character_sheet_from_form(&bytes) {
        if result.parse_confidence >= 0.5 {
            tracing::info!(
                "Direct form parse succeeded: {} (confidence {:.2})",
                result.name, result.parse_confidence
            );
            return Ok(Json(result));
        }
        tracing::info!(
            "Direct form parse confidence too low ({:.2}), falling back to LLM",
            result.parse_confidence
        );
    }

    // Strategy 2: Extract text, then run the text LLM to produce structured JSON.
    // Vision models are not reliable at emitting complex structured JSON in one shot,
    // so we always separate extraction from parsing.
    let raw_extracted_text = guide_pdf::extractor::extract_character_sheet_text(
        &bytes,
        state.llm.as_ref(),
        &state.config.ocr_model,
    )
    .await;

    tracing::info!(
        "Character sheet text extracted: {} chars, {} words",
        raw_extracted_text.len(),
        raw_extracted_text.split_whitespace().count()
    );

    if raw_extracted_text.trim().is_empty() {
        return Ok(Json(ParsedSheetResult {
            name: "Unknown".into(),
            class: None,
            race: None,
            level: 1,
            max_hp: 10,
            armor_class: 10,
            speed: 30,
            ability_scores: guide_core::models::AbilityScores::default(),
            backstory_text: None,
            raw_extracted_text,
            parse_confidence: 0.0,
            background: None,
            experience_points: None,
            hit_dice: None,
            saving_throws: None,
            skills: None,
            proficiencies: Vec::new(),
            languages: Vec::new(),
            features_and_traits: Vec::new(),
            equipment: Vec::new(),
            personality_traits: None,
            ideals: None,
            bonds: None,
            flaws: None,
            spell_slots: Vec::new(),
        }));
    }

    let user_msg = format!(
        "Parse this D&D 5e character sheet and return JSON:\n\n{}",
        &raw_extracted_text
    );

    let llm_req = CompletionRequest {
        task: LlmTask::CharacterSheetParse,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: guide_llm::prompts::character_sheet_parse_system().to_string(),
            },
            Message {
                role: MessageRole::User,
                content: user_msg,
            },
        ],
        model_override: None,
        temperature: Some(0.0),
        max_tokens: Some(2048),
        json_mode: false,
    };

    let resp = state.llm.complete(llm_req).await?;

    // Strip markdown fences the LLM may wrap around the JSON
    let raw_resp = resp.content.trim();
    let json_str = raw_resp
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Use type-coercing Value deserialization to tolerate minor field type mismatches
    let parsed = match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(val) => build_parsed_sheet_result(val, raw_extracted_text.clone()),
        Err(e) => {
            tracing::warn!("Failed to parse LLM sheet JSON: {e}\nRaw response: {json_str}");
            ParsedSheetResult {
                name: "Unknown".into(),
                class: None,
                race: None,
                level: 1,
                max_hp: 10,
                armor_class: 10,
                speed: 30,
                ability_scores: guide_core::models::AbilityScores::default(),
                backstory_text: None,
                raw_extracted_text,
                parse_confidence: 0.0,
                background: None,
                experience_points: None,
                hit_dice: None,
                saving_throws: None,
                skills: None,
                proficiencies: Vec::new(),
                languages: Vec::new(),
                features_and_traits: Vec::new(),
                equipment: Vec::new(),
                personality_traits: None,
                ideals: None,
                bonds: None,
                flaws: None,
                spell_slots: Vec::new(),
            }
        }
    };

    Ok(Json(parsed))
}

async fn import_dndbeyond(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(json): Json<serde_json::Value>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // D&D Beyond export may wrap content in a "data" field
    let char_data = json.get("data").unwrap_or(&json);

    let name = char_data
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GuideError::InvalidInput("Missing character name".into()))?
        .to_string();

    // Extract class (first class definition)
    let class = char_data
        .get("classes")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("definition"))
        .and_then(|d| d.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    // Extract level from first class
    let level = char_data
        .get("classes")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("level"))
        .and_then(|l| l.as_i64())
        .map(|l| l as i32);

    // Extract race
    let race = char_data
        .get("race")
        .and_then(|r| r.get("fullName").or_else(|| r.get("baseName")))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    // Extract HP
    let base_hp = char_data.get("baseHitPoints").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
    let bonus_hp = char_data.get("bonusHitPoints").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let max_hp = base_hp + bonus_hp;

    // Extract AC (from inventory items or a direct field)
    let armor_class = char_data
        .get("inventory")
        .and_then(|inv| inv.as_array())
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("equipped").and_then(|e| e.as_bool()).unwrap_or(false)
                    && item
                        .get("definition")
                        .and_then(|d| d.get("armorClass"))
                        .is_some()
            })
        })
        .and_then(|item| item.get("definition"))
        .and_then(|d| d.get("armorClass"))
        .and_then(|ac| ac.as_i64())
        .unwrap_or(10) as i32;

    // Extract ability scores (stats: [{id:1=STR, 2=DEX, 3=CON, 4=INT, 5=WIS, 6=CHA}])
    let ability_scores = char_data.get("stats").and_then(|stats| {
        let arr = stats.as_array()?;
        let mut scores = guide_core::models::AbilityScores {
            strength: 10, dexterity: 10, constitution: 10,
            intelligence: 10, wisdom: 10, charisma: 10,
        };
        for stat in arr {
            let id = stat.get("id").and_then(|i| i.as_i64()).unwrap_or(0);
            let val = stat.get("value").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
            match id {
                1 => scores.strength = val,
                2 => scores.dexterity = val,
                3 => scores.constitution = val,
                4 => scores.intelligence = val,
                5 => scores.wisdom = val,
                6 => scores.charisma = val,
                _ => {}
            }
        }
        Some(scores)
    });

    let req = CreateCharacterRequest {
        name,
        character_type: CharacterType::Pc,
        class,
        race,
        level,
        max_hp,
        armor_class,
        speed: None,
        ability_scores,
        backstory_text: None,
        spell_slots: None,
    };

    let repo = CharacterRepository::new(&state.db);
    let character = repo.create(campaign_id, req).await?;

    Ok((StatusCode::CREATED, Json(character)))
}
