use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CampaignWebhook, CreateWebhookRequest},
    GuideError, Result,
};

pub struct WebhookRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> WebhookRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CampaignWebhook>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, url, events, created_at \
             FROM campaign_webhooks WHERE campaign_id = ? ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_webhook).collect()
    }

    pub async fn list_by_event(
        &self,
        campaign_id: Uuid,
        event: &str,
    ) -> Result<Vec<CampaignWebhook>> {
        // We fetch all for the campaign and filter by event in memory (JSON array stored as text)
        let all = self.list_by_campaign(campaign_id).await?;
        Ok(all
            .into_iter()
            .filter(|w| w.events.iter().any(|e| e == event))
            .collect())
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: &CreateWebhookRequest,
    ) -> Result<CampaignWebhook> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let default_events = vec![
            "session_start".to_string(),
            "session_end".to_string(),
            "encounter_end".to_string(),
        ];
        let events = req.events.clone().unwrap_or(default_events);
        let events_json = serde_json::to_string(&events)
            .map_err(|e| GuideError::Internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO campaign_webhooks (id, campaign_id, url, events, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(&req.url)
        .bind(&events_json)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CampaignWebhook> {
        let row = sqlx::query(
            "SELECT id, campaign_id, url, events, created_at \
             FROM campaign_webhooks WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("CampaignWebhook {id}")))?;

        row_to_webhook(row)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM campaign_webhooks WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("CampaignWebhook {id}")));
        }
        Ok(())
    }
}

fn row_to_webhook(row: SqliteRow) -> Result<CampaignWebhook> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let events_str: String = row.try_get("events")?;
    let created_at_str: String = row.try_get("created_at")?;

    let events: Vec<String> = serde_json::from_str(&events_str)
        .unwrap_or_else(|_| vec!["session_start".to_string(), "session_end".to_string(), "encounter_end".to_string()]);

    Ok(CampaignWebhook {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        url: row.try_get("url")?,
        events,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
