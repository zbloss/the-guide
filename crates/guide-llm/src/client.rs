use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use guide_core::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmTask {
    OcrExtraction,
    EmbeddingGeneration,
    CampaignAssistant,
    BackstoryAnalysis,
    EncounterGeneration,
    SessionSummary,
    SessionRecap,
    StorySoFar,
    StoryAhead,
    CharacterRoadmap,
    CharacterSheetParse,
    CharacterSheetOcr,
    StoryExtraction,
    General,
}

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
    pub json_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Signals to the embedding backend whether the text is a search query
/// or a document being indexed. Bi-encoder models like EmbeddingGemma use
/// different task prefixes for the two roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbeddingHint {
    Query,
    #[default]
    Document,
}

#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model_override: Option<String>,
    /// Whether this text is a search query or a document being indexed.
    pub hint: EmbeddingHint,
}

#[derive(Debug, Clone)]
pub struct VisionRequest {
    pub task: LlmTask,
    pub prompt: String,
    pub image_bytes: Vec<u8>,
    pub image_mime_type: String,
    pub model_override: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    /// Note: top_k = 50 and repetition_penalty = 1.1 from GLM-OCR reference
    /// cannot be forwarded via async-openai's OpenAI-compatible builder.
    pub top_p: Option<f32>,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;
    async fn complete_stream(
        &self,
        req: CompletionRequest,
    ) -> Result<BoxStream<'static, Result<String>>>;
    async fn embed(&self, req: EmbeddingRequest) -> Result<Vec<f32>>;
    async fn complete_with_vision(&self, req: VisionRequest) -> Result<CompletionResponse>;
    fn provider_name(&self) -> &str;
}
