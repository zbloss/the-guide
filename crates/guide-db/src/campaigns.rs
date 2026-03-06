use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{Campaign, CreateCampaignRequest, GameSystem, UpdateCampaignRequest},
    GuideError, Result,
};

pub struct CampaignRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CampaignRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, req: CreateCampaignRequest) -> Result<Campaign> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let game_system_str = match req.game_system.unwrap_or_default() {
            GameSystem::Dnd5e => "dnd5e".to_string(),
            GameSystem::Pathfinder2e => "pathfinder2e".to_string(),
            GameSystem::Custom(s) => s,
        };

        sqlx::query(
            "INSERT INTO campaigns (id, name, description, game_system, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&req.name)
        .bind(&req.description)
        .bind(&game_system_str)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Campaign> {
        let row = sqlx::query(
            "SELECT id, name, description, game_system, world_state, created_at, updated_at \
             FROM campaigns WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?
        .ok_or_else(|| GuideError::NotFound(format!("Campaign {id}")))?;

        row_to_campaign(row)
    }

    pub async fn list(&self) -> Result<Vec<Campaign>> {
        let rows = sqlx::query(
            "SELECT id, name, description, game_system, world_state, created_at, updated_at \
             FROM campaigns ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;

        rows.into_iter().map(row_to_campaign).collect()
    }

    pub async fn update(&self, id: Uuid, req: UpdateCampaignRequest) -> Result<Campaign> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();

        if let Some(name) = &req.name {
            sqlx::query("UPDATE campaigns SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await
                .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;
        }

        if let Some(desc) = &req.description {
            sqlx::query("UPDATE campaigns SET description = ?, updated_at = ? WHERE id = ?")
                .bind(desc)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await
                .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;
        }

        if let Some(ws) = &req.world_state {
            let ws_json = serde_json::to_string(ws)
                .map_err(|e| GuideError::Serialization(e.to_string()))?;
            sqlx::query("UPDATE campaigns SET world_state = ?, updated_at = ? WHERE id = ?")
                .bind(&ws_json)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await
                .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?;
        }

        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM campaigns WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await
            .map_err(|e: sqlx::Error| GuideError::Database(e.to_string()))?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Campaign {id}")));
        }
        Ok(())
    }
}

fn row_to_campaign(row: SqliteRow) -> Result<Campaign> {
    let id_str: String = row.try_get("id").map_err(|e| GuideError::Database(e.to_string()))?;
    let game_system_str: String =
        row.try_get("game_system").map_err(|e| GuideError::Database(e.to_string()))?;
    let world_state_str: Option<String> =
        row.try_get("world_state").map_err(|e| GuideError::Database(e.to_string()))?;
    let created_at_str: String =
        row.try_get("created_at").map_err(|e| GuideError::Database(e.to_string()))?;
    let updated_at_str: String =
        row.try_get("updated_at").map_err(|e| GuideError::Database(e.to_string()))?;

    Ok(Campaign {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name").map_err(|e| GuideError::Database(e.to_string()))?,
        description: row.try_get("description").map_err(|e| GuideError::Database(e.to_string()))?,
        game_system: parse_game_system(&game_system_str),
        world_state: world_state_str
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok()),
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn parse_game_system(s: &str) -> GameSystem {
    match s {
        "dnd5e" => GameSystem::Dnd5e,
        "pathfinder2e" => GameSystem::Pathfinder2e,
        other => GameSystem::Custom(other.to_string()),
    }
}
