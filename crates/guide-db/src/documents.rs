use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CampaignDocument, IngestionStatus},
    GuideError, Result,
};

pub struct DocumentRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> DocumentRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, doc: &CampaignDocument) -> Result<CampaignDocument> {
        sqlx::query(
            "INSERT INTO campaign_documents \
             (id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
              ingestion_status, ingestion_error, uploaded_at, ingested_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(doc.id.to_string())
        .bind(doc.campaign_id.to_string())
        .bind(&doc.filename)
        .bind(doc.file_size_bytes)
        .bind(&doc.stored_path)
        .bind(doc.page_count)
        .bind(ingestion_status_to_str(&doc.ingestion_status))
        .bind(doc.ingestion_error.as_deref())
        .bind(doc.uploaded_at.to_rfc3339())
        .bind(doc.ingested_at.map(|t| t.to_rfc3339()))
        .execute(self.pool)
        .await
        .map_err(|e| GuideError::Database(e.to_string()))?;

        Ok(doc.clone())
    }

    pub async fn get_by_id(&self, doc_id: Uuid) -> Result<CampaignDocument> {
        let row = sqlx::query(
            "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
             ingestion_status, ingestion_error, uploaded_at, ingested_at \
             FROM campaign_documents WHERE id = ?",
        )
        .bind(doc_id.to_string())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| GuideError::Database(e.to_string()))?
        .ok_or_else(|| GuideError::NotFound(format!("Document {doc_id} not found")))?;

        row_to_doc(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CampaignDocument>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
             ingestion_status, ingestion_error, uploaded_at, ingested_at \
             FROM campaign_documents WHERE campaign_id = ? ORDER BY uploaded_at DESC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await
        .map_err(|e| GuideError::Database(e.to_string()))?;

        rows.into_iter().map(row_to_doc).collect()
    }

    pub async fn update_status(
        &self,
        doc_id: Uuid,
        status: &IngestionStatus,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE campaign_documents SET ingestion_status = ?, ingestion_error = ? WHERE id = ?",
        )
        .bind(ingestion_status_to_str(status))
        .bind(error)
        .bind(doc_id.to_string())
        .execute(self.pool)
        .await
        .map_err(|e| GuideError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn update_ingested(&self, doc_id: Uuid, page_count: Option<i32>) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE campaign_documents \
             SET ingestion_status = 'completed', ingested_at = ?, page_count = ?, ingestion_error = NULL \
             WHERE id = ?",
        )
        .bind(now.to_rfc3339())
        .bind(page_count)
        .bind(doc_id.to_string())
        .execute(self.pool)
        .await
        .map_err(|e| GuideError::Database(e.to_string()))?;

        Ok(())
    }
}

fn row_to_doc(row: sqlx::sqlite::SqliteRow) -> Result<CampaignDocument> {
    let id_str: String = row.try_get("id").map_err(|e| GuideError::Database(e.to_string()))?;
    let campaign_id_str: String =
        row.try_get("campaign_id").map_err(|e| GuideError::Database(e.to_string()))?;
    let status_str: String =
        row.try_get("ingestion_status").map_err(|e| GuideError::Database(e.to_string()))?;
    let uploaded_at_str: String =
        row.try_get("uploaded_at").map_err(|e| GuideError::Database(e.to_string()))?;
    let ingested_at_str: Option<String> =
        row.try_get("ingested_at").map_err(|e| GuideError::Database(e.to_string()))?;

    Ok(CampaignDocument {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Database(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Database(e.to_string()))?,
        filename: row.try_get("filename").map_err(|e| GuideError::Database(e.to_string()))?,
        file_size_bytes: row
            .try_get("file_size_bytes")
            .map_err(|e| GuideError::Database(e.to_string()))?,
        stored_path: row
            .try_get("stored_path")
            .map_err(|e| GuideError::Database(e.to_string()))?,
        page_count: row
            .try_get("page_count")
            .map_err(|e| GuideError::Database(e.to_string()))?,
        ingestion_status: str_to_ingestion_status(&status_str),
        ingestion_error: row
            .try_get("ingestion_error")
            .map_err(|e| GuideError::Database(e.to_string()))?,
        uploaded_at: chrono::DateTime::parse_from_rfc3339(&uploaded_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| GuideError::Database(e.to_string()))?,
        ingested_at: ingested_at_str
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| GuideError::Database(e.to_string()))
            })
            .transpose()?,
    })
}

fn ingestion_status_to_str(status: &IngestionStatus) -> &'static str {
    match status {
        IngestionStatus::Pending => "pending",
        IngestionStatus::Processing => "processing",
        IngestionStatus::Completed => "completed",
        IngestionStatus::Failed => "failed",
    }
}

fn str_to_ingestion_status(s: &str) -> IngestionStatus {
    match s {
        "processing" => IngestionStatus::Processing,
        "completed" => IngestionStatus::Completed,
        "failed" => IngestionStatus::Failed,
        _ => IngestionStatus::Pending,
    }
}
