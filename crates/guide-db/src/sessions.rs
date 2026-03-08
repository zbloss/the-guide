use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{
        CreateSessionEventRequest, CreateSessionRequest, EventSignificance, EventType, Session,
        SessionEvent,
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
        .await?;

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
        .await?
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
        .await?;

        rows.into_iter().map(row_to_session).collect()
    }

    pub async fn start_session(&self, id: Uuid) -> Result<Session> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET started_at = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(id.to_string())
            .execute(self.pool)
            .await?;
        self.get_by_id(id).await
    }

    pub async fn end_session(&self, id: Uuid) -> Result<Session> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET ended_at = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(id.to_string())
            .execute(self.pool)
            .await?;
        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
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
        .await?;

        Ok(row.try_get("next_num")?)
    }
}

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
        let event_type_str = event_type_to_str(&req.event_type);
        let significance = req.significance.unwrap_or_default();
        let significance_str = significance_to_str(&significance);
        let is_player_visible = req.is_player_visible.unwrap_or(true);
        let involved_ids = req.involved_character_ids.unwrap_or_default();
        let involved_json = serde_json::to_string(&involved_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>())?;

        sqlx::query(
            "INSERT INTO session_events \
             (id, session_id, campaign_id, event_type, description, significance, \
              is_player_visible, involved_character_ids, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(session_id.to_string())
        .bind(campaign_id.to_string())
        .bind(event_type_str)
        .bind(&req.description)
        .bind(significance_str)
        .bind(is_player_visible as i32)
        .bind(&involved_json)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

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
        .await?
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
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }

    pub async fn list_visible_by_session(&self, session_id: Uuid) -> Result<Vec<SessionEvent>> {
        let rows = sqlx::query(
            "SELECT id, session_id, campaign_id, event_type, description, significance, \
             is_player_visible, involved_character_ids, occurred_at \
             FROM session_events WHERE session_id = ? AND is_player_visible = 1 \
             ORDER BY occurred_at ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }
}

fn row_to_session(row: SqliteRow) -> Result<Session> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;
    let started_at_str: Option<String> = row.try_get("started_at")?;
    let ended_at_str: Option<String> = row.try_get("ended_at")?;

    Ok(Session {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        session_number: row.try_get("session_number")?,
        title: row.try_get("title")?,
        notes: row.try_get("notes")?,
        started_at: started_at_str.as_deref().and_then(|s| s.parse().ok()),
        ended_at: ended_at_str.as_deref().and_then(|s| s.parse().ok()),
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn row_to_event(row: SqliteRow) -> Result<SessionEvent> {
    let id_str: String = row.try_get("id")?;
    let session_id_str: String = row.try_get("session_id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let event_type_str: String = row.try_get("event_type")?;
    let significance_str: String = row.try_get("significance")?;
    let is_player_visible_int: i32 = row.try_get("is_player_visible")?;
    let involved_json: String = row.try_get("involved_character_ids")?;
    let occurred_at_str: String = row.try_get("occurred_at")?;

    let involved_strings: Vec<String> =
        serde_json::from_str(&involved_json).unwrap_or_default();
    let involved_character_ids = involved_strings
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect();

    Ok(SessionEvent {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        session_id: Uuid::parse_str(&session_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        event_type: parse_event_type(&event_type_str),
        description: row.try_get("description")?,
        significance: parse_significance(&significance_str),
        is_player_visible: is_player_visible_int != 0,
        involved_character_ids,
        occurred_at: occurred_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn event_type_to_str(t: &EventType) -> &'static str {
    match t {
        EventType::Combat => "combat",
        EventType::Exploration => "exploration",
        EventType::Social => "social",
        EventType::Rest => "rest",
        EventType::LevelUp => "level_up",
        EventType::ItemFound => "item_found",
        EventType::NpcMet => "npc_met",
        EventType::PlotRevealed => "plot_revealed",
        EventType::Custom => "custom",
    }
}

fn parse_event_type(s: &str) -> EventType {
    match s {
        "combat" => EventType::Combat,
        "exploration" => EventType::Exploration,
        "social" => EventType::Social,
        "rest" => EventType::Rest,
        "level_up" => EventType::LevelUp,
        "item_found" => EventType::ItemFound,
        "npc_met" => EventType::NpcMet,
        "plot_revealed" => EventType::PlotRevealed,
        _ => EventType::Custom,
    }
}

fn significance_to_str(s: &EventSignificance) -> &'static str {
    match s {
        EventSignificance::Minor => "minor",
        EventSignificance::Major => "major",
        EventSignificance::Milestone => "milestone",
    }
}

fn parse_significance(s: &str) -> EventSignificance {
    match s {
        "major" => EventSignificance::Major,
        "milestone" => EventSignificance::Milestone,
        _ => EventSignificance::Minor,
    }
}
