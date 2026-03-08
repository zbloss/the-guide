use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use guide_core::GuideError;
use serde_json::json;

/// Newtype wrapper so we can implement `IntoResponse` for `GuideError`
/// (orphan rule: both the trait and the type are from other crates).
pub struct AppError(pub GuideError);

impl From<GuideError> for AppError {
    fn from(e: GuideError) -> Self {
        AppError(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let e = self.0;
        let (status, message) = match &e {
            GuideError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            GuideError::InvalidInput(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            GuideError::Llm(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("LLM error: {msg}"),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        tracing::error!("Request error: {e}");
        (status, Json(json!({ "error": message }))).into_response()
    }
}
