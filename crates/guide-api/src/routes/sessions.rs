use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use guide_core::{
    models::{
        CreateSessionEventRequest, CreateSessionRequest, Perspective, Session, SessionEvent,
        SessionSummary,
    },
    GuideError,
};
use guide_db::sessions::{SessionEventRepository, SessionRepository};
use serde::Deserialize;
use uuid::Uuid;

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
            "/campaigns/{campaign_id}/sessions/{id}/summary",
            get(get_summary),
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
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    Ok(Json(repo.start_session(id).await?))
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
    Path((_campaign_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = SessionRepository::new(&state.db);
    Ok(Json(repo.end_session(id).await?))
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
