use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{IngestionStatus, LoreType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignDocument {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub filename: String,
    pub file_size_bytes: i64,
    /// Path to the file on disk (relative to data dir)
    pub stored_path: String,
    pub page_count: Option<i32>,
    pub ingestion_status: IngestionStatus,
    pub ingestion_error: Option<String>,
    pub uploaded_at: DateTime<Utc>,
    pub ingested_at: Option<DateTime<Utc>>,
}

/// A chunk of lore extracted from a document or session and stored in Qdrant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLore {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub source_document_id: Option<Uuid>,
    pub source_session_id: Option<Uuid>,
    pub lore_type: LoreType,
    pub content: String,
    pub is_player_visible: bool,
    pub significance: LoreSignificance,
    pub entities: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreSignificance {
    Minor,
    Major,
    Milestone,
}

impl Default for LoreSignificance {
    fn default() -> Self {
        LoreSignificance::Minor
    }
}
