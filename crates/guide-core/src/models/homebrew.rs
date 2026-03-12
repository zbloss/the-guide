use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HomebrewRule {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub title: String,
    pub description: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateHomebrewRuleRequest {
    pub title: String,
    pub description: String,
    pub category: Option<String>,
}
