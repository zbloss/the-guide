pub mod campaigns;
pub mod characters;
pub mod documents;
pub mod encounters;
pub mod qdrant;
pub mod sessions;

pub use sqlx::SqlitePool;

use guide_core::{GuideError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

/// Initialise the SQLite connection pool and run any pending migrations.
pub async fn init_sqlite(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| GuideError::Database(e.to_string()))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await
        .map_err(|e| GuideError::Database(e.to_string()))?;

    run_migrations(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| GuideError::Database(e.to_string()))?;
    Ok(())
}
