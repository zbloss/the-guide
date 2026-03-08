use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::{CharacterType, Condition};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Character {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub character_type: CharacterType,
    pub class: Option<String>,
    pub race: Option<String>,
    pub level: i32,
    pub max_hp: i32,
    pub current_hp: i32,
    pub armor_class: i32,
    pub speed: i32,
    pub ability_scores: AbilityScores,
    pub conditions: Vec<Condition>,
    pub backstory: Option<Backstory>,
    pub is_alive: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct AbilityScores {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

impl AbilityScores {
    /// D&D 5e modifier formula: floor((score - 10) / 2)
    pub fn modifier(score: i32) -> i32 {
        (score - 10).div_euclid(2)
    }

    pub fn initiative_modifier(&self) -> i32 {
        Self::modifier(self.dexterity)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Backstory {
    pub raw_text: String,
    pub extracted_hooks: Vec<PlotHook>,
    pub motivations: Vec<String>,
    pub key_relationships: Vec<String>,
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PlotHook {
    pub id: Uuid,
    pub character_id: Uuid,
    pub description: String,
    pub priority: HookPriority,
    pub is_active: bool,
    pub llm_extracted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateCharacterRequest {
    pub name: String,
    pub character_type: CharacterType,
    pub class: Option<String>,
    pub race: Option<String>,
    pub level: Option<i32>,
    pub max_hp: i32,
    pub armor_class: i32,
    pub speed: Option<i32>,
    pub ability_scores: Option<AbilityScores>,
    pub backstory_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateCharacterRequest {
    pub current_hp: Option<i32>,
    pub conditions: Option<Vec<Condition>>,
    pub is_alive: Option<bool>,
}
