use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Condition, EncounterStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encounter {
    pub id: Uuid,
    pub session_id: Uuid,
    pub campaign_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: EncounterStatus,
    pub round: i32,
    /// Index into `participants` (sorted by initiative) for the current turn.
    pub current_turn_index: i32,
    pub participants: Vec<CombatParticipant>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatParticipant {
    pub id: Uuid,
    pub encounter_id: Uuid,
    pub character_id: Uuid,
    pub name: String,
    /// The raw d20 roll (stored for transparency)
    pub initiative_roll: i32,
    /// Dexterity modifier applied
    pub initiative_modifier: i32,
    /// initiative_roll + initiative_modifier
    pub initiative_total: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub armor_class: i32,
    pub conditions: Vec<Condition>,
    pub action_budget: ActionBudget,
    pub has_taken_turn: bool,
    pub is_defeated: bool,
}

/// Tracks what a combatant can still do on their turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBudget {
    /// Standard action (Attack, Dash, Dodge, Help, Hide, Ready, Search, Use Object, Cast a spell)
    pub has_action: bool,
    /// Bonus action (class features, certain spells, off-hand attack, etc.)
    pub has_bonus_action: bool,
    /// Reaction (opportunity attack, shield spell, etc.) — resets at start of each turn
    pub has_reaction: bool,
    /// Remaining movement in feet
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

// ── Request/Response types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEncounterRequest {
    pub session_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub participant_character_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddParticipantRequest {
    pub character_id: Uuid,
    pub initiative_roll: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterSummary {
    pub encounter: Encounter,
    pub current_participant: Option<CombatParticipant>,
    pub round: i32,
}
