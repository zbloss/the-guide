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
    Database(String),

    #[error("PDF processing error: {0}")]
    PdfProcessing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, GuideError>;

impl From<serde_json::Error> for GuideError {
    fn from(e: serde_json::Error) -> Self {
        GuideError::Serialization(e.to_string())
    }
}

impl From<config::ConfigError> for GuideError {
    fn from(e: config::ConfigError) -> Self {
        GuideError::Config(e.to_string())
    }
}
