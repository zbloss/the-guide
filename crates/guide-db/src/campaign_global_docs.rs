use chrono::{DateTime, Utc};
use uuid::Uuid;

use guide_core::{models::GlobalDocument, GuideError, Result};

use crate::{documents::parse_doc_kind, with_db, DuckDbPool};

pub struct CampaignGlobalDocRepository {
    pool: DuckDbPool,
}

impl CampaignGlobalDocRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    /// Associate a global document with a campaign.
    pub async fn associate(&self, campaign_id: Uuid, global_doc_id: Uuid) -> Result<()> {
        let campaign_id_str = campaign_id.to_string();
        let global_doc_id_str = global_doc_id.to_string();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO campaign_global_docs (campaign_id, global_doc_id, added_at) \
                 VALUES (?, ?, ?)",
                duckdb::params![campaign_id_str, global_doc_id_str, Utc::now().to_rfc3339()],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    /// Remove a global document association from a campaign.
    pub async fn remove(&self, campaign_id: Uuid, global_doc_id: Uuid) -> Result<()> {
        let campaign_id_str = campaign_id.to_string();
        let global_doc_id_str = global_doc_id.to_string();

        with_db(&self.pool, move |conn| {
            let n = conn
                .execute(
                    "DELETE FROM campaign_global_docs WHERE campaign_id = ? AND global_doc_id = ?",
                    duckdb::params![campaign_id_str, global_doc_id_str],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!(
                    "Association campaign={campaign_id} global_doc={global_doc_id}"
                )));
            }
            Ok(())
        })
        .await
    }

    /// List all global documents associated with a campaign.
    pub async fn list_for_campaign(&self, campaign_id: Uuid) -> Result<Vec<GlobalDocument>> {
        let id_str = campaign_id.to_string();

        with_db(&self.pool, move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT gd.id, gd.title, gd.filename, gd.file_size_bytes, gd.stored_path, \
                     gd.page_count, gd.document_kind, gd.ingestion_status, gd.ingestion_error, \
                     gd.uploaded_at, gd.ingested_at \
                     FROM global_documents gd \
                     JOIN campaign_global_docs cgd ON cgd.global_doc_id = gd.id \
                     WHERE cgd.campaign_id = ? \
                     ORDER BY cgd.added_at DESC",
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            let rows = stmt
                .query_map([&id_str], |row| {
                    let id_str: String = row.get("id")?;
                    let status_str: String = row.get("ingestion_status")?;
                    let uploaded_at_str: String = row.get("uploaded_at")?;
                    let ingested_at_str: Option<String> = row.get("ingested_at")?;
                    let doc_kind_str: Option<String> = row.get("document_kind").ok();

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

                    Ok((
                        id_str,
                        status_str,
                        uploaded_at_str,
                        ingested_at_str,
                        doc_kind_str,
                        ingestion_status,
                        row.get::<_, String>("title")?,
                        row.get::<_, String>("filename")?,
                        row.get::<_, i64>("file_size_bytes")?,
                        row.get::<_, String>("stored_path")?,
                        row.get::<_, Option<i32>>("page_count")?,
                        row.get::<_, Option<String>>("ingestion_error")?,
                        parse_dt,
                    ))
                })
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            let mut result = Vec::new();
            for row in rows {
                let (
                    id_str,
                    _status_str,
                    uploaded_at_str,
                    ingested_at_str,
                    doc_kind_str,
                    ingestion_status,
                    title,
                    filename,
                    file_size_bytes,
                    stored_path,
                    page_count,
                    ingestion_error,
                    parse_dt,
                ) = row.map_err(|e| GuideError::Internal(e.to_string()))?;

                result.push(GlobalDocument {
                    id: Uuid::parse_str(&id_str)
                        .map_err(|e| GuideError::Internal(e.to_string()))?,
                    title,
                    filename,
                    file_size_bytes,
                    stored_path,
                    page_count,
                    document_kind: doc_kind_str
                        .as_deref()
                        .map(parse_doc_kind)
                        .unwrap_or(guide_core::models::DocumentKind::DmGuide),
                    ingestion_status,
                    ingestion_error,
                    ingestion_progress: None,
                    uploaded_at: parse_dt(&uploaded_at_str),
                    ingested_at: ingested_at_str.as_deref().map(parse_dt),
                });
            }
            Ok(result)
        })
        .await
    }
}
