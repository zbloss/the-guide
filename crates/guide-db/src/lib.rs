pub mod campaigns;
pub mod characters;
pub mod documents;
pub mod encounters;
pub mod qdrant;
pub mod sessions;

pub use sqlx::SqlitePool;

use guide_core::{GuideError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;

pub async fn init_sqlite(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| GuideError::Internal(e.to_string()))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await.map_err(|e| {
        GuideError::Internal(format!("Migration failed: {e}"))
    })?;

    Ok(pool)
}
