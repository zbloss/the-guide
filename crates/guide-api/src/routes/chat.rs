use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use guide_core::GuideError;

const MAX_MESSAGE_LEN: usize = 4000;
use serde::Deserialize;
use std::convert::Infallible;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/campaigns/{campaign_id}/chat", post(chat))
        .route("/campaigns/{campaign_id}/chat/history", get(chat_history))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ChatRequest {
    pub message: String,
    pub context_limit: Option<usize>,
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ChatHistoryQuery {
    pub limit: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/chat",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Chat response (SSE stream)", body = String)
    )
)]
async fn chat(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<ChatRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_llm::{prompts, CompletionRequest, LlmTask, Message, MessageRole};
    use guide_pdf::pipeline::query_indexes;

    if req.message.is_empty() {
        return Err(GuideError::InvalidInput("Message cannot be empty".into()).into());
    }
    if req.message.len() > MAX_MESSAGE_LEN {
        return Err(GuideError::InvalidInput(
            format!("Message exceeds maximum length of {MAX_MESSAGE_LEN} characters"),
        )
        .into());
    }

    let player_visible_only = false;
    let context_limit = req.context_limit.unwrap_or(5);
    let perspective_str = "dm".to_string();
    let user_message = req.message.clone();

    // Retrieve RAG context
    let chunks = query_indexes(
        &req.message,
        Some(campaign_id),
        player_visible_only,
        state.llm.as_ref(),
        &state.db,
    )
    .await
    .unwrap_or_default();

    let context = chunks
        .iter()
        .take(context_limit)
        .map(|c| {
            if c.section_path.is_empty() {
                c.content.clone()
            } else {
                format!("[{}]\n{}", c.section_path, c.content)
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let system_prompt = prompts::campaign_assistant_dm_system(&context);

    let llm_req = CompletionRequest {
        task: LlmTask::CampaignAssistant,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: system_prompt,
            },
            Message {
                role: MessageRole::User,
                content: req.message,
            },
        ],
        model_override: None,
        temperature: Some(0.7),
        max_tokens: Some(2048),
        json_mode: false,
    };

    let stream = state.llm.complete_stream(llm_req).await?;

    // Use a channel to tap the stream so we can collect the full assistant response
    // for persistence while forwarding tokens to the SSE client.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let db = state.db.clone();

    // Spawn a task to persist user + assistant messages once streaming completes
    tokio::spawn(async move {
        use guide_db::chat::ChatRepository;
        let mut full_response = String::new();
        while let Some(token) = rx.recv().await {
            full_response.push_str(&token);
        }
        if !full_response.is_empty() {
            let chat_repo = ChatRepository::new(&db);
            let _ = chat_repo
                .append(campaign_id, "user", &user_message, &perspective_str)
                .await;
            let _ = chat_repo
                .append(campaign_id, "assistant", &full_response, &perspective_str)
                .await;
        }
    });

    let sse_stream = stream
        .filter(|result| {
            futures::future::ready(match result {
                Ok(token) => !token.is_empty(),
                Err(_) => true,
            })
        })
        .map(move |result| {
            let event = match result {
                Ok(token) => {
                    let _ = tx.send(token.clone());
                    Event::default().event("token").data(token)
                }
                Err(e) => Event::default().event("error").data(e.to_string()),
            };
            Ok::<Event, Infallible>(event)
        })
        .chain(futures::stream::once(async {
            Ok::<Event, Infallible>(Event::default().event("done").data("[DONE]"))
        }));

    Ok(Sse::new(sse_stream))
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/chat/history",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ChatHistoryQuery
    ),
    responses(
        (status = 200, description = "Chat history for campaign", body = [guide_core::models::ChatMessage])
    )
)]
async fn chat_history(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Query(q): Query<ChatHistoryQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use guide_db::chat::ChatRepository;
    let limit = q.limit.unwrap_or(50);
    let repo = ChatRepository::new(&state.db);
    let messages = repo.list(campaign_id, limit).await?;
    Ok(Json(messages))
}
