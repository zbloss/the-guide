use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use guide_core::{
    models::{CampaignDocument, DocumentKind, GlobalDocument, IngestionStatus},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

#[derive(Serialize)]
pub struct PageOcrRow {
    pub page_num: i64,
    pub raw_text: String,
}

pub struct DocumentRepository {
    pool: DuckDbPool,
}

impl DocumentRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn insert(&self, doc: &CampaignDocument) -> Result<CampaignDocument> {
        let doc_kind_str = doc_kind_to_str(&doc.document_kind).to_string();
        let status_str = ingestion_status_to_str(&doc.ingestion_status).to_string();
        let id_str = doc.id.to_string();
        let campaign_id_str = doc.campaign_id.to_string();
        let filename = doc.filename.clone();
        let file_size = doc.file_size_bytes;
        let stored_path = doc.stored_path.clone();
        let page_count = doc.page_count;
        let description = doc.description.clone();
        let ingestion_error = doc.ingestion_error.clone();
        let story_status = doc.story_extraction_status.clone();
        let story_error = doc.story_extraction_error.clone();
        let uploaded_at = doc.uploaded_at.to_rfc3339();
        let ingested_at = doc.ingested_at.map(|t| t.to_rfc3339());

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO campaign_documents \
                 (id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
                  document_kind, description, ingestion_status, ingestion_error, \
                  story_extraction_status, story_extraction_error, uploaded_at, ingested_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, filename, file_size, stored_path, page_count,
                    doc_kind_str, description, status_str, ingestion_error,
                    story_status, story_error, uploaded_at, ingested_at
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        Ok(doc.clone())
    }

    pub async fn get_by_id(&self, doc_id: Uuid) -> Result<CampaignDocument> {
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
                 document_kind, description, ingestion_status, ingestion_error, \
                 story_extraction_status, story_extraction_error, uploaded_at, ingested_at \
                 FROM campaign_documents WHERE id = ?",
                [&id_str],
                row_to_doc,
                format!("Document {doc_id}"),
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CampaignDocument>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count, \
                 document_kind, description, ingestion_status, ingestion_error, \
                 story_extraction_status, story_extraction_error, uploaded_at, ingested_at \
                 FROM campaign_documents WHERE campaign_id = ? ORDER BY uploaded_at DESC",
                [&id_str],
                row_to_doc,
            )
        })
        .await
    }

    pub async fn update_status(
        &self,
        doc_id: Uuid,
        status: &IngestionStatus,
        error: Option<&str>,
    ) -> Result<()> {
        let status_str = ingestion_status_to_str(status).to_string();
        let id_str = doc_id.to_string();
        let error = error.map(|s| s.to_string());
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE campaign_documents SET ingestion_status = ?, ingestion_error = ? WHERE id = ?",
                duckdb::params![status_str, error, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn update_ingested(&self, doc_id: Uuid, page_count: Option<i32>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE campaign_documents \
                 SET ingestion_status = 'completed', ingested_at = ?, page_count = ?, ingestion_error = NULL \
                 WHERE id = ?",
                duckdb::params![now, page_count, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn update_story_extraction_progress(&self, doc_id: Uuid, progress: &str) -> Result<()> {
        let progress = progress.to_string();
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE campaign_documents SET story_extraction_progress = ? WHERE id = ?",
                duckdb::params![progress, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn update_story_extraction_status(
        &self,
        doc_id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let status = status.to_string();
        let error = error.map(|s| s.to_string());
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE campaign_documents \
                 SET story_extraction_status = ?, story_extraction_error = ? WHERE id = ?",
                duckdb::params![status, error, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn reset_stuck_processing(&self) -> Result<()> {
        with_db(&self.pool, |conn| {
            conn.execute(
                "UPDATE campaign_documents SET ingestion_status = 'failed', \
                 ingestion_error = 'Interrupted by server restart' \
                 WHERE ingestion_status = 'processing'",
                [],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn delete(&self, doc_id: Uuid) -> Result<()> {
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM campaign_documents WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Document {doc_id}")));
            }
            Ok(())
        })
        .await
    }

    pub async fn save_page_ocr(&self, doc_id: Uuid, pages: &[(u32, String)]) -> Result<()> {
        let doc_id_str = doc_id.to_string();
        let pages: Vec<(String, i64, String)> = pages
            .iter()
            .map(|(page_num, raw_text)| {
                (
                    Uuid::new_v4().to_string(),
                    *page_num as i64,
                    raw_text.clone(),
                )
            })
            .collect();

        with_db(&self.pool, move |conn| {
            // Delete existing pages for this document before re-inserting (idempotent ingest).
            conn.execute(
                "DELETE FROM document_page_ocr WHERE document_id = ?",
                duckdb::params![doc_id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;

            for (row_id, page_num, raw_text) in &pages {
                conn.execute(
                    "INSERT INTO document_page_ocr \
                     (id, document_id, page_num, raw_text) \
                     VALUES (?, ?, ?, ?)",
                    duckdb::params![row_id, doc_id_str, page_num, raw_text],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            }
            Ok(())
        })
        .await
    }

    pub async fn list_page_ocr(&self, doc_id: Uuid) -> Result<Vec<PageOcrRow>> {
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT page_num, raw_text \
                 FROM document_page_ocr WHERE document_id = ? ORDER BY page_num ASC",
                [&id_str],
                |row| {
                    let page_num: i64 = row.get("page_num")?;
                    let raw_text: String = row.get("raw_text")?;
                    Ok(PageOcrRow {
                        page_num,
                        raw_text,
                    })
                },
            )
        })
        .await
    }
}

pub struct GlobalDocumentRepository {
    pool: DuckDbPool,
}

impl GlobalDocumentRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn insert(&self, doc: &GlobalDocument) -> Result<GlobalDocument> {
        let doc_kind_str = doc_kind_to_str(&doc.document_kind).to_string();
        let id_str = doc.id.to_string();
        let title = doc.title.clone();
        let filename = doc.filename.clone();
        let file_size = doc.file_size_bytes;
        let stored_path = doc.stored_path.clone();
        let page_count = doc.page_count;
        let status_str = ingestion_status_to_str(&doc.ingestion_status).to_string();
        let ingestion_error = doc.ingestion_error.clone();
        let uploaded_at = doc.uploaded_at.to_rfc3339();
        let ingested_at = doc.ingested_at.map(|t| t.to_rfc3339());

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO global_documents \
                 (id, title, filename, file_size_bytes, stored_path, page_count, \
                  document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    id_str, title, filename, file_size, stored_path, page_count,
                    doc_kind_str, status_str, ingestion_error, uploaded_at, ingested_at
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;
        Ok(doc.clone())
    }

    pub async fn get_by_id(&self, doc_id: Uuid) -> Result<GlobalDocument> {
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, title, filename, file_size_bytes, stored_path, page_count, \
                 document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at \
                 FROM global_documents WHERE id = ?",
                [&id_str],
                row_to_global_doc,
                format!("Global document {doc_id}"),
            )
        })
        .await
    }

    pub async fn list_all(&self) -> Result<Vec<GlobalDocument>> {
        with_db(&self.pool, |conn| {
            query_all(
                conn,
                "SELECT id, title, filename, file_size_bytes, stored_path, page_count, \
                 document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at \
                 FROM global_documents ORDER BY uploaded_at DESC",
                [],
                row_to_global_doc,
            )
        })
        .await
    }

    pub async fn list_by_kind(&self, kind: &DocumentKind) -> Result<Vec<GlobalDocument>> {
        let kind_str = doc_kind_to_str(kind).to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, title, filename, file_size_bytes, stored_path, page_count, \
                 document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at \
                 FROM global_documents WHERE document_kind = ? ORDER BY uploaded_at DESC",
                [&kind_str],
                row_to_global_doc,
            )
        })
        .await
    }

    pub async fn update_status(
        &self,
        doc_id: Uuid,
        status: &IngestionStatus,
        error: Option<&str>,
    ) -> Result<()> {
        let status_str = ingestion_status_to_str(status).to_string();
        let id_str = doc_id.to_string();
        let error = error.map(|s| s.to_string());
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE global_documents SET ingestion_status = ?, ingestion_error = ? WHERE id = ?",
                duckdb::params![status_str, error, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn update_ingested(&self, doc_id: Uuid, page_count: Option<i32>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE global_documents \
                 SET ingestion_status = 'completed', ingested_at = ?, page_count = ?, ingestion_error = NULL \
                 WHERE id = ?",
                duckdb::params![now, page_count, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn reset_stuck_processing(&self) -> Result<()> {
        with_db(&self.pool, |conn| {
            conn.execute(
                "UPDATE global_documents SET ingestion_status = 'failed', \
                 ingestion_error = 'Interrupted by server restart' \
                 WHERE ingestion_status = 'processing'",
                [],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn delete(&self, doc_id: Uuid) -> Result<()> {
        let id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM global_documents WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Global document {doc_id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_doc(row: &duckdb::Row) -> duckdb::Result<CampaignDocument> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let status_str: String = row.get("ingestion_status")?;
    let doc_kind_str: Option<String> = row.get("document_kind").ok();
    let uploaded_at_str: String = row.get("uploaded_at")?;
    let ingested_at_str: Option<String> = row.get("ingested_at")?;
    let story_extraction_status: String = row
        .get::<_, String>("story_extraction_status")
        .unwrap_or_else(|_| "pending".to_string());
    let story_extraction_error: Option<String> = row.get("story_extraction_error").unwrap_or(None);

    Ok(CampaignDocument {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        filename: row.get("filename")?,
        file_size_bytes: row.get("file_size_bytes")?,
        stored_path: row.get("stored_path")?,
        page_count: row.get("page_count")?,
        document_kind: doc_kind_str.as_deref().map(parse_doc_kind).unwrap_or_default(),
        description: row.get("description").unwrap_or(None),
        ingestion_status: str_to_ingestion_status(&status_str),
        ingestion_error: row.get("ingestion_error")?,
        story_extraction_status,
        story_extraction_error,
        story_extraction_progress: row.get("story_extraction_progress").unwrap_or(None),
        ingestion_progress: row.get("ingestion_progress").unwrap_or(None),
        uploaded_at: parse_dt(&uploaded_at_str),
        ingested_at: ingested_at_str.as_deref().map(parse_dt),
    })
}

fn row_to_global_doc(row: &duckdb::Row) -> duckdb::Result<GlobalDocument> {
    let id_str: String = row.get("id")?;
    let status_str: String = row.get("ingestion_status")?;
    let uploaded_at_str: String = row.get("uploaded_at")?;
    let ingested_at_str: Option<String> = row.get("ingested_at")?;
    let doc_kind_str: Option<String> = row.get("document_kind").ok();

    Ok(GlobalDocument {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        title: row.get("title")?,
        filename: row.get("filename")?,
        file_size_bytes: row.get("file_size_bytes")?,
        stored_path: row.get("stored_path")?,
        page_count: row.get("page_count")?,
        document_kind: doc_kind_str
            .as_deref()
            .map(parse_doc_kind)
            .unwrap_or(DocumentKind::DmGuide),
        ingestion_status: str_to_ingestion_status(&status_str),
        ingestion_error: row.get("ingestion_error")?,
        ingestion_progress: row.get("ingestion_progress").unwrap_or(None),
        uploaded_at: parse_dt(&uploaded_at_str),
        ingested_at: ingested_at_str.as_deref().map(parse_dt),
    })
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

pub(crate) fn doc_kind_to_str(k: &DocumentKind) -> &'static str {
    match k {
        DocumentKind::DmGuide => "dm_guide",
        DocumentKind::MonsterManual => "monster_manual",
        DocumentKind::Srd => "srd",
        DocumentKind::Campaign => "campaign",
        DocumentKind::Supplemental => "supplemental",
    }
}

pub fn parse_doc_kind(s: &str) -> DocumentKind {
    match s {
        "dm_guide" => DocumentKind::DmGuide,
        "monster_manual" => DocumentKind::MonsterManual,
        "srd" => DocumentKind::Srd,
        "supplemental" => DocumentKind::Supplemental,
        "rulebook" => DocumentKind::DmGuide,
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
