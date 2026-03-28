use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::{EventSignificance, EventType};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Session {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub session_number: i32,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub map_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionEvent {
    pub id: Uuid,
    pub session_id: Uuid,
    pub campaign_id: Uuid,
    pub event_type: EventType,
    pub description: String,
    pub significance: EventSignificance,
    pub involved_character_ids: Vec<Uuid>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub content: String,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateSessionEventRequest {
    pub event_type: EventType,
    pub description: String,
    pub significance: Option<EventSignificance>,
    pub involved_character_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImprovPromptResponse {
    pub options: Vec<String>,
}
