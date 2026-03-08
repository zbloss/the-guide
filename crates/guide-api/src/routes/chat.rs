use axum::{
    extract::{Path, State},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::post,
    Json, Router,
};
use futures::StreamExt;
use guide_core::{models::Perspective, GuideError};

const MAX_MESSAGE_LEN: usize = 4000;
use serde::Deserialize;
use std::convert::Infallible;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/campaigns/{campaign_id}/chat", post(chat))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ChatRequest {
    pub message: String,
    pub perspective: Option<String>,
    pub context_limit: Option<usize>,
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

    let perspective = match req.perspective.as_deref() {
        Some("player") => Perspective::Player,
        _ => Perspective::Dm,
    };
    let player_visible_only = perspective == Perspective::Player;
    let context_limit = req.context_limit.unwrap_or(5);

    // Retrieve RAG context
    let chunks = query_indexes(
        &req.message,
        Some(campaign_id),
        player_visible_only,
        state.llm.as_ref(),
        &state.config,
        state.qdrant.as_deref(),
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

    let system_prompt = match perspective {
        Perspective::Dm => prompts::campaign_assistant_dm_system(&context),
        Perspective::Player => prompts::campaign_assistant_player_system(&context),
    };

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
    };

    let stream = state.llm.complete_stream(llm_req).await?;

    let sse_stream = stream
        .filter(|result| {
            futures::future::ready(match result {
                Ok(token) => !token.is_empty(),
                Err(_) => true,
            })
        })
        .map(|result| {
            let event = match result {
                Ok(token) => Event::default().event("token").data(token),
                Err(e) => Event::default().event("error").data(e.to_string()),
            };
            Ok::<Event, Infallible>(event)
        })
        .chain(futures::stream::once(async {
            Ok::<Event, Infallible>(Event::default().event("done").data("[DONE]"))
        }));

    Ok(Sse::new(sse_stream))
}
