pub mod client;
pub mod cloud;
pub mod ollama;
pub mod prompts;
pub mod router;

pub use client::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, LlmTask, Message,
    MessageRole, VisionRequest,
};
pub use cloud::CloudProvider;
pub use ollama::OllamaProvider;
pub use router::{LlmRouter, RoutingStrategy};
