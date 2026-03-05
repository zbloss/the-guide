//! Cloud provider implementations using the OpenAI-compatible API.
//! OpenAI and Gemini both support the OpenAI wire format — the same
//! `OllamaProvider` code works with a different base URL + real API key.
//!
//! For Anthropic, the native API format differs from OpenAI's, so a separate
//! implementation is needed (Phase 5 follow-up; requires `anthropic-sdk-rust`).

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
        CreateEmbeddingRequestArgs,
    },
    Client,
};
use async_trait::async_trait;
use tracing::{debug, instrument};

use guide_core::{GuideError, Result};

use crate::client::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, MessageRole, VisionRequest,
};

/// An LLM provider using the native OpenAI API (api.openai.com).
/// Also works for Gemini via its OpenAI-compatible endpoint.
pub struct OpenAICloudProvider {
    client: Client<OpenAIConfig>,
    model: String,
    provider_label: String,
}

impl OpenAICloudProvider {
    /// `base_url`: None → uses api.openai.com. Some(url) → use that endpoint (e.g. Gemini).
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: Option<String>,
        provider_label: impl Into<String>,
    ) -> Self {
        let mut config = OpenAIConfig::new().with_api_key(api_key.into());
        if let Some(url) = base_url {
            config = config.with_api_base(url);
        }
        Self {
            client: Client::with_config(config),
            model: model.into(),
            provider_label: provider_label.into(),
        }
    }
}

#[async_trait]
impl LlmClient for OpenAICloudProvider {
    #[instrument(skip(self, req), fields(provider = %self.provider_label))]
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let model = req.model_override.as_deref().unwrap_or(&self.model).to_string();
        debug!(?model, "OpenAICloudProvider::complete");

        let mut messages: Vec<ChatCompletionRequestMessage> = Vec::new();
        for msg in &req.messages {
            let m = match msg.role {
                MessageRole::System => ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(msg.content.clone())
                        .build()
                        .map_err(|e| GuideError::Llm(e.to_string()))?,
                ),
                MessageRole::User | MessageRole::Assistant => ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(msg.content.clone())
                        .build()
                        .map_err(|e| GuideError::Llm(e.to_string()))?,
                ),
            };
            messages.push(m);
        }

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder.model(model.clone()).messages(messages);
        if let Some(temp) = req.temperature {
            builder.temperature(temp);
        }
        if let Some(max) = req.max_tokens {
            builder.max_tokens(max as u16);
        }
        let request = builder.build().map_err(|e| GuideError::Llm(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| GuideError::Llm(e.to_string()))?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| GuideError::Llm("No choices returned".into()))?;

        let content = choice.message.content.unwrap_or_default();
        let (prompt_tokens, completion_tokens) = response
            .usage
            .map(|u| (u.prompt_tokens as u32, u.completion_tokens as u32))
            .unwrap_or((0, 0));

        Ok(CompletionResponse {
            content,
            model,
            provider: self.provider_label.clone(),
            prompt_tokens,
            completion_tokens,
        })
    }

    async fn embed(&self, req: EmbeddingRequest) -> Result<Vec<f32>> {
        let model = req.model_override.unwrap_or_else(|| "text-embedding-3-small".to_string());

        let request = CreateEmbeddingRequestArgs::default()
            .model(&model)
            .input(req.text)
            .build()
            .map_err(|e| GuideError::Llm(e.to_string()))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| GuideError::Llm(e.to_string()))?;

        let embedding = response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| GuideError::Llm("No embedding returned".into()))?
            .embedding;

        Ok(embedding)
    }

    async fn complete_with_vision(&self, req: VisionRequest) -> Result<CompletionResponse> {
        // Delegate to the shared vision path — same wire format as Ollama
        use base64::{engine::general_purpose::STANDARD, Engine};
        let model = req.model_override.as_deref().unwrap_or(&self.model).to_string();

        let encoded = STANDARD.encode(&req.image_bytes);
        let data_url = format!("data:{};base64,{}", req.image_mime_type, encoded);
        let content_json = serde_json::json!([
            { "type": "image_url", "image_url": { "url": data_url } },
            { "type": "text",      "text": req.prompt }
        ]);

        let request = CreateChatCompletionRequestArgs::default()
            .model(model.clone())
            .messages(vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(content_json.to_string())
                    .build()
                    .map_err(|e| GuideError::Llm(e.to_string()))?,
            )])
            .build()
            .map_err(|e| GuideError::Llm(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| GuideError::Llm(e.to_string()))?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| GuideError::Llm("No choices".into()))?;

        let content = choice.message.content.unwrap_or_default();
        let (prompt_tokens, completion_tokens) = response
            .usage
            .map(|u| (u.prompt_tokens as u32, u.completion_tokens as u32))
            .unwrap_or((0, 0));

        Ok(CompletionResponse {
            content,
            model,
            provider: self.provider_label.clone(),
            prompt_tokens,
            completion_tokens,
        })
    }

    fn provider_name(&self) -> &str {
        &self.provider_label
    }
}
