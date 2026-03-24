use std::sync::Arc;

use guide_core::AppConfig;
use guide_db::DuckDbPool;
use guide_llm::{LlmClient, LlmRouter};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub llm: Arc<dyn LlmClient>,
    pub db: DuckDbPool,
}

impl AppState {
    pub async fn init(config: AppConfig) -> anyhow::Result<Self> {
        let db = guide_db::init_duckdb(&config.database_url).await?;
        let llm: Arc<dyn LlmClient> = Arc::new(LlmRouter::from_config(&config).await);

        use guide_db::documents::{DocumentRepository, GlobalDocumentRepository};
        DocumentRepository::new(&db).reset_stuck_processing().await?;
        GlobalDocumentRepository::new(&db).reset_stuck_processing().await?;

        Ok(AppState {
            config: Arc::new(config),
            llm,
            db,
        })
    }
}
