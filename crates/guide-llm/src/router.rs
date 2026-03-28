use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;
use guide_core::{AppConfig, GuideError, Result};
use guide_embed::LocalEmbedder;

use crate::{
    client::{
        CompletionRequest, CompletionResponse, EmbeddingHint, EmbeddingRequest,
        LlmClient, LlmTask, VisionRequest,
    },
    CloudProvider, OllamaProvider,
};

#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    AlwaysLocal,
    AlwaysCloud { provider: String },
}

pub struct LlmRouter {
    strategy: RoutingStrategy,
    local: Arc<dyn LlmClient>,
    cloud: Option<Arc<dyn LlmClient>>,
    ocr_uses_cloud: bool,
    story_uses_cloud: bool,
    /// Set when embedding_provider = "local" and initialisation succeeded.
    local_embedder: Option<Arc<LocalEmbedder>>,
    embedding_uses_cloud: bool,
}

impl LlmRouter {
    pub fn new(
        strategy: RoutingStrategy,
        local: Arc<dyn LlmClient>,
        cloud: Option<Arc<dyn LlmClient>>,
        ocr_uses_cloud: bool,
        story_uses_cloud: bool,
        local_embedder: Option<Arc<LocalEmbedder>>,
        embedding_uses_cloud: bool,
    ) -> Self {
        Self {
            strategy,
            local,
            cloud,
            ocr_uses_cloud,
            story_uses_cloud,
            local_embedder,
            embedding_uses_cloud,
        }
    }

    pub fn always_local(config: &AppConfig) -> Self {
        let ollama = OllamaProvider::new(
            &config.ollama_base_url,
            &config.default_model,
            &config.ocr_model,
            &config.embedding_model,
        );
        Self::new(
            RoutingStrategy::AlwaysLocal,
            Arc::new(ollama),
            None,
            false,
            false,
            None,
            false,
        )
    }

    /// Build the router from config. Routing for OCR and story extraction is
    /// controlled exclusively by `GUIDE__OCR_PROVIDER` and `GUIDE__STORY_PROVIDER`
    /// (`"local"` or `"cloud"`). If either is `"cloud"`, `GUIDE__CLOUD_FALLBACK`
    /// and `GUIDE__CLOUD_API_KEY` must both be set — the server will panic at
    /// startup if they are missing rather than silently falling back to local.
    ///
    /// Embeddings default to local inference via `guide-embed` (ORT + EmbeddingGemma).
    /// Set `GUIDE__EMBEDDING_PROVIDER=ollama` to use Ollama's embedding endpoint instead.
    pub async fn from_config(config: &AppConfig) -> Self {
        let ocr_uses_cloud = config.ocr_provider == "cloud";
        let story_uses_cloud = config.story_provider == "cloud";
        let needs_cloud = ocr_uses_cloud || story_uses_cloud;

        let ollama = OllamaProvider::new(
            &config.ollama_base_url,
            &config.default_model,
            &config.ocr_model,
            &config.embedding_model,
        );

        let cloud: Option<Arc<dyn LlmClient>> = if needs_cloud {
            let api_key = config.cloud_api_key.as_deref().unwrap_or_else(|| {
                panic!(
                    "GUIDE__CLOUD_API_KEY is required when OCR_PROVIDER or STORY_PROVIDER is 'cloud'"
                )
            });
            let provider_name = config.cloud_fallback.as_deref().unwrap_or_else(|| {
                panic!(
                    "GUIDE__CLOUD_FALLBACK is required when OCR_PROVIDER or STORY_PROVIDER is 'cloud'"
                )
            });

            let (base_url, default_model, label) = match provider_name {
                "openai" => (None, "gpt-4o-mini".to_string(), "openai".to_string()),
                "gemini" => (
                    Some(
                        "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
                    ),
                    "gemini-2.0-flash-lite".to_string(),
                    "gemini".to_string(),
                ),
                unknown => panic!(
                    "Unknown GUIDE__CLOUD_FALLBACK provider '{unknown}'; expected 'openai' or 'gemini'"
                ),
            };

            let model = config.cloud_model.clone().unwrap_or(default_model);
            tracing::info!(
                provider = %label,
                model = %model,
                ocr = ocr_uses_cloud,
                story = story_uses_cloud,
                "Cloud provider initialised"
            );
            Some(Arc::new(CloudProvider::new(
                api_key, model, base_url, label,
            )))
        } else {
            None
        };

        // Initialise local embedder when embedding_provider = "local" (the default).
        let local_embedder = if config.embedding_provider == "local" {
            match LocalEmbedder::from_pretrained(&config.embedding_model).await {
                Ok(e) => {
                    tracing::info!(
                        model = %config.embedding_model,
                        "Local embedder ready"
                    );
                    Some(Arc::new(e))
                }
                Err(e) => {
                    tracing::error!(
                        "Local embedder init failed ({e}); falling back to Ollama for embeddings"
                    );
                    None
                }
            }
        } else {
            None
        };

        let embedding_uses_cloud = config.embedding_provider == "cloud" && cloud.is_some();

        Self::new(
            RoutingStrategy::AlwaysLocal,
            Arc::new(ollama),
            cloud,
            ocr_uses_cloud,
            story_uses_cloud,
            local_embedder,
            embedding_uses_cloud,
        )
    }

    fn select_provider(&self, task: &LlmTask) -> Arc<dyn LlmClient> {
        // Embedding and character-sheet OCR always stay local
        match task {
            LlmTask::EmbeddingGeneration | LlmTask::CharacterSheetOcr => {
                return Arc::clone(&self.local);
            }
            LlmTask::StoryExtraction if self.story_uses_cloud => {
                return self
                    .cloud
                    .as_ref()
                    .map(Arc::clone)
                    .expect("Cloud provider required for story extraction but not initialised");
            }
            _ => {}
        }

        match &self.strategy {
            RoutingStrategy::AlwaysLocal => Arc::clone(&self.local),
            RoutingStrategy::AlwaysCloud { .. } => self
                .cloud
                .as_ref()
                .map(Arc::clone)
                .unwrap_or_else(|| Arc::clone(&self.local)),
        }
    }

    pub async fn route_complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let provider = self.select_provider(&req.task);
        provider.complete(req).await
    }

    pub async fn route_stream(
        &self,
        req: CompletionRequest,
    ) -> Result<BoxStream<'static, Result<String>>> {
        let provider = self.select_provider(&req.task);
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
        // 1. Local ORT embedder (default)
        if let Some(embedder) = &self.local_embedder {
            return match req.hint {
                EmbeddingHint::Query => embedder.embed_query(&req.text).await,
                EmbeddingHint::Document => embedder.embed_document(&req.text).await,
            };
        }

        // 2. Cloud embedding (if configured)
        if self.embedding_uses_cloud {
            if let Some(cloud) = &self.cloud {
                return cloud.embed(req).await;
            }
        }

        // 3. Ollama fallback
        self.local.embed(req).await
    }

    async fn complete_with_vision(&self, req: VisionRequest) -> Result<CompletionResponse> {
        if self.ocr_uses_cloud {
            let cloud = self.cloud.as_ref().ok_or_else(|| {
                GuideError::Internal("Cloud provider required for OCR but not initialised".into())
            })?;
            return cloud.complete_with_vision(req).await;
        }
        self.local.complete_with_vision(req).await
    }

    fn provider_name(&self) -> &str {
        "router"
    }
}
