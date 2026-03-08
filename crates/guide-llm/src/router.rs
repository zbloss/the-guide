use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;
use guide_core::{AppConfig, Result};

use crate::{
    client::{CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, LlmTask, VisionRequest},
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
}

impl LlmRouter {
    pub fn new(
        strategy: RoutingStrategy,
        local: Arc<dyn LlmClient>,
        cloud: Option<Arc<dyn LlmClient>>,
    ) -> Self {
        Self { strategy, local, cloud }
    }

    pub fn always_local(config: &AppConfig) -> Self {
        let ollama = OllamaProvider::new(
            &config.ollama_base_url,
            &config.default_model,
            &config.ocr_model,
            &config.embedding_model,
        );
        Self::new(RoutingStrategy::AlwaysLocal, Arc::new(ollama), None)
    }

    pub fn with_cloud_fallback(config: &AppConfig) -> Option<Self> {
        let api_key = config.cloud_api_key.as_deref()?;
        let provider_name = config.cloud_fallback.as_deref()?;

        let (base_url, model, label) = match provider_name {
            "openai" => (None, "gpt-4o".to_string(), "openai".to_string()),
            "gemini" => (
                Some("https://generativelanguage.googleapis.com/v1beta/openai".to_string()),
                "gemini-1.5-flash".to_string(),
                "gemini".to_string(),
            ),
            unknown => {
                tracing::warn!("Unknown cloud_fallback provider '{unknown}', ignoring");
                return None;
            }
        };

        let ollama = OllamaProvider::new(
            &config.ollama_base_url,
            &config.default_model,
            &config.ocr_model,
            &config.embedding_model,
        );
        let cloud = CloudProvider::new(api_key, model, base_url, label.clone());

        Some(Self::new(
            RoutingStrategy::LocalWithFallback { fallback_provider: label },
            Arc::new(ollama),
            Some(Arc::new(cloud)),
        ))
    }

    pub fn from_config(config: &AppConfig) -> Self {
        Self::with_cloud_fallback(config).unwrap_or_else(|| Self::always_local(config))
    }

    fn select_provider(&self, task: &LlmTask) -> Arc<dyn LlmClient> {
        // Specialised local-only tasks always go to Ollama
        match task {
            LlmTask::OcrExtraction | LlmTask::EmbeddingGeneration => {
                return Arc::clone(&self.local);
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
        self.local.complete_with_vision(req).await
    }

    fn provider_name(&self) -> &str {
        "router"
    }
}
