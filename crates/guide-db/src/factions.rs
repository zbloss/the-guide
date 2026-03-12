use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CreateFactionRequest, Faction, UpdateFactionReputationRequest},
    GuideError, Result,
};

pub struct FactionRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> FactionRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: CreateFactionRequest,
    ) -> Result<Faction> {
        let faction_id = Uuid::new_v4();
        let rep_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO factions (id, campaign_id, name, description, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(faction_id.to_string())
        .bind(campaign_id.to_string())
        .bind(&req.name)
        .bind(&req.description)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO faction_reputation (id, faction_id, campaign_id, standing, notes, updated_at) \
             VALUES (?, ?, ?, 'Neutral', NULL, ?)",
        )
        .bind(rep_id.to_string())
        .bind(faction_id.to_string())
        .bind(campaign_id.to_string())
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(faction_id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Faction> {
        let row = sqlx::query(
            "SELECT f.id, f.campaign_id, f.name, f.description, f.created_at, \
                    r.standing, r.notes \
             FROM factions f \
             LEFT JOIN faction_reputation r ON r.faction_id = f.id \
             WHERE f.id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("Faction {id}")))?;

        row_to_faction(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<Faction>> {
        let rows = sqlx::query(
            "SELECT f.id, f.campaign_id, f.name, f.description, f.created_at, \
                    r.standing, r.notes \
             FROM factions f \
             LEFT JOIN faction_reputation r ON r.faction_id = f.id \
             WHERE f.campaign_id = ? ORDER BY f.created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_faction).collect()
    }

    pub async fn update_reputation(
        &self,
        faction_id: Uuid,
        req: UpdateFactionReputationRequest,
    ) -> Result<Faction> {
        let now = Utc::now();

        let affected = sqlx::query(
            "UPDATE faction_reputation SET standing = ?, notes = ?, updated_at = ? \
             WHERE faction_id = ?",
        )
        .bind(&req.standing)
        .bind(&req.notes)
        .bind(now.to_rfc3339())
        .bind(faction_id.to_string())
        .execute(self.pool)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Faction {faction_id}")));
        }

        self.get_by_id(faction_id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM factions WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Faction {id}")));
        }
        Ok(())
    }
}

fn row_to_faction(row: SqliteRow) -> Result<Faction> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let created_at_str: String = row.try_get("created_at")?;

    Ok(Faction {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        standing: row.try_get::<Option<String>, _>("standing")?.unwrap_or_else(|| "Neutral".to_string()),
        notes: row.try_get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
