use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{CampaignWebhook, CreateWebhookRequest},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct WebhookRepository {
    pool: DuckDbPool,
}

impl WebhookRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CampaignWebhook>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, url, events, created_at \
                 FROM campaign_webhooks WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_webhook,
            )
        })
        .await
    }

    pub async fn list_by_event(
        &self,
        campaign_id: Uuid,
        event: &str,
    ) -> Result<Vec<CampaignWebhook>> {
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
        let now = Utc::now().to_rfc3339();
        let default_events = vec![
            "session_start".to_string(),
            "session_end".to_string(),
            "encounter_end".to_string(),
        ];
        let events = req.events.clone().unwrap_or(default_events);
        let events_json = serde_json::to_string(&events)
            .map_err(|e| GuideError::Internal(e.to_string()))?;
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let url = req.url.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO campaign_webhooks (id, campaign_id, url, events, created_at) \
                 VALUES (?, ?, ?, ?, ?)",
                duckdb::params![id_str, campaign_id_str, url, events_json, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CampaignWebhook> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, url, events, created_at \
                 FROM campaign_webhooks WHERE id = ?",
                [&id_str],
                row_to_webhook,
                format!("CampaignWebhook {id}"),
            )
        })
        .await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM campaign_webhooks WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("CampaignWebhook {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_webhook(row: &duckdb::Row) -> duckdb::Result<CampaignWebhook> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let events_str: String = row.get("events")?;
    let created_at_str: String = row.get("created_at")?;

    let events: Vec<String> = serde_json::from_str(&events_str).unwrap_or_else(|_| {
        vec![
            "session_start".to_string(),
            "session_end".to_string(),
            "encounter_end".to_string(),
        ]
    });

    Ok(CampaignWebhook {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        url: row.get("url")?,
        events,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
