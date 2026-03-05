use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::GameSystem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub game_system: GameSystem,
    pub world_state: Option<WorldState>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// High-level persistent world metadata attached to a campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub current_location: Option<String>,
    pub current_date_in_world: Option<String>,
    pub active_quests: Vec<String>,
    pub completed_quests: Vec<String>,
    pub custom_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub description: Option<String>,
    pub game_system: Option<GameSystem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCampaignRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub world_state: Option<WorldState>,
}
