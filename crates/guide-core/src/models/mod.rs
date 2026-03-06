pub mod campaign;
pub mod character;
pub mod document;
pub mod encounter;
pub mod playstyle;
pub mod session;

pub use campaign::*;
pub use character::*;
pub use document::*;
pub use encounter::*;
pub use playstyle::*;
pub use session::*;

use serde::{Deserialize, Serialize};

// ── Shared enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameSystem {
    Dnd5e,
    Pathfinder2e,
    Custom(String),
}

impl Default for GameSystem {
    fn default() -> Self {
        GameSystem::Dnd5e
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterType {
    Pc,
    Npc,
    Monster,
}

/// D&D 5e conditions (PHB p. 290-293)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Blinded,
    Charmed,
    Deafened,
    Exhaustion(u8), // level 1-6
    Frightened,
    Grappled,
    Incapacitated,
    Invisible,
    Paralyzed,
    Petrified,
    Poisoned,
    Prone,
    Restrained,
    Stunned,
    Unconscious,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncounterStatus {
    Pending,
    Active,
    Completed,
    Fled,
}

impl Default for EncounterStatus {
    fn default() -> Self {
        EncounterStatus::Pending
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreType {
    Npc,
    Location,
    Item,
    Plot,
    Mechanic,
    Backstory,
    SessionEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Combat,
    Exploration,
    Social,
    Rest,
    LevelUp,
    ItemFound,
    NpcMet,
    PlotRevealed,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSignificance {
    Minor,
    Major,
    Milestone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl Default for IngestionStatus {
    fn default() -> Self {
        IngestionStatus::Pending
    }
}

/// Who is viewing the information — controls spoiler filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perspective {
    Dm,
    Player,
}
