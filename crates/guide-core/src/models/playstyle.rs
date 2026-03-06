use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tracks observed party playstyle preferences across sessions.
/// Updated by session event analysis; used to bias encounter generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaystyleProfile {
    pub campaign_id: Uuid,
    /// 0.0–1.0: proportion of time spent in combat
    pub combat_affinity: f32,
    /// 0.0–1.0: proportion of time spent in social/roleplay encounters
    pub social_affinity: f32,
    /// 0.0–1.0: proportion of time spent exploring
    pub exploration_affinity: f32,
    /// Preferred encounter difficulty (0=trivial, 1=deadly)
    pub preferred_difficulty: f32,
    /// Total sessions analysed
    pub sessions_sampled: u32,
    pub updated_at: DateTime<Utc>,
}

impl PlaystyleProfile {
    pub fn default_for(campaign_id: Uuid) -> Self {
        Self {
            campaign_id,
            combat_affinity: 0.33,
            social_affinity: 0.33,
            exploration_affinity: 0.34,
            preferred_difficulty: 0.5,
            sessions_sampled: 0,
            updated_at: Utc::now(),
        }
    }
}

/// A generated encounter suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedEncounter {
    pub title: String,
    pub description: String,
    pub encounter_type: GeneratedEncounterType,
    pub challenge_rating: Option<f32>,
    pub suggested_enemies: Vec<EnemySuggestion>,
    pub narrative_hook: String,
    pub alternative: Option<String>, // e.g. "Social alternative: negotiate with the bandits"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedEncounterType {
    Combat,
    Social,
    Exploration,
    Puzzle,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemySuggestion {
    pub name: String,
    pub count: u32,
    pub cr: Option<f32>,
}
