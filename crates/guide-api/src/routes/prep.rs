use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::{
    models::{
        CharacterRoadmapRequest, CharacterType, DmPrepResult, PrepType, SessionRecapRequest,
        StoryContextRequest,
    },
    GuideError,
};
use guide_db::{
    campaigns::CampaignRepository,
    characters::CharacterRepository,
    prep::DmPrepRepository,
    sessions::{SessionEventRepository, SessionRepository},
    story::StoryRepository,
};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/campaigns/{campaign_id}/prep", get(list_prep_results))
        .route(
            "/campaigns/{campaign_id}/prep/session-recap",
            get(get_session_recap).post(generate_session_recap),
        )
        .route(
            "/campaigns/{campaign_id}/prep/story-so-far",
            get(get_story_so_far).post(generate_story_so_far),
        )
        .route(
            "/campaigns/{campaign_id}/prep/story-ahead",
            get(get_story_ahead).post(generate_story_ahead),
        )
        .route(
            "/campaigns/{campaign_id}/prep/character-roadmap/{char_id}",
            get(get_character_roadmap).post(generate_character_roadmap),
        )
}

async fn list_prep_results(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DmPrepRepository::new(&state.db);
    let results: Vec<DmPrepResult> = repo.list_by_campaign(campaign_id).await?;
    Ok(Json(results))
}

async fn get_session_recap(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DmPrepRepository::new(&state.db);
    match repo.get(campaign_id, PrepType::SessionRecap, None).await? {
        Some(result) => Ok(Json(result).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

async fn generate_session_recap(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    body: Option<Json<SessionRecapRequest>>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{prompts, CompletionRequest, LlmTask, Message, MessageRole};

    let force = body.as_ref().and_then(|b| b.force_regenerate).unwrap_or(false);
    let prep_repo = DmPrepRepository::new(&state.db);

    // Return cached result if available and not forced
    if !force {
        if let Some(cached) = prep_repo
            .get(campaign_id, PrepType::SessionRecap, None)
            .await?
        {
            return Ok(Json(cached));
        }
    }

    // Load all sessions and their events
    let session_repo = SessionRepository::new(&state.db);
    let sessions = session_repo.list_by_campaign(campaign_id).await?;
    if sessions.is_empty() {
        return Err(
            GuideError::InvalidInput("No sessions found for this campaign".into()).into(),
        );
    }

    let event_repo = SessionEventRepository::new(&state.db);
    let mut sessions_data = String::new();
    for session in &sessions {
        let events = event_repo.list_by_session(session.id).await?;
        let title = session.title.as_deref().unwrap_or("Untitled");
        sessions_data.push_str(&format!(
            "\n### Session {} — {}\n",
            session.session_number, title
        ));
        if events.is_empty() {
            sessions_data.push_str("(no events recorded)\n");
        } else {
            for e in &events {
                let sig = format!("{:?}", e.significance);
                let vis = if e.is_player_visible {
                    "player-visible"
                } else {
                    "dm-only"
                };
                sessions_data.push_str(&format!(
                    "- [{}|{}] {}\n",
                    sig, vis, e.description
                ));
            }
        }
    }

    // Load PCs with backstory context
    let char_repo = CharacterRepository::new(&state.db);
    let characters = char_repo.list_by_campaign(campaign_id).await?;
    let pcs: Vec<_> = characters
        .iter()
        .filter(|c| c.character_type == CharacterType::Pc)
        .collect();

    let mut characters_data = String::new();
    for pc in &pcs {
        characters_data.push_str(&format!(
            "\n### {} ({} {} Lv{})\n",
            pc.name,
            pc.race.as_deref().unwrap_or("Unknown"),
            pc.class.as_deref().unwrap_or("Unknown"),
            pc.level
        ));
        if let Some(backstory) = &pc.backstory {
            if !backstory.raw_text.is_empty() {
                characters_data
                    .push_str(&format!("Backstory: {}\n", backstory.raw_text));
            }
            for hook in &backstory.extracted_hooks {
                characters_data.push_str(&format!(
                    "Plot hook [{:?}]: {}\n",
                    hook.priority, hook.description
                ));
            }
        }
    }

    let system_prompt = prompts::session_recap_system();
    let user_message = prompts::session_recap_user(&sessions_data, &characters_data);

    let req = CompletionRequest {
        task: LlmTask::SessionRecap,
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
        temperature: Some(0.7),
        max_tokens: Some(6000),
        json_mode: false,
    };

    let resp = state.llm.complete(req).await?;
    let result = prep_repo
        .upsert(campaign_id, PrepType::SessionRecap, resp.content, None)
        .await?;
    Ok(Json(result))
}

async fn get_story_so_far(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DmPrepRepository::new(&state.db);
    match repo.get(campaign_id, PrepType::StorySoFar, None).await? {
        Some(result) => Ok(Json(result).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

async fn generate_story_so_far(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    body: Option<Json<StoryContextRequest>>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{prompts, CompletionRequest, LlmTask, Message, MessageRole};
    use guide_pdf::pipeline::query_indexes;

    let chapter_override = body.as_ref().and_then(|b| b.current_chapter.clone());
    let force = body.as_ref().and_then(|b| b.force_regenerate).unwrap_or(false);
    let prep_repo = DmPrepRepository::new(&state.db);

    // Only serve cache if neither force nor an explicit chapter override was given
    if !force && chapter_override.is_none() {
        if let Some(cached) = prep_repo
            .get(campaign_id, PrepType::StorySoFar, None)
            .await?
        {
            return Ok(Json(cached));
        }
    }

    // Resolve chapter after cache check
    let campaign = CampaignRepository::new(&state.db)
        .get_by_id(campaign_id)
        .await?;
    let current_chapter = chapter_override
        .or(campaign.current_chapter)
        .unwrap_or_else(|| "beginning".to_string());

    // Fetch structured story data first so it can satisfy the guard
    let story_repo = StoryRepository::new(&state.db);
    let arcs = story_repo.list_arcs(campaign_id).await.unwrap_or_default();
    let events = story_repo.list_events(campaign_id).await.unwrap_or_default();

    // RAG query for past content
    let query = format!("campaign history setting lore up to {current_chapter}");
    let chunks = query_indexes(
        &query,
        Some(campaign_id),
        false,
        state.llm.as_ref(),
        &state.db,
    )
    .await
    .unwrap_or_default();

    if chunks.is_empty() && arcs.is_empty() && events.is_empty() {
        return Err(GuideError::InvalidInput(
            "No indexed documents found. Please ingest campaign documents first.".into(),
        )
        .into());
    }

    let context = chunks
        .iter()
        .take(15)
        .map(|c| {
            if c.section_path.is_empty() {
                c.content.clone()
            } else {
                format!("[{}]\n{}", c.section_path, c.content)
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let arcs_json = serde_json::to_string(&arcs).unwrap_or_else(|_| "[]".into());
    let events_json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".into());

    let req = CompletionRequest {
        task: LlmTask::StorySoFar,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: "You are a DM narrative assistant. Summarize the story so far for the Dungeon Master.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: prompts::story_so_far_structured(
                    &current_chapter,
                    &arcs_json,
                    &events_json,
                    &context,
                ),
            },
        ],
        model_override: None,
        temperature: Some(0.5),
        max_tokens: Some(5000),
        json_mode: false,
    };

    let resp = state.llm.complete(req).await?;
    let result = prep_repo
        .upsert(campaign_id, PrepType::StorySoFar, resp.content, None)
        .await?;
    Ok(Json(result))
}

async fn get_story_ahead(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DmPrepRepository::new(&state.db);
    match repo.get(campaign_id, PrepType::StoryAhead, None).await? {
        Some(result) => Ok(Json(result).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

async fn generate_story_ahead(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    body: Option<Json<StoryContextRequest>>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{prompts, CompletionRequest, LlmTask, Message, MessageRole};
    use guide_pdf::pipeline::query_indexes;

    let chapter_override = body.as_ref().and_then(|b| b.current_chapter.clone());
    let force = body.as_ref().and_then(|b| b.force_regenerate).unwrap_or(false);
    let prep_repo = DmPrepRepository::new(&state.db);

    // Only serve cache if neither force nor an explicit chapter override was given
    if !force && chapter_override.is_none() {
        if let Some(cached) = prep_repo
            .get(campaign_id, PrepType::StoryAhead, None)
            .await?
        {
            return Ok(Json(cached));
        }
    }

    // Resolve chapter after cache check
    let campaign = CampaignRepository::new(&state.db)
        .get_by_id(campaign_id)
        .await?;
    let current_chapter = chapter_override
        .or(campaign.current_chapter)
        .unwrap_or_else(|| "beginning".to_string());

    // Fetch structured story data first so it can satisfy the guard
    let story_repo = StoryRepository::new(&state.db);
    let arcs = story_repo.list_arcs(campaign_id).await.unwrap_or_default();
    let events = story_repo.list_events(campaign_id).await.unwrap_or_default();

    // RAG query for future content
    let query = format!("upcoming encounters events revelations after {current_chapter}");
    let chunks = query_indexes(
        &query,
        Some(campaign_id),
        false,
        state.llm.as_ref(),
        &state.db,
    )
    .await
    .unwrap_or_default();

    if chunks.is_empty() && arcs.is_empty() && events.is_empty() {
        return Err(GuideError::InvalidInput(
            "No indexed documents found. Please ingest campaign documents first.".into(),
        )
        .into());
    }

    let context = chunks
        .iter()
        .take(15)
        .map(|c| {
            if c.section_path.is_empty() {
                c.content.clone()
            } else {
                format!("[{}]\n{}", c.section_path, c.content)
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let arcs_json = serde_json::to_string(&arcs).unwrap_or_else(|_| "[]".into());
    let events_json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".into());

    let req = CompletionRequest {
        task: LlmTask::StoryAhead,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: "You are a DM narrative assistant. Describe upcoming story possibilities for the Dungeon Master.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: prompts::story_ahead_structured(
                    &current_chapter,
                    &arcs_json,
                    &events_json,
                    &context,
                ),
            },
        ],
        model_override: None,
        temperature: Some(0.6),
        max_tokens: Some(5000),
        json_mode: false,
    };

    let resp = state.llm.complete(req).await?;
    let result = prep_repo
        .upsert(campaign_id, PrepType::StoryAhead, resp.content, None)
        .await?;
    Ok(Json(result))
}

async fn get_character_roadmap(
    State(state): State<AppState>,
    Path((campaign_id, char_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = DmPrepRepository::new(&state.db);
    match repo
        .get(campaign_id, PrepType::CharacterRoadmap, Some(char_id))
        .await?
    {
        Some(result) => Ok(Json(result).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

async fn generate_character_roadmap(
    State(state): State<AppState>,
    Path((campaign_id, char_id)): Path<(Uuid, Uuid)>,
    body: Option<Json<CharacterRoadmapRequest>>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{prompts, CompletionRequest, LlmTask, Message, MessageRole};
    use guide_pdf::pipeline::query_indexes;

    let chapter_override = body.as_ref().and_then(|b| b.current_chapter.clone());
    let force = body.as_ref().and_then(|b| b.force_regenerate).unwrap_or(false);
    let prep_repo = DmPrepRepository::new(&state.db);

    // Only serve cache if neither force nor an explicit chapter override was given
    if !force && chapter_override.is_none() {
        if let Some(cached) = prep_repo
            .get(campaign_id, PrepType::CharacterRoadmap, Some(char_id))
            .await?
        {
            return Ok(Json(cached));
        }
    }

    // Resolve chapter after cache check
    let campaign = CampaignRepository::new(&state.db)
        .get_by_id(campaign_id)
        .await?;
    let current_chapter = chapter_override
        .or(campaign.current_chapter)
        .unwrap_or_else(|| "beginning".to_string());

    // Load character and verify it belongs to this campaign
    let char_repo = CharacterRepository::new(&state.db);
    let character = char_repo.get_by_id(char_id).await?;
    if character.campaign_id != campaign_id {
        return Err(GuideError::NotFound(format!("Character {char_id}")).into());
    }

    // Build character data string
    let mut character_data = format!(
        "Name: {}\nRace: {}\nClass: {}\nLevel: {}\nHP: {}/{}\nAC: {}\n",
        character.name,
        character.race.as_deref().unwrap_or("Unknown"),
        character.class.as_deref().unwrap_or("Unknown"),
        character.level,
        character.current_hp,
        character.max_hp,
        character.armor_class
    );
    if let Some(backstory) = &character.backstory {
        if !backstory.raw_text.is_empty() {
            character_data.push_str(&format!("\nBackstory:\n{}\n", backstory.raw_text));
        }
        if !backstory.motivations.is_empty() {
            character_data.push_str(&format!(
                "\nMotivations: {}\n",
                backstory.motivations.join(", ")
            ));
        }
        for hook in &backstory.extracted_hooks {
            character_data.push_str(&format!(
                "Plot hook [{:?}]: {}\n",
                hook.priority, hook.description
            ));
        }
    }

    // Load session events involving this character
    let session_repo = SessionRepository::new(&state.db);
    let event_repo = SessionEventRepository::new(&state.db);
    let sessions = session_repo.list_by_campaign(campaign_id).await?;
    let mut events_data = String::new();
    for session in &sessions {
        let events = event_repo.list_by_session(session.id).await?;
        let char_events: Vec<_> = events
            .iter()
            .filter(|e| e.involved_character_ids.contains(&char_id))
            .collect();
        if !char_events.is_empty() {
            events_data.push_str(&format!("\n#### Session {}\n", session.session_number));
            for e in char_events {
                events_data.push_str(&format!(
                    "- [{:?}] {}\n",
                    e.significance, e.description
                ));
            }
        }
    }
    if events_data.is_empty() {
        events_data =
            "(no session events recorded for this character yet)".to_string();
    }

    // RAG query for character-relevant upcoming content
    let char_name = &character.name;
    let char_class = character.class.as_deref().unwrap_or("adventurer");
    let rag_query = format!("{char_name} {char_class} encounter after {current_chapter} upcoming destiny");
    let chunks = query_indexes(
        &rag_query,
        Some(campaign_id),
        false,
        state.llm.as_ref(),
        &state.db,
    )
    .await
    .unwrap_or_default();

    let rag_context = if chunks.is_empty() {
        "(no indexed documents available)".to_string()
    } else {
        chunks
            .iter()
            .take(10)
            .map(|c| {
                if c.section_path.is_empty() {
                    c.content.clone()
                } else {
                    format!("[{}]\n{}", c.section_path, c.content)
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    };

    let system_prompt = prompts::character_roadmap_system();
    let user_message =
        prompts::character_roadmap_user(&character_data, &events_data, &rag_context);

    let req = CompletionRequest {
        task: LlmTask::CharacterRoadmap,
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
        temperature: Some(0.7),
        max_tokens: Some(5000),
        json_mode: false,
    };

    let resp = state.llm.complete(req).await?;
    let result = prep_repo
        .upsert(
            campaign_id,
            PrepType::CharacterRoadmap,
            resp.content,
            Some(char_id),
        )
        .await?;
    Ok(Json(result))
}
