use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub qdrant: QdrantConfig,
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// SQLite connection URL, e.g. "sqlite://./data/guide.db"
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QdrantConfig {
    pub url: String,
    /// Vector dimension for nomic-embed-text
    pub vector_size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    /// Ollama OpenAI-compatible base URL
    pub ollama_base_url: String,
    /// Default chat model for campaign assistant tasks
    pub default_model: String,
    /// Model used for OCR extraction (GLM-OCR)
    pub ocr_model: String,
    /// Model used for vision description
    pub vision_model: String,
    /// Model used for embedding generation
    pub embedding_model: String,
    /// Optional cloud fallback provider: "openai" | "anthropic" | "gemini" | null
    pub cloud_fallback: Option<String>,
    /// API key for cloud fallback provider (read from env)
    pub cloud_api_key: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8000,
            },
            database: DatabaseConfig {
                url: "sqlite://./data/guide.db".to_string(),
            },
            qdrant: QdrantConfig {
                url: "http://localhost:6334".to_string(),
                vector_size: 768,
            },
            llm: LlmConfig {
                ollama_base_url: "http://localhost:11434/v1".to_string(),
                default_model: "llama3.2".to_string(),
                ocr_model: "glm4v".to_string(),
                vision_model: "llama3.2-vision".to_string(),
                embedding_model: "nomic-embed-text".to_string(),
                cloud_fallback: None,
                cloud_api_key: None,
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from `config.toml` (optional) then environment variables.
    /// Environment variables use the `GUIDE__` prefix with `__` as separator.
    /// Falls back to defaults for any unset values.
    pub fn load() -> crate::Result<Self> {
        dotenvy::dotenv().ok();

        let d = AppConfig::default();

        let cfg = config::Config::builder()
            // Built-in defaults so partial env-var overrides work
            .set_default("server.host", d.server.host).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("server.port", d.server.port as i64).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("database.url", d.database.url).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("qdrant.url", d.qdrant.url).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("qdrant.vector_size", d.qdrant.vector_size as i64).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("llm.ollama_base_url", d.llm.ollama_base_url).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("llm.default_model", d.llm.default_model).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("llm.ocr_model", d.llm.ocr_model).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("llm.vision_model", d.llm.vision_model).map_err(|e| crate::GuideError::Config(e.to_string()))?
            .set_default("llm.embedding_model", d.llm.embedding_model).map_err(|e| crate::GuideError::Config(e.to_string()))?
            // Optional file override
            .add_source(config::File::with_name("config").required(false))
            // Env vars win: GUIDE__SERVER__PORT=8000 etc.
            .add_source(config::Environment::with_prefix("GUIDE").separator("__"))
            .build()
            .map_err(|e| crate::GuideError::Config(e.to_string()))?;

        cfg.try_deserialize::<AppConfig>().map_err(|e| crate::GuideError::Config(e.to_string()))
    }
}
