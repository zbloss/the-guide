use sqlx::SqlitePool;
use uuid::Uuid;

use guide_core::{models::GlobalDocument, GuideError, Result};

use crate::documents::parse_doc_kind;

pub struct CampaignGlobalDocRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CampaignGlobalDocRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Associate a global document with a campaign.
    pub async fn associate(&self, campaign_id: Uuid, global_doc_id: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO campaign_global_docs (campaign_id, global_doc_id) \
             VALUES (?, ?)",
        )
        .bind(campaign_id.to_string())
        .bind(global_doc_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Remove a global document association from a campaign.
    pub async fn remove(&self, campaign_id: Uuid, global_doc_id: Uuid) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM campaign_global_docs WHERE campaign_id = ? AND global_doc_id = ?",
        )
        .bind(campaign_id.to_string())
        .bind(global_doc_id.to_string())
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(GuideError::NotFound(format!(
                "Association campaign={campaign_id} global_doc={global_doc_id}"
            )));
        }
        Ok(())
    }

    /// List all global documents associated with a campaign.
    pub async fn list_for_campaign(&self, campaign_id: Uuid) -> Result<Vec<GlobalDocument>> {
        use chrono::{DateTime, Utc};
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT gd.id, gd.title, gd.filename, gd.file_size_bytes, gd.stored_path, \
             gd.page_count, gd.document_kind, gd.ingestion_status, gd.ingestion_error, \
             gd.uploaded_at, gd.ingested_at \
             FROM global_documents gd \
             JOIN campaign_global_docs cgd ON cgd.global_doc_id = gd.id \
             WHERE cgd.campaign_id = ? \
             ORDER BY cgd.added_at DESC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let id_str: String = row.try_get("id")?;
                let status_str: String = row.try_get("ingestion_status")?;
                let uploaded_at_str: String = row.try_get("uploaded_at")?;
                let ingested_at_str: Option<String> = row.try_get("ingested_at")?;
                let doc_kind_str: Option<String> = row.try_get("document_kind").ok();

                let parse_dt = |s: &str| -> DateTime<Utc> {
                    DateTime::parse_from_rfc3339(s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                };

                let ingestion_status = match status_str.as_str() {
                    "processing" => guide_core::models::IngestionStatus::Processing,
                    "completed" => guide_core::models::IngestionStatus::Completed,
                    "failed" => guide_core::models::IngestionStatus::Failed,
                    _ => guide_core::models::IngestionStatus::Pending,
                };

                Ok(GlobalDocument {
                    id: Uuid::parse_str(&id_str)
                        .map_err(|e| GuideError::Internal(e.to_string()))?,
                    title: row.try_get("title")?,
                    filename: row.try_get("filename")?,
                    file_size_bytes: row.try_get("file_size_bytes")?,
                    stored_path: row.try_get("stored_path")?,
                    page_count: row.try_get("page_count")?,
                    document_kind: doc_kind_str
                        .as_deref()
                        .map(parse_doc_kind)
                        .unwrap_or(guide_core::models::DocumentKind::DmGuide),
                    ingestion_status,
                    ingestion_error: row.try_get("ingestion_error")?,
                    uploaded_at: parse_dt(&uploaded_at_str),
                    ingested_at: ingested_at_str.as_deref().map(parse_dt),
                })
            })
            .collect()
    }
}

