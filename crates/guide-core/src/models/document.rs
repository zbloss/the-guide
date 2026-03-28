use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::IngestionStatus;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    DmGuide,
    MonsterManual,
    Srd,
    #[default]
    Campaign,
    Supplemental,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CampaignDocument {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub filename: String,
    pub file_size_bytes: i64,
    pub stored_path: String,
    pub page_count: Option<i32>,
    pub document_kind: DocumentKind,
    pub description: Option<String>,
    pub ingestion_status: IngestionStatus,
    pub ingestion_error: Option<String>,
    pub story_extraction_status: String,
    pub story_extraction_error: Option<String>,
    pub ingestion_progress: Option<String>,
    pub uploaded_at: DateTime<Utc>,
    pub ingested_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GlobalDocument {
    pub id: Uuid,
    pub title: String,
    pub filename: String,
    pub file_size_bytes: i64,
    pub stored_path: String,
    pub page_count: Option<i32>,
    pub document_kind: DocumentKind,
    pub ingestion_status: IngestionStatus,
    pub ingestion_error: Option<String>,
    pub ingestion_progress: Option<String>,
    pub uploaded_at: DateTime<Utc>,
    pub ingested_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RankedChunk {
    pub content: String,
    pub section_path: String,
    pub doc_title: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DocSummary {
    pub doc_id: Uuid,
    pub doc_name: String,
    pub filename: String,
    pub summary: String,
    pub scope: String,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct MetaIndex {
    pub scope: String,
    pub entries: Vec<DocSummary>,
}

/// A single entry from a document's Table of Contents.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TocEntry {
    pub title: String,
    /// 1-based page number as printed in the document.
    pub page: u32,
    /// Nesting depth: 0 = top-level chapter, 1 = section, 2 = subsection, …
    pub depth: u8,
}

/// A single entry from a document's back-of-book Index.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IndexEntry {
    pub term: String,
    /// Page numbers where this term appears.
    pub pages: Vec<u32>,
}
