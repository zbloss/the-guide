use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{
        CreateSessionEventRequest, CreateSessionRequest, EventSignificance, EventType, Session,
        SessionEvent,
    },
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct SessionRepository {
    pool: DuckDbPool,
}

impl SessionRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(&self, campaign_id: Uuid, req: CreateSessionRequest) -> Result<Session> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let session_number = self.next_session_number(campaign_id).await?;
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let title = req.title.clone();
        let notes = req.notes.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO sessions \
                 (id, campaign_id, session_number, title, notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![id_str, campaign_id_str, session_number, title, notes, now, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Session> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, session_number, title, notes, \
                 started_at, ended_at, map_url, created_at, updated_at \
                 FROM sessions WHERE id = ?",
                [&id_str],
                row_to_session,
                format!("Session {id}"),
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<Session>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, session_number, title, notes, \
                 started_at, ended_at, map_url, created_at, updated_at \
                 FROM sessions WHERE campaign_id = ? ORDER BY session_number ASC",
                [&id_str],
                row_to_session,
            )
        })
        .await
    }

    pub async fn update_map_url(&self, id: Uuid, url: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let url = url.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE sessions SET map_url = ?, updated_at = ? WHERE id = ?",
                duckdb::params![url, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn start_session(&self, id: Uuid) -> Result<Session> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let now2 = now.clone();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE sessions SET started_at = ?, updated_at = ? WHERE id = ?",
                duckdb::params![now, now2, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;
        self.get_by_id(id).await
    }

    pub async fn end_session(&self, id: Uuid) -> Result<Session> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let now2 = now.clone();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE sessions SET ended_at = ?, updated_at = ? WHERE id = ?",
                duckdb::params![now, now2, id_str],
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
                .execute("DELETE FROM sessions WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Session {id}")));
            }
            Ok(())
        })
        .await
    }

    async fn next_session_number(&self, campaign_id: Uuid) -> Result<i32> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.query_row(
                "SELECT COALESCE(MAX(session_number), 0) + 1 AS next_num \
                 FROM sessions WHERE campaign_id = ?",
                [&id_str],
                |r| r.get("next_num"),
            )
            .map_err(|e| GuideError::Internal(e.to_string()))
        })
        .await
    }
}

pub struct SessionEventRepository {
    pool: DuckDbPool,
}

impl SessionEventRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(
        &self,
        session_id: Uuid,
        campaign_id: Uuid,
        req: CreateSessionEventRequest,
    ) -> Result<SessionEvent> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let event_type_str = event_type_to_str(&req.event_type).to_string();
        let significance = req.significance.unwrap_or_default();
        let significance_str = significance_to_str(&significance).to_string();
        let involved_ids = req.involved_character_ids.unwrap_or_default();
        let involved_json = serde_json::to_string(
            &involved_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
        )?;
        let id_str = id.to_string();
        let session_id_str = session_id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let desc = req.description.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO session_events \
                 (id, session_id, campaign_id, event_type, description, significance, \
                  involved_character_ids, occurred_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    id_str, session_id_str, campaign_id_str, event_type_str,
                    desc, significance_str, involved_json, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<SessionEvent> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, session_id, campaign_id, event_type, description, significance, \
                 involved_character_ids, occurred_at \
                 FROM session_events WHERE id = ?",
                [&id_str],
                row_to_event,
                format!("SessionEvent {id}"),
            )
        })
        .await
    }

    pub async fn list_by_session(&self, session_id: Uuid) -> Result<Vec<SessionEvent>> {
        let id_str = session_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, session_id, campaign_id, event_type, description, significance, \
                 involved_character_ids, occurred_at \
                 FROM session_events WHERE session_id = ? ORDER BY occurred_at ASC",
                [&id_str],
                row_to_event,
            )
        })
        .await
    }

    pub async fn delete_event(&self, event_id: Uuid) -> Result<()> {
        let id_str = event_id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM session_events WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("SessionEvent {event_id}")));
            }
            Ok(())
        })
        .await
    }

    pub async fn list_visible_by_session(&self, session_id: Uuid) -> Result<Vec<SessionEvent>> {
        let id_str = session_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, session_id, campaign_id, event_type, description, significance, \
                 involved_character_ids, occurred_at \
                 FROM session_events WHERE session_id = ? \
                 ORDER BY occurred_at ASC",
                [&id_str],
                row_to_event,
            )
        })
        .await
    }
}

fn row_to_session(row: &duckdb::Row) -> duckdb::Result<Session> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;
    let started_at_str: Option<String> = row.get("started_at")?;
    let ended_at_str: Option<String> = row.get("ended_at")?;
    let map_url: Option<String> = row.get("map_url").unwrap_or(None);

    Ok(Session {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        session_number: row.get("session_number")?,
        title: row.get("title")?,
        notes: row.get("notes")?,
        started_at: started_at_str.as_deref().and_then(|s| s.parse().ok()),
        ended_at: ended_at_str.as_deref().and_then(|s| s.parse().ok()),
        map_url,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn row_to_event(row: &duckdb::Row) -> duckdb::Result<SessionEvent> {
    let id_str: String = row.get("id")?;
    let session_id_str: String = row.get("session_id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let event_type_str: String = row.get("event_type")?;
    let significance_str: String = row.get("significance")?;
    let involved_json: String = row.get("involved_character_ids")?;
    let occurred_at_str: String = row.get("occurred_at")?;

    let involved_strings: Vec<String> = serde_json::from_str(&involved_json).unwrap_or_default();
    let involved_character_ids = involved_strings
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect();

    Ok(SessionEvent {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        session_id: Uuid::parse_str(&session_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(2, duckdb::types::Type::Text, Box::new(e)))?,
        event_type: parse_event_type(&event_type_str),
        description: row.get("description")?,
        significance: parse_significance(&significance_str),
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
