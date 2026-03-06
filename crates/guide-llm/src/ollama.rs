use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImage,
        ChatCompletionRequestMessageContentPartText, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
        ChatCompletionRequestUserMessageContentPart, CreateChatCompletionRequestArgs,
        CreateEmbeddingRequestArgs, ImageUrl,
    },
    Client,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use tracing::{debug, instrument};

use guide_core::{GuideError, Result};

use crate::client::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, LlmTask, MessageRole,
    VisionRequest,
};

pub struct OllamaProvider {
    client: Client<OpenAIConfig>,
    base_url: String,
    /// Default chat model (e.g. "llama3.2")
    default_model: String,
    /// OCR model (e.g. "glm-ocr")
    ocr_model: String,
    /// Vision model (e.g. "llama3.2-vision")
    vision_model: String,
    /// Embedding model (e.g. "nomic-embed-text")
    embedding_model: String,
}

impl OllamaProvider {
    pub fn new(
        base_url: impl Into<String>,
        default_model: impl Into<String>,
        ocr_model: impl Into<String>,
        vision_model: impl Into<String>,
        embedding_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into();
        let config = OpenAIConfig::new()
            .with_api_base(base_url.clone())
            .with_api_key("ollama"); // Ollama ignores the key but async-openai requires one

        Self {
            client: Client::with_config(config),
            base_url,
            default_model: default_model.into(),
            ocr_model: ocr_model.into(),
            vision_model: vision_model.into(),
            embedding_model: embedding_model.into(),
        }
    }

    fn model_for_task(&self, task: &LlmTask, override_model: Option<&str>) -> String {
        if let Some(m) = override_model {
            return m.to_string();
        }
        match task {
            LlmTask::OcrExtraction => self.ocr_model.clone(),
            LlmTask::VisionDescription => self.vision_model.clone(),
            LlmTask::EmbeddingGeneration => self.embedding_model.clone(),
            _ => self.default_model.clone(),
        }
    }
}

#[async_trait]
impl LlmClient for OllamaProvider {
    #[instrument(skip(self, req), fields(task = ?req.task))]
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let model = self.model_for_task(&req.task, req.model_override.as_deref());
        debug!(?model, "OllamaProvider::complete");

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
        let request = builder
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
            .ok_or_else(|| GuideError::Llm("No choices returned".into()))?;

        let content = choice.message.content.unwrap_or_default();

        let (prompt_tokens, completion_tokens) = response
            .usage
            .map(|u| (u.prompt_tokens as u32, u.completion_tokens as u32))
            .unwrap_or((0, 0));

        Ok(CompletionResponse {
            content,
            model,
            provider: "ollama".into(),
            prompt_tokens,
            completion_tokens,
        })
    }

    #[instrument(skip(self, req))]
    async fn embed(&self, req: EmbeddingRequest) -> Result<Vec<f32>> {
        let model = req
            .model_override
            .unwrap_or_else(|| self.embedding_model.clone());
        debug!(?model, "OllamaProvider::embed");

        let request = CreateEmbeddingRequestArgs::default()
            .model(model)
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

    #[instrument(skip(self, req), fields(task = ?req.task))]
    async fn complete_with_vision(&self, req: VisionRequest) -> Result<CompletionResponse> {
        let model = self.model_for_task(&req.task, req.model_override.as_deref());
        debug!(?model, "OllamaProvider::complete_with_vision");

        let encoded = STANDARD.encode(&req.image_bytes);
        let data_url = format!("data:{};base64,{}", req.image_mime_type, encoded);

        let content = ChatCompletionRequestUserMessageContent::Array(vec![
            ChatCompletionRequestUserMessageContentPart::ImageUrl(
                ChatCompletionRequestMessageContentPartImage {
                    image_url: ImageUrl { url: data_url, detail: None },
                },
            ),
            ChatCompletionRequestUserMessageContentPart::Text(
                ChatCompletionRequestMessageContentPartText { text: req.prompt.clone() },
            ),
        ]);

        let request = CreateChatCompletionRequestArgs::default()
            .model(model.clone())
            .messages(vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(content)
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
            .ok_or_else(|| GuideError::Llm("No choices returned".into()))?;

        let content = choice.message.content.unwrap_or_default();
        let (prompt_tokens, completion_tokens) = response
            .usage
            .map(|u| (u.prompt_tokens as u32, u.completion_tokens as u32))
            .unwrap_or((0, 0));

        Ok(CompletionResponse {
            content,
            model,
            provider: "ollama".into(),
            prompt_tokens,
            completion_tokens,
        })
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }
}
