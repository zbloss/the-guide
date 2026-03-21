use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArcStatus {
    #[default]
    Open,
    Resolved,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum StoryEventType {
    #[default]
    Combat,
    Social,
    Revelation,
    Travel,
    Rest,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorySignificance {
    #[default]
    Minor,
    Major,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubplotStatus {
    #[default]
    Open,
    Resolved,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArcPoint {
    pub description: String,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MonsterHint {
    pub name: String,
    pub count: Option<i32>,
    pub cr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StoryArc {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub source_doc_id: Uuid,
    pub title: String,
    pub description: String,
    pub arc_order: i32,
    pub status: ArcStatus,
    pub dm_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StoryEvent {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub arc_id: Option<Uuid>,
    pub source_doc_id: Uuid,
    pub title: String,
    pub description: String,
    pub event_type: StoryEventType,
    pub significance: StorySignificance,
    pub location: Option<String>,
    pub involved_characters: Vec<String>,
    pub event_order: i32,
    pub is_dm_only: bool,
    pub dm_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StorySubplot {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub arc_id: Option<Uuid>,
    pub source_doc_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: SubplotStatus,
    pub dm_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CharacterArc {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub character_name: String,
    pub character_id: Option<Uuid>,
    pub source_doc_id: Uuid,
    pub description: String,
    pub arc_points: Vec<ArcPoint>,
    pub dm_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PrepopulatedEncounter {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub story_event_id: Option<Uuid>,
    pub source_doc_id: Uuid,
    pub name: String,
    pub description: String,
    pub location: Option<String>,
    pub difficulty_hint: Option<String>,
    pub monsters: Vec<MonsterHint>,
    pub dm_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// LLM output shape for story extraction
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoryExtractionResult {
    #[serde(default)]
    pub arcs: Vec<StoryArcInput>,
    #[serde(default)]
    pub events: Vec<StoryEventInput>,
    #[serde(default)]
    pub subplots: Vec<StorySubplotInput>,
    #[serde(default)]
    pub character_arcs: Vec<CharacterArcInput>,
    #[serde(default)]
    pub encounters: Vec<PrepopulatedEncounterInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryArcInput {
    pub title: String,
    pub description: String,
    pub arc_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryEventInput {
    pub title: String,
    pub description: String,
    pub event_type: StoryEventType,
    pub significance: StorySignificance,
    pub location: Option<String>,
    pub involved_characters: Vec<String>,
    pub event_order: i32,
    /// Used to link to arc by title
    pub arc_title: Option<String>,
    pub is_dm_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorySubplotInput {
    pub title: String,
    pub description: String,
    pub arc_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterArcInput {
    pub character_name: String,
    pub description: String,
    pub arc_points: Vec<ArcPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepopulatedEncounterInput {
    pub name: String,
    pub description: String,
    pub location: Option<String>,
    pub difficulty_hint: Option<String>,
    pub monsters: Vec<MonsterHint>,
    /// Used to link to event by title
    pub story_event_title: Option<String>,
}
