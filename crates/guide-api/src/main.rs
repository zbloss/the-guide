use anyhow::Context;
use guide_api::{routes, state};
use guide_core::AppConfig;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::load().context("Failed to load configuration")?;
    let addr = format!("{}:{}", config.host, config.port);
    info!("Starting The Guide on {addr}");

    let state = state::AppState::init(config)
        .await
        .context("Failed to initialise application state")?;

    let max_upload = state.config.max_upload_bytes as usize;
    let app = routes::all_routes(state)
        .layer(axum::extract::DefaultBodyLimit::max(max_upload));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;

    info!("Listening on http://{addr}");
    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}
