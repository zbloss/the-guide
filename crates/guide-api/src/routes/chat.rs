use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use guide_core::models::{Perspective, RankedChunk};
use guide_llm::client::{CompletionRequest, EmbeddingRequest, LlmTask, Message, MessageRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/campaigns/{campaign_id}/chat", post(chat))
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    /// Who is asking — controls spoiler filtering
    pub perspective: Option<Perspective>,
    /// Max number of Qdrant lore chunks to inject as context
    pub context_limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub answer: String,
    pub context_chunks_used: usize,
    pub model: String,
    pub provider: String,
}

const MAX_MESSAGE_CHARS: usize = 4_000;

async fn chat(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    if req.message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "message must not be empty" })),
        )
            .into_response();
    }
    if req.message.len() > MAX_MESSAGE_CHARS {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": format!("message exceeds maximum length of {MAX_MESSAGE_CHARS} characters")
            })),
        )
            .into_response();
    }

    let perspective = req.perspective.unwrap_or(Perspective::Dm);
    let context_limit = req.context_limit.unwrap_or(5);

    // ── Step 1: Embed the query ───────────────────────────────────────────────
    let query_embedding = match state
        .llm
        .embed(EmbeddingRequest {
            text: req.message.clone(),
            model_override: None,
        })
        .await
    {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("Embedding failed (proceeding without RAG context): {e}");
            None
        }
    };

    // ── Step 2: Retrieve lore from both Qdrant collections ───────────────────
    let lore_chunks = if let (Some(embedding), Some(qdrant)) = (query_embedding, &state.qdrant) {
        retrieve_lore(
            qdrant,
            &campaign_id.to_string(),
            embedding,
            &perspective,
            context_limit,
        )
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let context_chunks_used = lore_chunks.len();

    // ── Step 3: Build prompt with injected context ────────────────────────────
    let system_prompt = build_system_prompt(&perspective, &lore_chunks);

    let messages = vec![
        Message {
            role: MessageRole::System,
            content: system_prompt,
        },
        Message {
            role: MessageRole::User,
            content: req.message,
        },
    ];

    // ── Step 4: LLM completion ────────────────────────────────────────────────
    match state
        .llm
        .complete(CompletionRequest {
            task: LlmTask::CampaignAssistant,
            messages,
            model_override: None,
            temperature: Some(0.7),
            max_tokens: Some(1024),
        })
        .await
    {
        Ok(resp) => (
            StatusCode::OK,
            Json(ChatResponse {
                answer: resp.content,
                context_chunks_used,
                model: resp.model,
                provider: resp.provider,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": format!("LLM unavailable: {}", e)
            })),
        )
            .into_response(),
    }
}

fn build_system_prompt(perspective: &Perspective, lore_chunks: &[RankedChunk]) -> String {
    let role_instruction = match perspective {
        Perspective::Dm => {
            "You are The Guide, an AI assistant for a Dungeon Master running a D&D campaign. \
             You have access to full campaign lore including DM-only information. \
             Be concise, accurate, and helpful."
        }
        Perspective::Player => {
            "You are The Guide, an AI assistant for players in a D&D campaign. \
             You MUST NOT reveal DM-only information, secret plot points, or unrevealed lore. \
             Only share what the players have discovered in-game. \
             If you are unsure whether something is player-visible, do not share it."
        }
    };

    if lore_chunks.is_empty() {
        return format!(
            "{role_instruction}\n\n\
             No campaign-specific lore is available yet. \
             Answer using your general D&D knowledge where appropriate."
        );
    }

    let context_block = lore_chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let attribution = build_attribution(chunk);
            format!("[{}] {}\n{}", i + 1, attribution, chunk.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "{role_instruction}\n\n\
         ## Campaign Context\n\
         The following lore has been retrieved from the campaign knowledge base. \
         Use it to answer accurately:\n\n\
         {context_block}"
    )
}

fn build_attribution(chunk: &RankedChunk) -> String {
    let mut parts = Vec::new();
    if !chunk.doc_title.is_empty() {
        parts.push(chunk.doc_title.clone());
    }
    if !chunk.section_path.is_empty() {
        parts.push(chunk.section_path.clone());
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("({})", parts.join(" — "))
}

/// Query both campaign and global Qdrant collections, merge by score, return top N.
async fn retrieve_lore(
    qdrant: &qdrant_client::Qdrant,
    campaign_id: &str,
    embedding: Vec<f32>,
    perspective: &Perspective,
    limit: usize,
) -> guide_core::Result<Vec<RankedChunk>> {
    use guide_db::qdrant::{search_campaign_lore, search_global_rules};

    let player_visible_only = matches!(perspective, Perspective::Player);

    // Query campaign-specific collection
    let campaign_results = search_campaign_lore(
        qdrant,
        campaign_id,
        embedding.clone(),
        limit,
        player_visible_only,
    )
    .await
    .unwrap_or_default();

    // Query global rulebook collection (never spoiler-filtered)
    let global_results = search_global_rules(qdrant, embedding, limit).await.unwrap_or_default();

    // Merge by score, take top N
    let mut all = [campaign_results, global_results].concat();
    all.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all.dedup_by(|a, b| a.content == b.content);
    Ok(all.into_iter().take(limit).collect())
}
