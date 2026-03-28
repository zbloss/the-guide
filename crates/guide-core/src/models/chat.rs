use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatMessage {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
