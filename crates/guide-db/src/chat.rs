use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;
use guide_core::{models::ChatMessage, GuideError, Result};

pub struct ChatRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ChatRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn append(
        &self,
        campaign_id: Uuid,
        role: &str,
        content: &str,
        perspective: &str,
    ) -> Result<ChatMessage> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO campaign_chat (id, campaign_id, role, content, perspective, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(role)
        .bind(content)
        .bind(perspective)
        .bind(&now)
        .execute(self.pool)
        .await?;

        Ok(ChatMessage {
            id,
            campaign_id,
            role: role.to_string(),
            content: content.to_string(),
            perspective: perspective.to_string(),
            created_at: now.parse().unwrap_or_else(|_| Utc::now()),
        })
    }

    pub async fn list(&self, campaign_id: Uuid, limit: i64) -> Result<Vec<ChatMessage>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, role, content, perspective, created_at \
             FROM campaign_chat \
             WHERE campaign_id = ? \
             ORDER BY created_at DESC \
             LIMIT ?",
        )
        .bind(campaign_id.to_string())
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        let mut msgs: Vec<ChatMessage> = rows
            .into_iter()
            .map(row_to_msg)
            .collect::<Result<_>>()?;
        msgs.reverse();
        Ok(msgs)
    }
}

fn row_to_msg(row: SqliteRow) -> Result<ChatMessage> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let created_at_str: String = row.try_get("created_at")?;
    Ok(ChatMessage {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        role: row.try_get("role")?,
        content: row.try_get("content")?,
        perspective: row.try_get("perspective")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
