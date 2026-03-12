use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CalendarEntry, CreateCalendarEntryRequest},
    GuideError, Result,
};

pub struct CalendarRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CalendarRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: CreateCalendarEntryRequest,
    ) -> Result<CalendarEntry> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO campaign_calendar \
             (id, campaign_id, session_id, in_game_date, real_date, notes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(req.session_id.map(|u| u.to_string()))
        .bind(&req.in_game_date)
        .bind(&req.real_date)
        .bind(&req.notes)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CalendarEntry> {
        let row = sqlx::query(
            "SELECT id, campaign_id, session_id, in_game_date, real_date, notes, created_at \
             FROM campaign_calendar WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("CalendarEntry {id}")))?;

        row_to_entry(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CalendarEntry>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, session_id, in_game_date, real_date, notes, created_at \
             FROM campaign_calendar WHERE campaign_id = ? ORDER BY real_date ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_entry).collect()
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM campaign_calendar WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("CalendarEntry {id}")));
        }
        Ok(())
    }
}

fn row_to_entry(row: SqliteRow) -> Result<CalendarEntry> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let session_id_str: Option<String> = row.try_get("session_id")?;
    let created_at_str: String = row.try_get("created_at")?;

    Ok(CalendarEntry {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        session_id: session_id_str
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
        in_game_date: row.try_get("in_game_date")?,
        real_date: row.try_get("real_date")?,
        notes: row.try_get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
