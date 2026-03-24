use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{
        CreateTrackedPlotHookRequest, PlotHookStatus, TrackedPlotHook, UpdateTrackedPlotHookRequest,
    },
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct PlotHookRepository {
    pool: DuckDbPool,
}

impl PlotHookRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn list_by_character(&self, character_id: Uuid) -> Result<Vec<TrackedPlotHook>> {
        let id_str = character_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, character_id, hook_text, status, session_resolved_id, created_at, updated_at \
                 FROM plot_hooks WHERE character_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_hook,
            )
        })
        .await
    }

    pub async fn create(
        &self,
        character_id: Uuid,
        req: &CreateTrackedPlotHookRequest,
    ) -> Result<TrackedPlotHook> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let status = req.status.as_ref().map(status_to_str).unwrap_or("open").to_string();
        let id_str = id.to_string();
        let char_id_str = character_id.to_string();
        let hook_text = req.hook_text.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO plot_hooks \
                 (id, character_id, hook_text, status, session_resolved_id, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, NULL, ?, ?)",
                duckdb::params![id_str, char_id_str, hook_text, status, now, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<TrackedPlotHook> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, character_id, hook_text, status, session_resolved_id, created_at, updated_at \
                 FROM plot_hooks WHERE id = ?",
                [&id_str],
                row_to_hook,
                format!("PlotHook {id}"),
            )
        })
        .await
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: &UpdateTrackedPlotHookRequest,
    ) -> Result<TrackedPlotHook> {
        let now = Utc::now();
        let existing = self.get_by_id(id).await?;

        let new_status = req
            .status
            .as_ref()
            .map(status_to_str)
            .unwrap_or_else(|| status_to_str(&existing.status))
            .to_string();
        let new_resolved_id = if req.session_resolved_id.is_some() {
            req.session_resolved_id.map(|u| u.to_string())
        } else {
            existing.session_resolved_id.map(|u| u.to_string())
        };
        let id_str = id.to_string();
        let now_str = now.to_rfc3339();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE plot_hooks SET status = ?, session_resolved_id = ?, updated_at = ? WHERE id = ?",
                duckdb::params![new_status, new_resolved_id, now_str, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM plot_hooks WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("PlotHook {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn status_to_str(status: &PlotHookStatus) -> &'static str {
    match status {
        PlotHookStatus::Open => "open",
        PlotHookStatus::Active => "active",
        PlotHookStatus::Resolved => "resolved",
    }
}

fn str_to_status(s: &str) -> PlotHookStatus {
    match s {
        "active" => PlotHookStatus::Active,
        "resolved" => PlotHookStatus::Resolved,
        _ => PlotHookStatus::Open,
    }
}

fn row_to_hook(row: &duckdb::Row) -> duckdb::Result<TrackedPlotHook> {
    let id_str: String = row.get("id")?;
    let character_id_str: String = row.get("character_id")?;
    let status_str: String = row.get("status")?;
    let session_resolved_id_str: Option<String> = row.get("session_resolved_id")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let session_resolved_id = session_resolved_id_str
        .as_deref()
        .map(|s| {
            Uuid::parse_str(s).map_err(|e| {
                duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e))
            })
        })
        .transpose()?;

    Ok(TrackedPlotHook {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        character_id: Uuid::parse_str(&character_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        hook_text: row.get("hook_text")?,
        status: str_to_status(&status_str),
        session_resolved_id,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
