use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CharacterRelationship {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub from_character_id: Uuid,
    pub to_character_id: Uuid,
    pub relationship_type: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateRelationshipRequest {
    pub from_character_id: Uuid,
    pub to_character_id: Uuid,
    pub relationship_type: String,
    pub notes: Option<String>,
}
