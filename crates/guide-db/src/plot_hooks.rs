use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{
        CreateTrackedPlotHookRequest, PlotHookStatus, TrackedPlotHook, UpdateTrackedPlotHookRequest,
    },
    GuideError, Result,
};

pub struct PlotHookRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> PlotHookRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_by_character(&self, character_id: Uuid) -> Result<Vec<TrackedPlotHook>> {
        let rows = sqlx::query(
            "SELECT id, character_id, hook_text, status, session_resolved_id, created_at, updated_at \
             FROM plot_hooks WHERE character_id = ? ORDER BY created_at ASC",
        )
        .bind(character_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_hook).collect()
    }

    pub async fn create(
        &self,
        character_id: Uuid,
        req: &CreateTrackedPlotHookRequest,
    ) -> Result<TrackedPlotHook> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let status = req
            .status
            .as_ref()
            .map(status_to_str)
            .unwrap_or("open");

        sqlx::query(
            "INSERT INTO plot_hooks \
             (id, character_id, hook_text, status, session_resolved_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(character_id.to_string())
        .bind(&req.hook_text)
        .bind(status)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<TrackedPlotHook> {
        let row = sqlx::query(
            "SELECT id, character_id, hook_text, status, session_resolved_id, created_at, updated_at \
             FROM plot_hooks WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("PlotHook {id}")))?;

        row_to_hook(row)
    }

    pub async fn update(&self, id: Uuid, req: &UpdateTrackedPlotHookRequest) -> Result<TrackedPlotHook> {
        let now = Utc::now();

        let existing = self.get_by_id(id).await?;

        let new_status = req
            .status
            .as_ref()
            .map(status_to_str)
            .unwrap_or_else(|| status_to_str(&existing.status));

        let new_resolved_id = if req.session_resolved_id.is_some() {
            req.session_resolved_id.map(|u| u.to_string())
        } else {
            existing.session_resolved_id.map(|u| u.to_string())
        };

        sqlx::query(
            "UPDATE plot_hooks SET status = ?, session_resolved_id = ?, updated_at = ? WHERE id = ?",
        )
        .bind(new_status)
        .bind(new_resolved_id)
        .bind(now.to_rfc3339())
        .bind(id.to_string())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM plot_hooks WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("PlotHook {id}")));
        }
        Ok(())
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

fn row_to_hook(row: SqliteRow) -> Result<TrackedPlotHook> {
    let id_str: String = row.try_get("id")?;
    let character_id_str: String = row.try_get("character_id")?;
    let status_str: String = row.try_get("status")?;
    let session_resolved_id_str: Option<String> = row.try_get("session_resolved_id")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(TrackedPlotHook {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        character_id: Uuid::parse_str(&character_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        hook_text: row.try_get("hook_text")?,
        status: str_to_status(&status_str),
        session_resolved_id: session_resolved_id_str
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
