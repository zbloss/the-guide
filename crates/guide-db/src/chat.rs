use chrono::Utc;
use uuid::Uuid;

use guide_core::{models::ChatMessage, GuideError, Result};

use crate::{query_all, with_db, DuckDbPool};

pub struct ChatRepository {
    pool: DuckDbPool,
}

impl ChatRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn append(
        &self,
        campaign_id: Uuid,
        role: &str,
        content: &str,
    ) -> Result<ChatMessage> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let role = role.to_string();
        let content = content.to_string();
        let role2 = role.clone();
        let content2 = content.clone();
        let now2 = now.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO campaign_chat (id, campaign_id, role, content, created_at) \
                 VALUES (?, ?, ?, ?, ?)",
                duckdb::params![id_str, campaign_id_str, role, content, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        Ok(ChatMessage {
            id,
            campaign_id,
            role: role2,
            content: content2,
            created_at: now2.parse().unwrap_or_else(|_| Utc::now()),
        })
    }

    pub async fn list(&self, campaign_id: Uuid, limit: i64) -> Result<Vec<ChatMessage>> {
        let id_str = campaign_id.to_string();
        let mut msgs: Vec<ChatMessage> = with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, role, content, created_at \
                 FROM campaign_chat WHERE campaign_id = ? \
                 ORDER BY created_at DESC LIMIT ?",
                duckdb::params![id_str, limit],
                row_to_msg,
            )
        })
        .await?;
        msgs.reverse();
        Ok(msgs)
    }
}

fn row_to_msg(row: &duckdb::Row) -> duckdb::Result<ChatMessage> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let created_at_str: String = row.get("created_at")?;
    Ok(ChatMessage {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        role: row.get("role")?,
        content: row.get("content")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
