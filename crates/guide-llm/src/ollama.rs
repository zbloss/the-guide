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
use futures::stream::BoxStream;
use tracing::{debug, instrument};

use guide_core::{GuideError, Result};

use crate::client::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, LlmTask, MessageRole,
    VisionRequest,
};

/// Strip `<think>…</think>` blocks that Qwen3 thinking models embed in content.
/// Returns everything after the last `</think>` tag, trimmed.
fn strip_think_tags(content: &str) -> String {
    match content.rfind("</think>") {
        Some(end) => content[end + "</think>".len()..].trim().to_string(),
        None => content.trim().to_string(),
    }
}

pub struct OllamaProvider {
    client: Client<OpenAIConfig>,
    default_model: String,
    ocr_model: String,
    embedding_model: String,
}

impl OllamaProvider {
    pub fn new(
        base_url: impl Into<String>,
        default_model: impl Into<String>,
        ocr_model: impl Into<String>,
        embedding_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into();
        let config = OpenAIConfig::new()
            .with_api_base(base_url)
            .with_api_key("ollama");

        Self {
            client: Client::with_config(config),
            default_model: default_model.into(),
            ocr_model: ocr_model.into(),
            embedding_model: embedding_model.into(),
        }
    }

    fn model_for_task(&self, task: &LlmTask, override_model: Option<&str>) -> String {
        if let Some(m) = override_model {
            return m.to_string();
        }
        match task {
            LlmTask::OcrExtraction => self.ocr_model.clone(),
            LlmTask::EmbeddingGeneration => self.embedding_model.clone(),
            _ => self.default_model.clone(),
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
impl LlmClient for OllamaProvider {
    #[instrument(skip(self, req), fields(task = ?req.task))]
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let model = self.model_for_task(&req.task, req.model_override.as_deref());
        debug!(?model, "OllamaProvider::complete");

        let messages = Self::build_chat_messages(&req.messages)?;

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

        let raw = choice.message.content.unwrap_or_default();
        // Qwen3 thinking models embed reasoning in <think>…</think> inside content.
        // Strip those blocks so callers only see the actual response.
        let content = strip_think_tags(&raw);
        debug!(
            raw_len = raw.len(),
            content_len = content.len(),
            has_think = raw.contains("<think>"),
            "OllamaProvider::complete response"
        );
        let (prompt_tokens, completion_tokens) = response
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        Ok(CompletionResponse {
            content,
            model,
            provider: "ollama".into(),
            prompt_tokens,
            completion_tokens,
        })
    }

    async fn complete_stream(
        &self,
        req: CompletionRequest,
    ) -> Result<BoxStream<'static, Result<String>>> {
        use async_openai::types::CreateChatCompletionStreamResponse;
        use futures::StreamExt;

        let model = self.model_for_task(&req.task, req.model_override.as_deref());
        let messages = Self::build_chat_messages(&req.messages)?;

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder.model(model).messages(messages);
        if let Some(temp) = req.temperature {
            builder.temperature(temp);
        }
        let request = builder.build().map_err(|e| GuideError::Llm(e.to_string()))?;

        let stream = self
            .client
            .chat()
            .create_stream(request)
            .await
            .map_err(|e| GuideError::Llm(e.to_string()))?;

        // State: (in_think: bool, past_think: bool, buf: String)
        // We accumulate content until we know whether we're in a <think> block.
        // Once past the </think> tag we emit normally.
        let mapped = stream
            .map(|result: std::result::Result<CreateChatCompletionStreamResponse, _>| {
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
            })
            .scan(
                (false, false, String::new()),
                |state: &mut (bool, bool, String), result| {
                    let (in_think, past_think, buf) = state;
                    let out = match result {
                        Err(e) => Some(Err(e)),
                        Ok(token) => {
                            if *past_think {
                                Some(Ok(token))
                            } else {
                                buf.push_str(&token);
                                if *in_think {
                                    if let Some(end) = buf.find("</think>") {
                                        let after =
                                            buf[end + "</think>".len()..].trim().to_string();
                                        buf.clear();
                                        *in_think = false;
                                        *past_think = true;
                                        Some(Ok(after))
                                    } else {
                                        buf.clear(); // discard think content
                                        Some(Ok(String::new()))
                                    }
                                } else if buf.contains("<think>") {
                                    *in_think = true;
                                    buf.clear();
                                    Some(Ok(String::new()))
                                } else if buf.len() > 32 || (!buf.is_empty() && !buf.contains('<')) {
                                    // No think tag coming — emit what we buffered
                                    *past_think = true;
                                    let out = buf.clone();
                                    buf.clear();
                                    Some(Ok(out))
                                } else {
                                    // Still uncertain, keep buffering
                                    Some(Ok(String::new()))
                                }
                            }
                        }
                    };
                    futures::future::ready(out)
                },
            )
            .filter(|result| {
                futures::future::ready(match result {
                    Ok(token) => !token.is_empty(),
                    Err(_) => true,
                })
            });

        Ok(Box::pin(mapped))
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
                    image_url: ImageUrl {
                        url: data_url,
                        detail: None,
                    },
                },
            ),
            ChatCompletionRequestUserMessageContentPart::Text(
                ChatCompletionRequestMessageContentPartText {
                    text: req.prompt.clone(),
                },
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

        let content_str = choice.message.content.unwrap_or_default();
        let (prompt_tokens, completion_tokens) = response
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        Ok(CompletionResponse {
            content: content_str,
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
