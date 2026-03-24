pub mod calendar;
pub mod campaign_global_docs;
pub mod campaigns;
pub mod characters;
pub mod chat;
pub mod documents;
pub mod encounters;
pub mod factions;
pub mod homebrew;
pub mod loot;
pub mod plot_hooks;
pub mod prep;
pub mod qdrant;
pub mod relationships;
pub mod sessions;
pub mod story;
pub mod templates;
pub mod webhooks;

pub use documents::PageOcrRow;
pub use plot_hooks::PlotHookRepository;
pub use prep::DmPrepRepository;
pub use relationships::RelationshipRepository;
pub use templates::TemplateRepository;
pub use webhooks::WebhookRepository;

pub use duckdb::DuckdbConnectionManager;
pub use r2d2::Pool;
pub type DuckDbPool = Pool<DuckdbConnectionManager>;

use guide_core::{GuideError, Result};

pub async fn init_duckdb(database_url: &str) -> Result<DuckDbPool> {
    let is_memory = database_url == ":memory:";
    let manager = if is_memory {
        DuckdbConnectionManager::memory()
            .map_err(|e| GuideError::Internal(e.to_string()))?
    } else {
        DuckdbConnectionManager::file(database_url)
            .map_err(|e| GuideError::Internal(e.to_string()))?
    };
    // In-memory: use single connection so all ops share the same DB instance.
    let max_size = if is_memory { 1 } else { 10 };
    let pool = r2d2::Pool::builder()
        .max_size(max_size)
        .build(manager)
        .map_err(|e| GuideError::Internal(e.to_string()))?;
    run_migrations(&pool).await?;
    Ok(pool)
}

/// Runs a blocking DB operation on a threadpool thread.
pub async fn with_db<F, R>(pool: &DuckDbPool, f: F) -> Result<R>
where
    F: FnOnce(&duckdb::Connection) -> Result<R> + Send + 'static,
    R: Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| GuideError::Internal(e.to_string()))?;
        f(&conn)
    })
    .await
    .map_err(|e| GuideError::Internal(e.to_string()))?
}

async fn run_migrations(pool: &DuckDbPool) -> Result<()> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let conn = pool.get().map_err(|e| GuideError::Internal(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                name TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
            )",
        )
        .map_err(|e| GuideError::Internal(e.to_string()))?;

        let migrations: &[(&str, &str)] = &[(
            "001_schema",
            include_str!("../migrations/001_schema.sql"),
        )];

        for (name, sql) in migrations {
            let applied: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM _migrations WHERE name = ?",
                    [name],
                    |r| r.get(0),
                )
                .unwrap_or(false);

            if !applied {
                conn.execute_batch(sql).map_err(|e| {
                    GuideError::Internal(format!("Migration {name} failed: {e}"))
                })?;
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "INSERT INTO _migrations (name, applied_at) VALUES (?, ?)",
                    duckdb::params![name, now],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| GuideError::Internal(e.to_string()))?
}

/// Helper: run a query that must return exactly one row, mapping NotFound gracefully.
pub(crate) fn query_one<T, P, F>(
    conn: &duckdb::Connection,
    sql: &str,
    params: P,
    f: F,
    not_found_msg: String,
) -> Result<T>
where
    P: duckdb::Params,
    F: FnOnce(&duckdb::Row) -> duckdb::Result<T>,
{
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| GuideError::Internal(e.to_string()))?;
    stmt.query_row(params, f).map_err(|e| match e {
        duckdb::Error::QueryReturnedNoRows => GuideError::NotFound(not_found_msg),
        _ => GuideError::Internal(e.to_string()),
    })
}

/// Helper: run a query that may return zero or one row.
pub(crate) fn query_optional<T, P, F>(
    conn: &duckdb::Connection,
    sql: &str,
    params: P,
    f: F,
) -> Result<Option<T>>
where
    P: duckdb::Params,
    F: FnOnce(&duckdb::Row) -> duckdb::Result<T>,
{
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| GuideError::Internal(e.to_string()))?;
    match stmt.query_row(params, f) {
        Ok(val) => Ok(Some(val)),
        Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(GuideError::Internal(e.to_string())),
    }
}

/// Helper: run a query that returns multiple rows.
pub(crate) fn query_all<T, P, F>(
    conn: &duckdb::Connection,
    sql: &str,
    params: P,
    f: F,
) -> Result<Vec<T>>
where
    P: duckdb::Params,
    F: Fn(&duckdb::Row) -> duckdb::Result<T>,
{
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| GuideError::Internal(e.to_string()))?;
    stmt.query_map(params, f)
        .map_err(|e| GuideError::Internal(e.to_string()))?
        .collect::<duckdb::Result<Vec<T>>>()
        .map_err(|e| GuideError::Internal(e.to_string()))
}
