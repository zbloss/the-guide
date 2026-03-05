use std::sync::Arc;

use guide_core::config::AppConfig;
use guide_db::SqlitePool;
use guide_llm::LlmRouter;
use qdrant_client::Qdrant;

/// Shared application state, wrapped in `Arc` and injected by Axum.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub llm: Arc<LlmRouter>,
    pub db: SqlitePool,
    /// None if Qdrant is unavailable at startup — routes that require vector
    /// search will return 503.
    pub qdrant: Option<Arc<Qdrant>>,
}

impl AppState {
    pub async fn init(config: AppConfig) -> anyhow::Result<Self> {
        // ── SQLite ────────────────────────────────────────────────────────────
        let db = guide_db::init_sqlite(&config.database.url).await?;

        // ── LLM router ────────────────────────────────────────────────────────
        let llm = Arc::new(LlmRouter::from_config(&config.llm));

        // ── Qdrant (optional) ─────────────────────────────────────────────────
        let qdrant = guide_db::qdrant::try_connect(&config.qdrant.url)
            .await
            .map(|q| Arc::new(q));

        Ok(AppState {
            config: Arc::new(config),
            llm,
            db,
            qdrant,
        })
    }
}
