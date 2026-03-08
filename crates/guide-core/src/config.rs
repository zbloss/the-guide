use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub ollama_base_url: String,
    pub default_model: String,
    pub embedding_model: String,
    pub ocr_model: String,
    pub cloud_fallback: Option<String>,
    pub cloud_api_key: Option<String>,
    pub max_upload_bytes: u64,
    pub chunk_max_chars: usize,
    pub chunk_overlap_chars: usize,
    pub qdrant_url: String,
    pub qdrant_collection: String,
    pub embedding_dims: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8000,
            database_url: "sqlite://./data/guide.db".into(),
            ollama_base_url: "http://localhost:11434/v1".into(),
            default_model: "qwen3.5:9b".into(),
            embedding_model: "nomic-embed-text".into(),
            ocr_model: "glm-ocr".into(),
            cloud_fallback: None,
            cloud_api_key: None,
            max_upload_bytes: 50 * 1024 * 1024,
            chunk_max_chars: 2048,
            chunk_overlap_chars: 64,
            qdrant_url: "http://localhost:6333".into(),
            qdrant_collection: "guide_chunks".into(),
            embedding_dims: 768,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        let defaults = Self::default();

        let cfg = config::Config::builder()
            .set_default("host", defaults.host)?
            .set_default("port", defaults.port as i64)?
            .set_default("database_url", defaults.database_url)?
            .set_default("ollama_base_url", defaults.ollama_base_url)?
            .set_default("default_model", defaults.default_model)?
            .set_default("embedding_model", defaults.embedding_model)?
            .set_default("ocr_model", defaults.ocr_model)?
            .set_default("cloud_fallback", Option::<String>::None)?
            .set_default("cloud_api_key", Option::<String>::None)?
            .set_default("max_upload_bytes", defaults.max_upload_bytes as i64)?
            .set_default("chunk_max_chars", defaults.chunk_max_chars as i64)?
            .set_default("chunk_overlap_chars", defaults.chunk_overlap_chars as i64)?
            .set_default("qdrant_url", defaults.qdrant_url)?
            .set_default("qdrant_collection", defaults.qdrant_collection)?
            .set_default("embedding_dims", defaults.embedding_dims as i64)?
            .add_source(
                config::Environment::with_prefix("GUIDE")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        Ok(cfg.try_deserialize()?)
    }
}
