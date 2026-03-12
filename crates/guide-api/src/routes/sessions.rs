use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use guide_core::{
    models::{
        CreateSessionEventRequest, CreateSessionRequest, ImprovPromptResponse, Perspective,
        Session, SessionEvent, SessionSummary,
    },
    GuideError,
};
use guide_db::sessions::{SessionEventRepository, SessionRepository};
use serde::Deserialize;
use uuid::Uuid;

use crate::routes::webhooks::fire_webhooks;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/sessions",
            get(list_sessions).post(create_session),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}",
            get(get_session).delete(delete_session),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/start",
            post(start_session),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/end",
            post(end_session),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/events",
            get(list_events).post(create_event),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/events/{event_id}",
            axum::routing::delete(delete_event),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/summary",
            get(get_summary),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/summary/export",
            get(export_summary),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/improv-prompt",
            post(get_improv_prompt),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/debrief",
            post(generate_debrief),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/prep",
            post(session_prep),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{id}/map",
            post(upload_map),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/sessions",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    responses(
        (status = 200, description = "List all sessions in a campaign", body = [Session])
    )
)]
async fn list_sessions(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    Ok(Json(repo.list_by_campaign(campaign_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/sessions",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = CreateSessionRequest,
    responses(
        (status = 201, description = "Session created successfully", body = Session)
    )
)]
async fn create_session(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(campaign_id, req).await?)))
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/sessions/{id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Found session", body = Session),
        (status = 404, description = "Session not found")
    )
)]
async fn get_session(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    Ok(Json(repo.get_by_id(id).await?))
}

#[utoipa::path(
    delete,
    path = "/campaigns/{campaign_id}/sessions/{id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 204, description = "Session deleted successfully"),
        (status = 404, description = "Session not found")
    )
)]
async fn delete_session(
    State(state): State<AppState>,
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/sessions/{id}/start",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Session started", body = Session),
        (status = 404, description = "Session not found")
    )
)]
async fn start_session(
    State(state): State<AppState>,
    Path((campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    let session = repo.start_session(id).await?;
    let session_name = session.title.clone().unwrap_or_else(|| format!("Session {}", session.session_number));
    fire_webhooks(
        state.db.clone(),
        campaign_id,
        "session_start".to_string(),
        serde_json::json!({
            "event": "session_start",
            "campaign_id": campaign_id.to_string(),
            "session_id": id.to_string(),
            "session_name": session_name,
        }),
    );
    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/sessions/{id}/end",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Session ended", body = Session),
        (status = 404, description = "Session not found")
    )
)]
async fn end_session(
    State(state): State<AppState>,
    Path((campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    let session = repo.end_session(id).await?;
    let session_name = session.title.clone().unwrap_or_else(|| format!("Session {}", session.session_number));
    fire_webhooks(
        state.db.clone(),
        campaign_id,
        "session_end".to_string(),
        serde_json::json!({
            "event": "session_end",
            "campaign_id": campaign_id.to_string(),
            "session_id": id.to_string(),
            "session_name": session_name,
        }),
    );
    Ok(Json(session))
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/sessions/{id}/events",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "List all events in a session", body = [SessionEvent])
    )
)]
async fn list_events(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionEventRepository::new(&state.db);
    Ok(Json(repo.list_by_session(session_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/sessions/{id}/events",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    request_body = CreateSessionEventRequest,
    responses(
        (status = 201, description = "Event created successfully", body = SessionEvent)
    )
)]
async fn create_event(
    State(state): State<AppState>,
    Path((campaign_id, session_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateSessionEventRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionEventRepository::new(&state.db);
    Ok((
        StatusCode::CREATED,
        Json(repo.create(session_id, campaign_id, req).await?),
    ))
}

async fn delete_event(
    State(state): State<AppState>,
    Path((_campaign_id, _session_id, event_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionEventRepository::new(&state.db);
    repo.delete_event(event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct SummaryQuery {
    pub perspective: Option<String>,
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/sessions/{id}/summary",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID"),
        SummaryQuery
    ),
    responses(
        (status = 200, description = "Session summary generated", body = SessionSummary),
        (status = 404, description = "Session not found")
    )
)]
async fn get_summary(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(q): axum::extract::Query<SummaryQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{prompts, CompletionRequest, LlmTask, Message, MessageRole};

    let perspective = match q.perspective.as_deref().map(str::to_lowercase).as_deref() {
        Some("player") => Perspective::Player,
        _ => Perspective::Dm,
    };

    let event_repo = SessionEventRepository::new(&state.db);
    let events = event_repo.list_by_session(session_id).await?;

    if events.is_empty() {
        return Err(
            GuideError::InvalidInput("Session has no events to summarize".into()).into()
        );
    }

    let events_text = events
        .iter()
        .map(|e| format!("[{:?}] {}", e.event_type, e.description))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = match perspective {
        Perspective::Dm => prompts::session_summary_dm_system().to_string(),
        Perspective::Player => prompts::session_summary_player_system().to_string(),
    };

    let req = CompletionRequest {
        task: LlmTask::SessionSummary,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: system_prompt,
            },
            Message {
                role: MessageRole::User,
                content: events_text,
            },
        ],
        model_override: None,
        temperature: Some(0.7),
        max_tokens: Some(2048),
    };

    let resp = state.llm.complete(req).await?;

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "perspective": perspective,
        "content": resp.content,
        "generated_at": chrono::Utc::now().to_rfc3339(),
    })))
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ExportQuery {
    pub perspective: Option<String>,
    pub format: Option<String>,
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/sessions/{id}/summary/export",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID"),
        ExportQuery
    ),
    responses(
        (status = 200, description = "Session summary exported as markdown file"),
        (status = 404, description = "Session not found")
    )
)]
async fn export_summary(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(q): axum::extract::Query<ExportQuery>,
) -> Result<Response, crate::error::AppError> {
    use guide_llm::{prompts, CompletionRequest, LlmTask, Message, MessageRole};

    let perspective = match q.perspective.as_deref().map(str::to_lowercase).as_deref() {
        Some("player") => Perspective::Player,
        _ => Perspective::Dm,
    };

    let session_repo = SessionRepository::new(&state.db);
    let session = session_repo.get_by_id(session_id).await?;

    let event_repo = SessionEventRepository::new(&state.db);
    let events = event_repo.list_by_session(session_id).await?;

    if events.is_empty() {
        return Err(GuideError::InvalidInput("Session has no events to summarize".into()).into());
    }

    let events_text = events
        .iter()
        .map(|e| format!("[{:?}] {}", e.event_type, e.description))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = match perspective {
        Perspective::Dm => prompts::session_summary_dm_system().to_string(),
        Perspective::Player => prompts::session_summary_player_system().to_string(),
    };

    let req = CompletionRequest {
        task: LlmTask::SessionSummary,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: system_prompt,
            },
            Message {
                role: MessageRole::User,
                content: events_text,
            },
        ],
        model_override: None,
        temperature: Some(0.7),
        max_tokens: Some(2048),
    };

    let resp = state.llm.complete(req).await?;

    let perspective_label = match perspective {
        Perspective::Dm => "DM",
        Perspective::Player => "Player",
    };
    let header_text = format!(
        "# Session {} Summary — {} Perspective\n_Generated: {}_\n\n",
        session.session_number,
        perspective_label,
        chrono::Utc::now().to_rfc3339(),
    );
    let content = format!("{}{}", header_text, resp.content);
    let filename = format!("session-{}-summary.md", session.session_number);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(content))
        .unwrap())
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/sessions/{id}/improv-prompt",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Three improv narrative options generated", body = ImprovPromptResponse),
        (status = 404, description = "Session not found")
    )
)]
async fn get_improv_prompt(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let event_repo = SessionEventRepository::new(&state.db);
    let events = event_repo.list_by_session(session_id).await?;

    let events_text = if events.is_empty() {
        "No events recorded yet for this session.".to_string()
    } else {
        events
            .iter()
            .map(|e| format!("[{:?}] {}", e.event_type, e.description))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let improv_system = concat!(
        "You are an expert improv storytelling coach for tabletop RPGs. ",
        "Given the session events so far, generate exactly 3 narrative options the DM could use right now. ",
        "Return a JSON object with a single key \"options\" containing an array of 3 strings. ",
        "Each option should be a single sentence, vivid and immediately actionable."
    );

    let req = CompletionRequest {
        task: LlmTask::General,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: improv_system.to_string(),
            },
            Message {
                role: MessageRole::User,
                content: events_text,
            },
        ],
        model_override: None,
        temperature: Some(0.9),
        max_tokens: Some(512),
    };

    let resp = state.llm.complete(req).await?;
    let raw = resp.content.trim();
    if raw.is_empty() {
        return Err(GuideError::Llm(
            "LLM returned an empty response for improv prompt.".into(),
        )
        .into());
    }

    let json_str = raw
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(serde::Deserialize)]
    struct ImprovRaw {
        options: Vec<String>,
    }

    let parsed: ImprovRaw = serde_json::from_str(json_str)
        .map_err(|e| GuideError::Llm(format!("Failed to parse improv JSON: {e}")))?;

    Ok(Json(ImprovPromptResponse {
        options: parsed.options,
    }))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/sessions/{id}/debrief",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "AI post-session debrief generated"),
        (status = 404, description = "Session not found")
    )
)]
async fn generate_debrief(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let session_repo = SessionRepository::new(&state.db);
    let _session = session_repo.get_by_id(session_id).await?;

    let event_repo = SessionEventRepository::new(&state.db);
    let events = event_repo.list_by_session(session_id).await?;

    let events_text = if events.is_empty() {
        "No events recorded yet for this session.".to_string()
    } else {
        events
            .iter()
            .map(|e| {
                format!(
                    "[{:?}] ({:?}): {}",
                    e.event_type, e.significance, e.description
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system_prompt = concat!(
        "You are an expert D&D Dungeon Master coach. Given session events, ",
        "generate a structured post-session debrief with these sections:\n",
        "- **What Went Well**: Highlight memorable moments and player engagement\n",
        "- **What Was Confusing**: Identify unclear moments or pacing issues\n",
        "- **Open Plot Threads**: List unresolved hooks and dangling story elements\n",
        "- **Suggested Follow-Up**: Concrete suggestions for the next session\n",
        "Keep it concise and actionable. Output plain text with markdown headers."
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
                content: events_text,
            },
        ],
        model_override: None,
        temperature: Some(0.7),
        max_tokens: Some(1024),
    };

    let resp = state.llm.complete(req).await?;

    Ok(Json(serde_json::json!({ "debrief": resp.content })))
}

#[derive(Deserialize)]
struct SessionPrepRequest {
    upcoming_notes: Option<String>,
}

async fn session_prep(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    body: Option<Json<SessionPrepRequest>>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let upcoming_notes = body
        .as_ref()
        .and_then(|b| b.upcoming_notes.clone())
        .unwrap_or_default();

    // Fetch the most recent session's events as context
    let session_repo = SessionRepository::new(&state.db);
    let sessions = session_repo.list_by_campaign(campaign_id).await?;

    let context_text = if let Some(last_session) = sessions.last() {
        let event_repo = SessionEventRepository::new(&state.db);
        let events = event_repo.list_by_session(last_session.id).await?;
        if events.is_empty() {
            format!("Last session: Session {}", last_session.session_number)
        } else {
            let events_text = events
                .iter()
                .map(|e| format!("[{:?}] {}", e.event_type, e.description))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Last session (Session {}) events:\n{}",
                last_session.session_number, events_text
            )
        }
    } else {
        "No previous sessions recorded yet.".to_string()
    };

    let system_prompt = concat!(
        "You are a D&D Dungeon Master assistant. Based on the previous session context, ",
        "generate a structured session prep document with these sections: ",
        "**Key NPCs to Feature** (3 NPCs with brief role), ",
        "**Likely Player Choices** (3 decision points the players may face), ",
        "**Scene Hooks** (3 scene openings with atmosphere), ",
        "**Encounter Suggestion** (one tailored combat/social/exploration encounter), ",
        "**Foreshadowing Moments** (2 hints at future plot). ",
        "Be specific and actionable."
    );

    let user_message = if upcoming_notes.is_empty() {
        context_text
    } else {
        format!("{}\n\nUpcoming notes from DM: {}", context_text, upcoming_notes)
    };

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
        temperature: Some(0.8),
        max_tokens: Some(1500),
    };

    let resp = state.llm.complete(req).await?;

    Ok(Json(serde_json::json!({ "prep": resp.content })))
}

async fn upload_map(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use tokio::io::AsyncWriteExt;

    let maps_dir = std::path::PathBuf::from("data/maps");
    tokio::fs::create_dir_all(&maps_dir)
        .await
        .map_err(|e| GuideError::Internal(format!("Failed to create maps dir: {e}")))?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| GuideError::InvalidInput(format!("Multipart error: {e}")))?
        .ok_or_else(|| GuideError::InvalidInput("No file field found in multipart request".into()))?;

    let file_name = field.file_name().unwrap_or("map.png").to_string();
    let ext = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let dest_path = maps_dir.join(format!("{session_id}.{ext}"));
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

    let url = format!("/maps/{session_id}.{ext}");
    let repo = SessionRepository::new(&state.db);
    repo.update_map_url(session_id, &url).await?;
    let session = repo.get_by_id(session_id).await?;
    Ok(Json(session))
}
