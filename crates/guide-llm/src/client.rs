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
    AudioTranscription,
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

#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model_override: Option<String>,
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

/// Audio bytes submitted for speech-to-text transcription.
#[derive(Debug, Clone)]
pub struct AudioRequest {
    /// Raw audio bytes (e.g. a WebM or WAV recording).
    pub audio_bytes: Vec<u8>,
    /// MIME type of the audio data, e.g. `"audio/webm"`.
    pub mime_type: String,
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
    /// Transcribe audio bytes to text using a Whisper-compatible model.
    async fn transcribe(&self, req: AudioRequest) -> Result<String>;
    fn provider_name(&self) -> &str;
}
