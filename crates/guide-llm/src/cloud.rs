use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
        CreateEmbeddingRequestArgs, ResponseFormat,
    },
    Client,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use futures::stream::BoxStream;
use tracing::{debug, instrument, warn};

use guide_core::{GuideError, Result};

use crate::client::{
    AudioRequest, CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, MessageRole,
    VisionRequest,
};

// ── Rate-limit retry ──────────────────────────────────────────────────────────

const MAX_RETRIES: u32 = 6;
/// Base delay in milliseconds — doubles each attempt (2s, 4s, 8s, 16s, 32s, 64s).
const BASE_DELAY_MS: u64 = 2_000;
/// Hard cap so we never wait more than 90 seconds between retries.
const MAX_DELAY_MS: u64 = 90_000;

/// Returns true if the error looks like an HTTP 429 / quota-exhausted response.
///
/// Checks the lowercased error string for patterns emitted by:
/// - Gemini: "resource_exhausted", "429", "quota"
/// - OpenAI: "rate_limit_exceeded", "too_many_requests"
fn is_rate_limited(err: &GuideError) -> bool {
    let GuideError::Llm(msg) = err else { return false };
    let m = msg.to_lowercase();
    m.contains("429")
        || m.contains("resource_exhausted")
        || m.contains("rate_limit")
        || m.contains("quota")
        || m.contains("too many requests")
        || m.contains("too_many_requests")
}

/// Runs `f()` and retries with exponential backoff whenever the call is rate-limited.
///
/// Backoff schedule (ms): 2 000 → 4 000 → 8 000 → 16 000 → 32 000 → 64 000
/// Capped at [`MAX_DELAY_MS`]. After [`MAX_RETRIES`] rate-limit errors the last
/// error is returned to the caller.
async fn retry_with_backoff<T, F, Fut>(mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(err) if attempt < MAX_RETRIES && is_rate_limited(&err) => {
                attempt += 1;
                let delay_ms = (BASE_DELAY_MS * (1u64 << (attempt - 1))).min(MAX_DELAY_MS);
                warn!(
                    attempt,
                    delay_ms,
                    "Cloud provider rate-limited (429); retrying after backoff"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            Err(err) => return Err(err),
        }
    }
}

// ── CloudProvider ─────────────────────────────────────────────────────────────

pub struct CloudProvider {
    client: Client<OpenAIConfig>,
    model: String,
    provider_label: String,
}

impl CloudProvider {
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

    fn build_chat_messages(
        messages: &[crate::client::Message],
    ) -> Result<Vec<ChatCompletionRequestMessage>> {
        messages
            .iter()
            .map(|msg| {
                let m = match msg.role {
                    MessageRole::System => ChatCompletionRequestMessage::System(
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(msg.content.clone())
                            .build()
                            .map_err(|e| GuideError::Llm(e.to_string()))?,
                    ),
                    MessageRole::User | MessageRole::Assistant => {
                        ChatCompletionRequestMessage::User(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(msg.content.clone())
                                .build()
                                .map_err(|e| GuideError::Llm(e.to_string()))?,
                        )
                    }
                };
                Ok(m)
            })
            .collect()
    }
}

#[async_trait]
impl LlmClient for CloudProvider {
    #[instrument(skip(self, req), fields(provider = %self.provider_label))]
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let model = req.model_override.as_deref().unwrap_or(&self.model).to_string();
        debug!(?model, "CloudProvider::complete");

        // Build messages once — cheap and infallible once built.
        let messages = Self::build_chat_messages(&req.messages)?;

        let client = &self.client;
        let provider_label = self.provider_label.clone();

        retry_with_backoff(|| {
            let model = model.clone();
            let messages = messages.clone();
            let provider_label = provider_label.clone();
            async move {
                let mut builder = CreateChatCompletionRequestArgs::default();
                builder.model(model.clone()).messages(messages);
                if let Some(temp) = req.temperature {
                    builder.temperature(temp);
                }
                if let Some(max) = req.max_tokens {
                    builder.max_tokens(max as u16);
                }
                if req.json_mode {
                    builder.response_format(ResponseFormat::JsonObject);
                }
                let request = builder.build().map_err(|e| GuideError::Llm(e.to_string()))?;

                let response = client
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
                    .map(|u| (u.prompt_tokens, u.completion_tokens))
                    .unwrap_or((0, 0));

                Ok(CompletionResponse {
                    content,
                    model,
                    provider: provider_label,
                    prompt_tokens,
                    completion_tokens,
                })
            }
        })
        .await
    }

    async fn complete_stream(
        &self,
        req: CompletionRequest,
    ) -> Result<BoxStream<'static, Result<String>>> {
        use async_openai::types::CreateChatCompletionStreamResponse;
        use futures::StreamExt;

        let model = req.model_override.as_deref().unwrap_or(&self.model).to_string();
        let messages = Self::build_chat_messages(&req.messages)?;

        // For streaming we retry the stream *creation* (before any data flows).
        // Once the stream is open we cannot roll it back, so we don't retry mid-stream.
        let client = &self.client;
        let stream = retry_with_backoff(|| {
            let model = model.clone();
            let messages = messages.clone();
            let temperature = req.temperature;
            async move {
                let mut builder = CreateChatCompletionRequestArgs::default();
                builder.model(model).messages(messages);
                if let Some(temp) = temperature {
                    builder.temperature(temp);
                }
                let request =
                    builder.build().map_err(|e| GuideError::Llm(e.to_string()))?;

                client
                    .chat()
                    .create_stream(request)
                    .await
                    .map_err(|e| GuideError::Llm(e.to_string()))
            }
        })
        .await?;

        let mapped =
            stream.map(|result: std::result::Result<CreateChatCompletionStreamResponse, _>| {
                match result {
                    Ok(resp) => {
                        let token = resp
                            .choices
                            .into_iter()
                            .next()
                            .and_then(|c| c.delta.content)
                            .unwrap_or_default();
                        Ok(token)
                    }
                    Err(e) => Err(GuideError::Llm(e.to_string())),
                }
            });

        Ok(Box::pin(mapped))
    }

    async fn embed(&self, req: EmbeddingRequest) -> Result<Vec<f32>> {
        let model = req.model_override.unwrap_or_else(|| self.model.clone());
        let client = &self.client;

        retry_with_backoff(|| {
            let model = model.clone();
            let text = req.text.clone();
            async move {
                let request = CreateEmbeddingRequestArgs::default()
                    .model(&model)
                    .input(text)
                    .build()
                    .map_err(|e| GuideError::Llm(e.to_string()))?;

                let response = client
                    .embeddings()
                    .create(request)
                    .await
                    .map_err(|e| GuideError::Llm(e.to_string()))?;

                response
                    .data
                    .into_iter()
                    .next()
                    .ok_or_else(|| GuideError::Llm("No embedding returned".into()))
                    .map(|e| e.embedding)
            }
        })
        .await
    }

    async fn complete_with_vision(&self, req: VisionRequest) -> Result<CompletionResponse> {
        let model = req.model_override.as_deref().unwrap_or(&self.model).to_string();

        // Base64-encode the image once before entering the retry loop.
        let encoded = STANDARD.encode(&req.image_bytes);
        let data_url = format!("data:{};base64,{}", req.image_mime_type, encoded);

        let client = &self.client;
        let provider_label = self.provider_label.clone();

        retry_with_backoff(|| {
            let model = model.clone();
            let data_url = data_url.clone();
            let prompt = req.prompt.clone();
            let provider_label = provider_label.clone();
            async move {
                let content_parts = serde_json::json!([
                    { "type": "image_url", "image_url": { "url": data_url } },
                    { "type": "text", "text": prompt }
                ]);

                let mut builder = CreateChatCompletionRequestArgs::default();
                builder.model(model.clone()).messages(vec![
                    ChatCompletionRequestMessage::User(
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(content_parts.to_string())
                            .build()
                            .map_err(|e| GuideError::Llm(e.to_string()))?,
                    ),
                ]);
                if let Some(temp) = req.temperature {
                    builder.temperature(temp);
                }
                if let Some(max) = req.max_tokens {
                    builder.max_tokens(max as u16);
                }
                if let Some(top_p) = req.top_p {
                    builder.top_p(top_p);
                }
                let request =
                    builder.build().map_err(|e| GuideError::Llm(e.to_string()))?;

                let response = client
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
                    .map(|u| (u.prompt_tokens, u.completion_tokens))
                    .unwrap_or((0, 0));

                Ok(CompletionResponse {
                    content,
                    model,
                    provider: provider_label,
                    prompt_tokens,
                    completion_tokens,
                })
            }
        })
        .await
    }

    async fn transcribe(&self, _req: AudioRequest) -> Result<String> {
        Err(GuideError::Llm(
            "audio transcription not supported for cloud provider".to_string(),
        ))
    }

    fn provider_name(&self) -> &str {
        &self.provider_label
    }
}
