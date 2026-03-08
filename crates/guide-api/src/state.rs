use std::sync::Arc;

use guide_core::AppConfig;
use guide_db::SqlitePool;
use guide_llm::{LlmClient, LlmRouter};
use qdrant_client::Qdrant;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub llm: Arc<dyn LlmClient>,
    pub db: SqlitePool,
    pub qdrant: Option<Arc<Qdrant>>,
}

impl AppState {
    pub async fn init(config: AppConfig) -> anyhow::Result<Self> {
        let db = guide_db::init_sqlite(&config.database_url).await?;
        let llm: Arc<dyn LlmClient> = Arc::new(LlmRouter::from_config(&config));
        let qdrant = guide_db::qdrant::try_connect(&config.qdrant_url)
            .await
            .map(Arc::new);

        Ok(AppState {
            config: Arc::new(config),
            llm,
            db,
            qdrant,
        })
    }
}
