use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = Value, example = json!({"status": "ok"}))
    )
)]
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

#[utoipa::path(
    get,
    path = "/version",
    responses(
        (status = 200, description = "Service version information", body = Value, example = json!({"version": "0.1.0", "name": "guide-api"}))
    )
)]
async fn version() -> Json<Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
    }))
}
