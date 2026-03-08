use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CampaignDocument, DocumentKind, GlobalDocument, IngestionStatus},
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
        let doc_kind_str = doc_kind_to_str(&doc.document_kind);
        sqlx::query(
            "INSERT INTO campaign_documents \
             (id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
              document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(doc.id.to_string())
        .bind(doc.campaign_id.to_string())
        .bind(&doc.filename)
        .bind(doc.file_size_bytes)
        .bind(&doc.stored_path)
        .bind(doc.page_count)
        .bind(doc_kind_str)
        .bind(ingestion_status_to_str(&doc.ingestion_status))
        .bind(doc.ingestion_error.as_deref())
        .bind(doc.uploaded_at.to_rfc3339())
        .bind(doc.ingested_at.map(|t| t.to_rfc3339()))
        .execute(self.pool)
        .await?;

        Ok(doc.clone())
    }

    pub async fn get_by_id(&self, doc_id: Uuid) -> Result<CampaignDocument> {
        let row = sqlx::query(
            "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
             document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at \
             FROM campaign_documents WHERE id = ?",
        )
        .bind(doc_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("Document {doc_id}")))?;

        row_to_doc(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CampaignDocument>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
             document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at \
             FROM campaign_documents WHERE campaign_id = ? ORDER BY uploaded_at DESC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

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
        .await?;
        Ok(())
    }

    pub async fn update_ingested(&self, doc_id: Uuid, page_count: Option<i32>) -> Result<()> {
        sqlx::query(
            "UPDATE campaign_documents \
             SET ingestion_status = 'completed', ingested_at = ?, page_count = ?, ingestion_error = NULL \
             WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(page_count)
        .bind(doc_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

pub struct GlobalDocumentRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> GlobalDocumentRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, doc: &GlobalDocument) -> Result<GlobalDocument> {
        sqlx::query(
            "INSERT INTO global_documents \
             (id, title, filename, file_size_bytes, stored_path, page_count, \
              ingestion_status, ingestion_error, uploaded_at, ingested_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(doc.id.to_string())
        .bind(&doc.title)
        .bind(&doc.filename)
        .bind(doc.file_size_bytes)
        .bind(&doc.stored_path)
        .bind(doc.page_count)
        .bind(ingestion_status_to_str(&doc.ingestion_status))
        .bind(doc.ingestion_error.as_deref())
        .bind(doc.uploaded_at.to_rfc3339())
        .bind(doc.ingested_at.map(|t| t.to_rfc3339()))
        .execute(self.pool)
        .await?;
        Ok(doc.clone())
    }

    pub async fn get_by_id(&self, doc_id: Uuid) -> Result<GlobalDocument> {
        let row = sqlx::query(
            "SELECT id, title, filename, file_size_bytes, stored_path, page_count, \
             ingestion_status, ingestion_error, uploaded_at, ingested_at \
             FROM global_documents WHERE id = ?",
        )
        .bind(doc_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("Global document {doc_id}")))?;

        row_to_global_doc(row)
    }

    pub async fn list_all(&self) -> Result<Vec<GlobalDocument>> {
        let rows = sqlx::query(
            "SELECT id, title, filename, file_size_bytes, stored_path, page_count, \
             ingestion_status, ingestion_error, uploaded_at, ingested_at \
             FROM global_documents ORDER BY uploaded_at DESC",
        )
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_global_doc).collect()
    }

    pub async fn update_status(
        &self,
        doc_id: Uuid,
        status: &IngestionStatus,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE global_documents SET ingestion_status = ?, ingestion_error = ? WHERE id = ?",
        )
        .bind(ingestion_status_to_str(status))
        .bind(error)
        .bind(doc_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_ingested(&self, doc_id: Uuid, page_count: Option<i32>) -> Result<()> {
        sqlx::query(
            "UPDATE global_documents \
             SET ingestion_status = 'completed', ingested_at = ?, page_count = ?, ingestion_error = NULL \
             WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(page_count)
        .bind(doc_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

fn row_to_doc(row: SqliteRow) -> Result<CampaignDocument> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let status_str: String = row.try_get("ingestion_status")?;
    let doc_kind_str: Option<String> = row.try_get("document_kind").ok();
    let uploaded_at_str: String = row.try_get("uploaded_at")?;
    let ingested_at_str: Option<String> = row.try_get("ingested_at")?;

    Ok(CampaignDocument {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        filename: row.try_get("filename")?,
        file_size_bytes: row.try_get("file_size_bytes")?,
        stored_path: row.try_get("stored_path")?,
        page_count: row.try_get("page_count")?,
        document_kind: doc_kind_str
            .as_deref()
            .map(parse_doc_kind)
            .unwrap_or_default(),
        ingestion_status: str_to_ingestion_status(&status_str),
        ingestion_error: row.try_get("ingestion_error")?,
        uploaded_at: parse_dt(&uploaded_at_str),
        ingested_at: ingested_at_str.as_deref().map(parse_dt),
    })
}

fn row_to_global_doc(row: SqliteRow) -> Result<GlobalDocument> {
    let id_str: String = row.try_get("id")?;
    let status_str: String = row.try_get("ingestion_status")?;
    let uploaded_at_str: String = row.try_get("uploaded_at")?;
    let ingested_at_str: Option<String> = row.try_get("ingested_at")?;

    Ok(GlobalDocument {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        title: row.try_get("title")?,
        filename: row.try_get("filename")?,
        file_size_bytes: row.try_get("file_size_bytes")?,
        stored_path: row.try_get("stored_path")?,
        page_count: row.try_get("page_count")?,
        ingestion_status: str_to_ingestion_status(&status_str),
        ingestion_error: row.try_get("ingestion_error")?,
        uploaded_at: parse_dt(&uploaded_at_str),
        ingested_at: ingested_at_str.as_deref().map(parse_dt),
    })
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn doc_kind_to_str(k: &DocumentKind) -> &'static str {
    match k {
        DocumentKind::Campaign => "campaign",
        DocumentKind::Rulebook => "rulebook",
    }
}

fn parse_doc_kind(s: &str) -> DocumentKind {
    match s {
        "rulebook" => DocumentKind::Rulebook,
        _ => DocumentKind::Campaign,
    }
}

fn ingestion_status_to_str(s: &IngestionStatus) -> &'static str {
    match s {
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
