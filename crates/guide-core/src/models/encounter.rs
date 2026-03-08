use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::{Condition, EncounterStatus};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Encounter {
    pub id: Uuid,
    pub session_id: Uuid,
    pub campaign_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: EncounterStatus,
    pub round: i32,
    pub current_turn_index: i32,
    pub participants: Vec<CombatParticipant>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CombatParticipant {
    pub id: Uuid,
    pub encounter_id: Uuid,
    pub character_id: Uuid,
    pub name: String,
    pub initiative_roll: i32,
    pub initiative_modifier: i32,
    pub initiative_total: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub armor_class: i32,
    pub conditions: Vec<Condition>,
    pub action_budget: ActionBudget,
    pub has_taken_turn: bool,
    pub is_defeated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActionBudget {
    pub has_action: bool,
    pub has_bonus_action: bool,
    pub has_reaction: bool,
    pub movement_remaining: i32,
}

impl ActionBudget {
    pub fn new(speed: i32) -> Self {
        Self {
            has_action: true,
            has_bonus_action: true,
            has_reaction: true,
            movement_remaining: speed,
        }
    }

    pub fn reset(&mut self, speed: i32) {
        self.has_action = true;
        self.has_bonus_action = true;
        self.has_reaction = true;
        self.movement_remaining = speed;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateEncounterRequest {
    pub session_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub participant_character_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AddParticipantRequest {
    pub character_id: Uuid,
    pub initiative_roll: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateParticipantRequest {
    pub hp_delta: Option<i32>,
    pub set_hp: Option<i32>,
    pub add_condition: Option<Condition>,
    pub remove_condition: Option<Condition>,
    pub spend_action: Option<bool>,
    pub spend_bonus_action: Option<bool>,
    pub spend_reaction: Option<bool>,
    pub spend_movement: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EncounterSummary {
    pub encounter: Encounter,
    pub current_participant: Option<CombatParticipant>,
    pub round: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GeneratedEncounter {
    pub title: String,
    pub description: String,
    pub encounter_type: GeneratedEncounterType,
    pub challenge_rating: Option<f32>,
    pub suggested_enemies: Vec<EnemySuggestion>,
    pub narrative_hook: String,
    pub alternative: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedEncounterType {
    Combat,
    Social,
    Exploration,
    Puzzle,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EnemySuggestion {
    pub name: String,
    pub count: u32,
    pub cr: Option<f32>,
}
