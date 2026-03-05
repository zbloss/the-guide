use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{
        CreateSessionEventRequest, CreateSessionRequest, EventSignificance, EventType,
        Session, SessionEvent,
    },
    GuideError, Result,
};

pub struct SessionRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SessionRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, campaign_id: Uuid, req: CreateSessionRequest) -> Result<Session> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let session_number = self.next_session_number(campaign_id).await?;

        sqlx::query(
            "INSERT INTO sessions \
             (id, campaign_id, session_number, title, notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(session_number)
        .bind(&req.title)
        .bind(&req.notes)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Session> {
        let row = sqlx::query(
            "SELECT id, campaign_id, session_number, title, notes, \
             started_at, ended_at, created_at, updated_at \
             FROM sessions WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?
        .ok_or_else(|| GuideError::NotFound(format!("Session {id}")))?;

        row_to_session(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, session_number, title, notes, \
             started_at, ended_at, created_at, updated_at \
             FROM sessions WHERE campaign_id = ? ORDER BY session_number ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        rows.into_iter().map(row_to_session).collect()
    }

    pub async fn start_session(&self, id: Uuid) -> Result<Session> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET started_at = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(id.to_string())
            .execute(self.pool)
            .await
            .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;
        self.get_by_id(id).await
    }

    pub async fn end_session(&self, id: Uuid) -> Result<Session> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET ended_at = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(id.to_string())
            .execute(self.pool)
            .await
            .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;
        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await
            .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Session {id}")));
        }
        Ok(())
    }

    async fn next_session_number(&self, campaign_id: Uuid) -> Result<i32> {
        let row = sqlx::query(
            "SELECT COALESCE(MAX(session_number), 0) + 1 AS next_num \
             FROM sessions WHERE campaign_id = ?",
        )
        .bind(campaign_id.to_string())
        .fetch_one(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        let next: i32 = row
            .try_get("next_num")
            .map_err(|e| GuideError::Database(e.to_string()))?;
        Ok(next)
    }
}

// ── Session Events ────────────────────────────────────────────────────────────

pub struct SessionEventRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SessionEventRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        session_id: Uuid,
        campaign_id: Uuid,
        req: CreateSessionEventRequest,
    ) -> Result<SessionEvent> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let event_type_str = serde_json::to_string(&req.event_type)
            .map_err(|e| GuideError::Serialization(e.to_string()))?;
        let event_type_str = event_type_str.trim_matches('"').to_string();

        let significance = req.significance.unwrap_or(EventSignificance::Minor);
        let sig_str = serde_json::to_string(&significance)
            .map_err(|e| GuideError::Serialization(e.to_string()))?;
        let sig_str = sig_str.trim_matches('"').to_string();

        let is_player_visible = req.is_player_visible.unwrap_or(true) as i32;
        let char_ids = req.involved_character_ids.unwrap_or_default();
        let char_ids_json = serde_json::to_string(&char_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>())
            .map_err(|e| GuideError::Serialization(e.to_string()))?;

        sqlx::query(
            "INSERT INTO session_events \
             (id, session_id, campaign_id, event_type, description, significance, \
              is_player_visible, involved_character_ids, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(session_id.to_string())
        .bind(campaign_id.to_string())
        .bind(&event_type_str)
        .bind(&req.description)
        .bind(&sig_str)
        .bind(is_player_visible)
        .bind(&char_ids_json)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<SessionEvent> {
        let row = sqlx::query(
            "SELECT id, session_id, campaign_id, event_type, description, significance, \
             is_player_visible, involved_character_ids, occurred_at \
             FROM session_events WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?
        .ok_or_else(|| GuideError::NotFound(format!("SessionEvent {id}")))?;

        row_to_event(row)
    }

    pub async fn list_by_session(&self, session_id: Uuid) -> Result<Vec<SessionEvent>> {
        let rows = sqlx::query(
            "SELECT id, session_id, campaign_id, event_type, description, significance, \
             is_player_visible, involved_character_ids, occurred_at \
             FROM session_events WHERE session_id = ? ORDER BY occurred_at ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        rows.into_iter().map(row_to_event).collect()
    }

    /// List only player-visible events (used for player summaries).
    pub async fn list_visible_by_session(&self, session_id: Uuid) -> Result<Vec<SessionEvent>> {
        let rows = sqlx::query(
            "SELECT id, session_id, campaign_id, event_type, description, significance, \
             is_player_visible, involved_character_ids, occurred_at \
             FROM session_events WHERE session_id = ? AND is_player_visible = 1 \
             ORDER BY occurred_at ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        rows.into_iter().map(row_to_event).collect()
    }
}

// ── Row mapping ───────────────────────────────────────────────────────────────

fn row_to_session(row: SqliteRow) -> Result<Session> {
    let id_str: String = row.try_get("id").map_err(|e| GuideError::Database(e.to_string()))?;
    let campaign_id_str: String =
        row.try_get("campaign_id").map_err(|e| GuideError::Database(e.to_string()))?;
    let created_at_str: String =
        row.try_get("created_at").map_err(|e| GuideError::Database(e.to_string()))?;
    let updated_at_str: String =
        row.try_get("updated_at").map_err(|e| GuideError::Database(e.to_string()))?;
    let started_at_str: Option<String> =
        row.try_get("started_at").map_err(|e| GuideError::Database(e.to_string()))?;
    let ended_at_str: Option<String> =
        row.try_get("ended_at").map_err(|e| GuideError::Database(e.to_string()))?;

    Ok(Session {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        session_number: row
            .try_get("session_number")
            .map_err(|e| GuideError::Database(e.to_string()))?,
        title: row.try_get("title").map_err(|e| GuideError::Database(e.to_string()))?,
        notes: row.try_get("notes").map_err(|e| GuideError::Database(e.to_string()))?,
        started_at: started_at_str.as_deref().and_then(|s| s.parse().ok()),
        ended_at: ended_at_str.as_deref().and_then(|s| s.parse().ok()),
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn row_to_event(row: SqliteRow) -> Result<SessionEvent> {
    let id_str: String = row.try_get("id").map_err(|e| GuideError::Database(e.to_string()))?;
    let session_id_str: String =
        row.try_get("session_id").map_err(|e| GuideError::Database(e.to_string()))?;
    let campaign_id_str: String =
        row.try_get("campaign_id").map_err(|e| GuideError::Database(e.to_string()))?;
    let event_type_str: String =
        row.try_get("event_type").map_err(|e| GuideError::Database(e.to_string()))?;
    let sig_str: String =
        row.try_get("significance").map_err(|e| GuideError::Database(e.to_string()))?;
    let is_player_visible_int: i32 =
        row.try_get("is_player_visible").map_err(|e| GuideError::Database(e.to_string()))?;
    let char_ids_json: String =
        row.try_get("involved_character_ids").map_err(|e| GuideError::Database(e.to_string()))?;
    let occurred_at_str: String =
        row.try_get("occurred_at").map_err(|e| GuideError::Database(e.to_string()))?;

    let event_type: EventType =
        serde_json::from_str(&format!("\"{}\"", event_type_str)).unwrap_or(EventType::Custom(event_type_str));
    let significance: EventSignificance =
        serde_json::from_str(&format!("\"{}\"", sig_str)).unwrap_or(EventSignificance::Minor);
    let char_id_strings: Vec<String> =
        serde_json::from_str(&char_ids_json).unwrap_or_default();
    let involved_character_ids: Vec<Uuid> = char_id_strings
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    Ok(SessionEvent {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        session_id: Uuid::parse_str(&session_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        event_type,
        description: row
            .try_get("description")
            .map_err(|e| GuideError::Database(e.to_string()))?,
        significance,
        is_player_visible: is_player_visible_int != 0,
        involved_character_ids,
        occurred_at: occurred_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
