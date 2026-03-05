use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use guide_core::Result;

// ── Task classification ───────────────────────────────────────────────────────

/// Describes the intent of an LLM call so the router can select the right
/// provider and model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmTask {
    /// PDF page image → structured text (GLM-OCR via Ollama)
    OcrExtraction,
    /// General vision understanding (maps, illustrations)
    VisionDescription,
    /// Dense vector embedding for Qdrant upsert/search
    EmbeddingGeneration,
    /// DM / player Q&A over campaign lore (RAG)
    CampaignAssistant,
    /// Backstory hook extraction
    BackstoryAnalysis,
    /// Narrative encounter generation
    EncounterGeneration,
    /// Session recap generation
    SessionSummary,
    /// Catch-all for unclassified tasks
    General,
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub task: LlmTask,
    pub messages: Vec<Message>,
    pub model_override: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model_override: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VisionRequest {
    pub task: LlmTask,
    pub prompt: String,
    /// Raw image bytes — the provider will base64-encode them
    pub image_bytes: Vec<u8>,
    pub image_mime_type: String,
    pub model_override: Option<String>,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;
    async fn embed(&self, req: EmbeddingRequest) -> Result<Vec<f32>>;
    async fn complete_with_vision(&self, req: VisionRequest) -> Result<CompletionResponse>;
    fn provider_name(&self) -> &str;
}
