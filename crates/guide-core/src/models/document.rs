use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::IngestionStatus;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    #[default]
    Campaign,
    Rulebook,
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
    pub ingestion_status: IngestionStatus,
    pub ingestion_error: Option<String>,
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
    pub ingestion_status: IngestionStatus,
    pub ingestion_error: Option<String>,
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
