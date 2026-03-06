use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use guide_core::models::{
    CreateSessionEventRequest, CreateSessionRequest, Perspective, SessionSummary,
};
use guide_db::sessions::{SessionEventRepository, SessionRepository};
use guide_llm::{
    client::{CompletionRequest, LlmTask, Message, MessageRole},
    prompts,
};
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
            "/campaigns/{campaign_id}/sessions/{session_id}",
            get(get_session).delete(delete_session),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{session_id}/start",
            post(start_session),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{session_id}/end",
            post(end_session),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{session_id}/events",
            get(list_events).post(create_event),
        )
        .route(
            "/campaigns/{campaign_id}/sessions/{session_id}/summary",
            get(session_summary),
        )
}

async fn list_sessions(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = SessionRepository::new(&state.db);
    match repo.list_by_campaign(campaign_id).await {
        Ok(sessions) => (StatusCode::OK, Json(sessions)).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn create_session(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let repo = SessionRepository::new(&state.db);
    match repo.create(campaign_id, req).await {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn get_session(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = SessionRepository::new(&state.db);
    match repo.get_by_id(session_id).await {
        Ok(session) => (StatusCode::OK, Json(session)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn start_session(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = SessionRepository::new(&state.db);
    match repo.start_session(session_id).await {
        Ok(session) => (StatusCode::OK, Json(session)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn end_session(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = SessionRepository::new(&state.db);
    match repo.end_session(session_id).await {
        Ok(session) => (StatusCode::OK, Json(session)).into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn delete_session(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = SessionRepository::new(&state.db);
    match repo.delete(session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(guide_core::GuideError::NotFound(msg)) => error_response(StatusCode::NOT_FOUND, &msg),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn list_events(
    State(state): State<AppState>,
    Path((_campaign_id, session_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = SessionEventRepository::new(&state.db);
    match repo.list_by_session(session_id).await {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn create_event(
    State(state): State<AppState>,
    Path((campaign_id, session_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateSessionEventRequest>,
) -> impl IntoResponse {
    let repo = SessionEventRepository::new(&state.db);
    match repo.create(session_id, campaign_id, req).await {
        Ok(event) => (StatusCode::CREATED, Json(event)).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// ── Session Summary ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SummaryQuery {
    perspective: Option<String>,
}

/// Generate a tiered session summary via LLM.
/// `?perspective=dm` (default) → full DM master log
/// `?perspective=players` → spoiler-free player recap
async fn session_summary(
    State(state): State<AppState>,
    Path((campaign_id, session_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<SummaryQuery>,
) -> impl IntoResponse {
    let perspective = match query.perspective.as_deref() {
        Some("players") | Some("player") => Perspective::Player,
        _ => Perspective::Dm,
    };

    let event_repo = SessionEventRepository::new(&state.db);

    // Fetch events according to perspective
    let events = match perspective {
        Perspective::Player => event_repo.list_visible_by_session(session_id).await,
        Perspective::Dm => event_repo.list_by_session(session_id).await,
    };

    let events = match events {
        Ok(e) => e,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    if events.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "No events recorded for this session yet",
        );
    }

    // Format events as a numbered list for the LLM
    let event_list = events
        .iter()
        .enumerate()
        .map(|(i, e)| {
            format!(
                "{}. [{}] {} (significance: {:?})",
                i + 1,
                format!("{:?}", e.event_type).to_lowercase(),
                e.description,
                e.significance,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = match perspective {
        Perspective::Dm => prompts::session_summary_dm_system().to_string(),
        Perspective::Player => prompts::session_summary_player_system().to_string(),
    };

    let completion = state
        .llm
        .complete(CompletionRequest {
            task: LlmTask::SessionSummary,
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: system_prompt,
                },
                Message {
                    role: MessageRole::User,
                    content: format!("Session events:\n\n{event_list}"),
                },
            ],
            model_override: None,
            temperature: Some(0.7),
            max_tokens: Some(1500),
        })
        .await;

    match completion {
        Ok(resp) => (
            StatusCode::OK,
            Json(SessionSummary {
                session_id,
                perspective,
                content: resp.content,
                generated_at: chrono::Utc::now(),
            }),
        )
            .into_response(),
        Err(e) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("LLM unavailable: {e}"),
        ),
    }
}

fn error_response(status: StatusCode, msg: &str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": msg }))).into_response()
}
