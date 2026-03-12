use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CalendarEntry {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub session_id: Option<Uuid>,
    pub in_game_date: String,
    pub real_date: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateCalendarEntryRequest {
    pub session_id: Option<Uuid>,
    pub in_game_date: String,
    pub real_date: String,
    pub notes: Option<String>,
}
