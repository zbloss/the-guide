use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;
use guide_core::{AppConfig, Result};

use crate::{
    client::{
        AudioRequest, CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, LlmTask,
        VisionRequest,
    },
    CloudProvider, OllamaProvider,
};

#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    AlwaysLocal,
    LocalWithFallback { fallback_provider: String },
    AlwaysCloud { provider: String },
}

pub struct LlmRouter {
    strategy: RoutingStrategy,
    local: Arc<dyn LlmClient>,
    cloud: Option<Arc<dyn LlmClient>>,
    ocr_uses_cloud: bool,
    story_uses_cloud: bool,
}

impl LlmRouter {
    pub fn new(
        strategy: RoutingStrategy,
        local: Arc<dyn LlmClient>,
        cloud: Option<Arc<dyn LlmClient>>,
        ocr_uses_cloud: bool,
        story_uses_cloud: bool,
    ) -> Self {
        Self { strategy, local, cloud, ocr_uses_cloud, story_uses_cloud }
    }

    pub fn always_local(config: &AppConfig) -> Self {
        let ollama = OllamaProvider::new(
            &config.ollama_base_url,
            &config.default_model,
            &config.ocr_model,
            &config.embedding_model,
            &config.whisper_model,
        );
        Self::new(RoutingStrategy::AlwaysLocal, Arc::new(ollama), None, false, false)
    }

    pub fn with_cloud_fallback(config: &AppConfig) -> Option<Self> {
        let api_key = config.cloud_api_key.as_deref()?;
        let provider_name = config.cloud_fallback.as_deref()?;

        let (base_url, default_model, label) = match provider_name {
            "openai" => (None, "gpt-4o-mini".to_string(), "openai".to_string()),
            "gemini" => (
                Some("https://generativelanguage.googleapis.com/v1beta/openai".to_string()),
                "gemini-2.5-flash-lite".to_string(),
                "gemini".to_string(),
            ),
            unknown => {
                tracing::warn!("Unknown cloud_fallback provider '{unknown}', ignoring");
                return None;
            }
        };

        // Allow explicit cloud_model override from config
        let model = config.cloud_model.clone().unwrap_or(default_model);

        let ollama = OllamaProvider::new(
            &config.ollama_base_url,
            &config.default_model,
            &config.ocr_model,
            &config.embedding_model,
            &config.whisper_model,
        );
        let cloud = CloudProvider::new(api_key, model, base_url, label.clone());

        let ocr_uses_cloud = config.ocr_provider == "cloud";
        let story_uses_cloud = config.story_provider == "cloud";

        Some(Self::new(
            RoutingStrategy::LocalWithFallback { fallback_provider: label },
            Arc::new(ollama),
            Some(Arc::new(cloud)),
            ocr_uses_cloud,
            story_uses_cloud,
        ))
    }

    pub fn from_config(config: &AppConfig) -> Self {
        Self::with_cloud_fallback(config).unwrap_or_else(|| Self::always_local(config))
    }

    fn select_provider(&self, task: &LlmTask) -> Arc<dyn LlmClient> {
        // Embedding and character-sheet OCR always stay local
        match task {
            LlmTask::EmbeddingGeneration | LlmTask::CharacterSheetOcr => {
                return Arc::clone(&self.local);
            }
            // Story extraction routes to cloud if configured
            LlmTask::StoryExtraction if self.story_uses_cloud => {
                return self.cloud.as_ref().map(Arc::clone).unwrap_or_else(|| Arc::clone(&self.local));
            }
            _ => {}
        }

        match &self.strategy {
            RoutingStrategy::AlwaysLocal => Arc::clone(&self.local),
            RoutingStrategy::LocalWithFallback { .. } => Arc::clone(&self.local),
            RoutingStrategy::AlwaysCloud { .. } => {
                self.cloud.as_ref().map(Arc::clone).unwrap_or_else(|| Arc::clone(&self.local))
            }
        }
    }

    pub async fn route_complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let task = req.task.clone();
        let provider = self.select_provider(&task);
        let result = provider.complete(req.clone()).await;

        match result {
            Ok(resp) => Ok(resp),
            Err(local_err) => {
                if matches!(&self.strategy, RoutingStrategy::LocalWithFallback { .. }) {
                    if let Some(cloud) = &self.cloud {
                        tracing::warn!("Local LLM failed ({local_err}), falling back to cloud");
                        return cloud.complete(req).await;
                    }
                }
                Err(local_err)
            }
        }
    }

    pub async fn route_stream(
        &self,
        req: CompletionRequest,
    ) -> Result<BoxStream<'static, Result<String>>> {
        let task = req.task.clone();
        let provider = self.select_provider(&task);
        provider.complete_stream(req).await
    }
}

#[async_trait]
impl LlmClient for LlmRouter {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        self.route_complete(req).await
    }

    async fn complete_stream(
        &self,
        req: CompletionRequest,
    ) -> Result<BoxStream<'static, Result<String>>> {
        self.route_stream(req).await
    }

    async fn embed(&self, req: EmbeddingRequest) -> Result<Vec<f32>> {
        self.local.embed(req).await
    }

    async fn complete_with_vision(&self, req: VisionRequest) -> Result<CompletionResponse> {
        if self.ocr_uses_cloud {
            if let Some(cloud) = &self.cloud {
                return cloud.complete_with_vision(req).await;
            }
        }
        self.local.complete_with_vision(req).await
    }

    async fn transcribe(&self, req: AudioRequest) -> Result<String> {
        // Transcription is always local — Whisper runs via Ollama.
        self.local.transcribe(req).await
    }

    fn provider_name(&self) -> &str {
        "router"
    }
}
