use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TemplateParticipant {
    pub name: String,
    pub max_hp: i32,
    pub armor_class: i32,
    pub speed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EncounterTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub participants: Vec<TemplateParticipant>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SaveTemplateRequest {
    pub name: Option<String>,
}
