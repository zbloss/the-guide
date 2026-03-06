mod routes;
mod state;

use anyhow::Context;
use guide_core::config::AppConfig;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── Config ────────────────────────────────────────────────────────────────
    let config = AppConfig::load().context("Failed to load configuration")?;
    let addr = format!("{}:{}", config.server.host, config.server.port);
    info!("Starting The Guide on {addr}");

    // ── App state (DB + LLM + Qdrant) ─────────────────────────────────────────
    let state = state::AppState::init(config)
        .await
        .context("Failed to initialise application state")?;

    // ── Router ────────────────────────────────────────────────────────────────
    let max_upload = state.config.upload.max_upload_bytes as usize;
    let app = routes::all_routes(state).layer(axum::extract::DefaultBodyLimit::max(max_upload));

    // ── Serve ─────────────────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;

    info!("Listening on http://{addr}");
    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
