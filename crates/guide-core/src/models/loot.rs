use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LootItem {
    pub id: Uuid,
    pub session_id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub item_type: String,
    pub quantity: i32,
    pub value_gp: f64,
    pub assigned_to_char_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateLootItemRequest {
    pub name: String,
    pub item_type: Option<String>,
    pub quantity: Option<i32>,
    pub value_gp: Option<f64>,
    pub assigned_to_char_id: Option<Uuid>,
    pub notes: Option<String>,
}
