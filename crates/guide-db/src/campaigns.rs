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
        let game_system_str = game_system_to_str(&req.game_system.unwrap_or_default());

        sqlx::query(
            "INSERT INTO campaigns (id, name, description, game_system, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&req.name)
        .bind(&req.description)
        .bind(game_system_str)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Campaign> {
        let row = sqlx::query(
            "SELECT id, name, description, game_system, world_state, share_token, created_at, updated_at \
             FROM campaigns WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("Campaign {id}")))?;

        row_to_campaign(row)
    }

    pub async fn list(&self) -> Result<Vec<Campaign>> {
        let rows = sqlx::query(
            "SELECT id, name, description, game_system, world_state, share_token, created_at, updated_at \
             FROM campaigns ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await?;

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
                .await?;
        }

        if let Some(desc) = &req.description {
            sqlx::query("UPDATE campaigns SET description = ?, updated_at = ? WHERE id = ?")
                .bind(desc)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await?;
        }

        if let Some(ws) = &req.world_state {
            let ws_json = serde_json::to_string(ws)?;
            sqlx::query("UPDATE campaigns SET world_state = ?, updated_at = ? WHERE id = ?")
                .bind(&ws_json)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await?;
        }

        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM campaigns WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Campaign {id}")));
        }
        Ok(())
    }

    pub async fn generate_share_token(&self, id: Uuid) -> Result<Campaign> {
        let token = Uuid::new_v4().to_string().replace('-', "");
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE campaigns SET share_token = ?, updated_at = ? WHERE id = ?")
            .bind(&token)
            .bind(&now)
            .bind(id.to_string())
            .execute(self.pool)
            .await?;
        self.get_by_id(id).await
    }

    pub async fn get_by_share_token(&self, token: &str) -> Result<Campaign> {
        let row = sqlx::query(
            "SELECT id, name, description, game_system, world_state, share_token, created_at, updated_at \
             FROM campaigns WHERE share_token = ?",
        )
        .bind(token)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound("Campaign not found".into()))?;
        row_to_campaign(row)
    }

    pub async fn analytics(&self, campaign_id: Uuid) -> Result<serde_json::Value> {
        let id_str = campaign_id.to_string();

        let sessions_count: i64 =
            sqlx::query("SELECT COUNT(*) FROM sessions WHERE campaign_id = ?")
                .bind(&id_str)
                .fetch_one(self.pool)
                .await?
                .get(0);

        let encounters_count: i64 =
            sqlx::query("SELECT COUNT(*) FROM encounters WHERE campaign_id = ?")
                .bind(&id_str)
                .fetch_one(self.pool)
                .await?
                .get(0);

        let characters_count: i64 =
            sqlx::query("SELECT COUNT(*) FROM characters WHERE campaign_id = ?")
                .bind(&id_str)
                .fetch_one(self.pool)
                .await?
                .get(0);

        let session_rows = sqlx::query(
            "SELECT strftime('%Y-%m', created_at) AS month, COUNT(*) AS cnt \
             FROM sessions WHERE campaign_id = ? GROUP BY month ORDER BY month",
        )
        .bind(&id_str)
        .fetch_all(self.pool)
        .await?;
        let sessions_by_month: Vec<serde_json::Value> = session_rows
            .iter()
            .map(|r| {
                let month: String = r.get("month");
                let count: i64 = r.get("cnt");
                serde_json::json!({ "month": month, "count": count })
            })
            .collect();

        let difficulty_rows = sqlx::query(
            "SELECT CASE WHEN round <= 1 THEN 'easy' WHEN round <= 3 THEN 'medium' \
             WHEN round <= 6 THEN 'hard' ELSE 'deadly' END AS difficulty, COUNT(*) AS cnt \
             FROM encounters WHERE campaign_id = ? AND status = 'completed' GROUP BY difficulty",
        )
        .bind(&id_str)
        .fetch_all(self.pool)
        .await?;
        let encounter_difficulty: Vec<serde_json::Value> = difficulty_rows
            .iter()
            .map(|r| {
                let diff: String = r.get("difficulty");
                let count: i64 = r.get("cnt");
                serde_json::json!({ "difficulty": diff, "count": count })
            })
            .collect();

        Ok(serde_json::json!({
            "sessions_count": sessions_count,
            "encounters_count": encounters_count,
            "characters_count": characters_count,
            "sessions_by_month": sessions_by_month,
            "encounter_difficulty": encounter_difficulty,
        }))
    }
}

fn row_to_campaign(row: SqliteRow) -> Result<Campaign> {
    let id_str: String = row.try_get("id")?;
    let game_system_str: String = row.try_get("game_system")?;
    let world_state_str: Option<String> = row.try_get("world_state")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;
    let share_token: Option<String> = row.try_get("share_token").unwrap_or(None);

    Ok(Campaign {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        game_system: parse_game_system(&game_system_str),
        world_state: world_state_str
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok()),
        share_token,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn game_system_to_str(s: &GameSystem) -> &'static str {
    match s {
        GameSystem::Dnd5e => "dnd5e",
        GameSystem::Pathfinder2e => "pathfinder2e",
    }
}

fn parse_game_system(s: &str) -> GameSystem {
    match s {
        "pathfinder2e" => GameSystem::Pathfinder2e,
        _ => GameSystem::Dnd5e,
    }
}
