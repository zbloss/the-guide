use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{CalendarEntry, CreateCalendarEntryRequest},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct CalendarRepository {
    pool: DuckDbPool,
}

impl CalendarRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: CreateCalendarEntryRequest,
    ) -> Result<CalendarEntry> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let session_id_str = req.session_id.map(|u| u.to_string());
        let in_game_date = req.in_game_date.clone();
        let real_date = req.real_date.clone();
        let notes = req.notes.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO campaign_calendar \
                 (id, campaign_id, session_id, in_game_date, real_date, notes, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, session_id_str,
                    in_game_date, real_date, notes, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CalendarEntry> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, session_id, in_game_date, real_date, notes, created_at \
                 FROM campaign_calendar WHERE id = ?",
                [&id_str],
                row_to_entry,
                format!("CalendarEntry {id}"),
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CalendarEntry>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, session_id, in_game_date, real_date, notes, created_at \
                 FROM campaign_calendar WHERE campaign_id = ? ORDER BY real_date ASC",
                [&id_str],
                row_to_entry,
            )
        })
        .await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM campaign_calendar WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("CalendarEntry {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_entry(row: &duckdb::Row) -> duckdb::Result<CalendarEntry> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let session_id_str: Option<String> = row.get("session_id")?;
    let created_at_str: String = row.get("created_at")?;

    Ok(CalendarEntry {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        session_id: session_id_str.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        in_game_date: row.get("in_game_date")?,
        real_date: row.get("real_date")?,
        notes: row.get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
