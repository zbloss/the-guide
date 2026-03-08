use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuideError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Qdrant error: {0}")]
    Qdrant(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("PDF processing error: {0}")]
    PdfProcessing(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, GuideError>;
