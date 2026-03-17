use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrepType {
    SessionRecap,
    StorySoFar,
    StoryAhead,
    CharacterRoadmap,
}

impl PrepType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrepType::SessionRecap => "session_recap",
            PrepType::StorySoFar => "story_so_far",
            PrepType::StoryAhead => "story_ahead",
            PrepType::CharacterRoadmap => "character_roadmap",
        }
    }
}

impl FromStr for PrepType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "session_recap" => Ok(PrepType::SessionRecap),
            "story_so_far" => Ok(PrepType::StorySoFar),
            "story_ahead" => Ok(PrepType::StoryAhead),
            "character_roadmap" => Ok(PrepType::CharacterRoadmap),
            other => Err(format!("Unknown prep type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DmPrepResult {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub prep_type: PrepType,
    pub content: String,
    pub character_id: Option<Uuid>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionRecapRequest {
    pub force_regenerate: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StoryContextRequest {
    pub current_chapter: Option<String>,
    pub force_regenerate: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CharacterRoadmapRequest {
    pub current_chapter: Option<String>,
    pub force_regenerate: Option<bool>,
}
