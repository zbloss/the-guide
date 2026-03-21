use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::{CharacterType, Condition};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct SpellSlot {
    pub level: i32,
    pub total: i32,
    pub remaining: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SpendSlotRequest {
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RestoreSlotRequest {
    pub level: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GenerateNpcRequest {
    pub prompt: String,
}

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
    pub portrait_url: Option<String>,
    pub spell_slots: Vec<SpellSlot>,
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
    pub spell_slots: Option<Vec<SpellSlot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateCharacterRequest {
    pub name: Option<String>,
    pub class: Option<String>,
    pub race: Option<String>,
    pub level: Option<i32>,
    pub max_hp: Option<i32>,
    pub current_hp: Option<i32>,
    pub armor_class: Option<i32>,
    pub speed: Option<i32>,
    pub ability_scores: Option<AbilityScores>,
    pub conditions: Option<Vec<Condition>>,
    pub is_alive: Option<bool>,
    pub backstory_text: Option<String>,
    pub spell_slots: Option<Vec<SpellSlot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct SavingThrows {
    pub strength: Option<i32>,
    pub dexterity: Option<i32>,
    pub constitution: Option<i32>,
    pub intelligence: Option<i32>,
    pub wisdom: Option<i32>,
    pub charisma: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct SkillScores {
    pub acrobatics: Option<i32>,
    pub animal_handling: Option<i32>,
    pub arcana: Option<i32>,
    pub athletics: Option<i32>,
    pub deception: Option<i32>,
    pub history: Option<i32>,
    pub insight: Option<i32>,
    pub intimidation: Option<i32>,
    pub investigation: Option<i32>,
    pub medicine: Option<i32>,
    pub nature: Option<i32>,
    pub perception: Option<i32>,
    pub performance: Option<i32>,
    pub persuasion: Option<i32>,
    pub religion: Option<i32>,
    pub sleight_of_hand: Option<i32>,
    pub stealth: Option<i32>,
    pub survival: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ParsedSheetResult {
    pub name: String,
    pub class: Option<String>,
    pub race: Option<String>,
    pub level: i32,
    pub max_hp: i32,
    pub armor_class: i32,
    pub speed: i32,
    pub ability_scores: AbilityScores,
    pub backstory_text: Option<String>,
    pub raw_extracted_text: String,
    pub parse_confidence: f32,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub experience_points: Option<i32>,
    #[serde(default)]
    pub hit_dice: Option<String>,
    #[serde(default)]
    pub saving_throws: Option<SavingThrows>,
    #[serde(default)]
    pub skills: Option<SkillScores>,
    #[serde(default)]
    pub proficiencies: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub features_and_traits: Vec<String>,
    #[serde(default)]
    pub equipment: Vec<String>,
    #[serde(default)]
    pub personality_traits: Option<String>,
    #[serde(default)]
    pub ideals: Option<String>,
    #[serde(default)]
    pub bonds: Option<String>,
    #[serde(default)]
    pub flaws: Option<String>,
    #[serde(default)]
    pub spell_slots: Vec<SpellSlot>,
}
