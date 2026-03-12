use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlotHookStatus {
    Open,
    Active,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TrackedPlotHook {
    pub id: Uuid,
    pub character_id: Uuid,
    pub hook_text: String,
    pub status: PlotHookStatus,
    pub session_resolved_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateTrackedPlotHookRequest {
    pub hook_text: String,
    pub status: Option<PlotHookStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateTrackedPlotHookRequest {
    pub status: Option<PlotHookStatus>,
    pub session_resolved_id: Option<Uuid>,
}
