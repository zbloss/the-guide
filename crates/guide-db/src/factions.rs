use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{CreateFactionRequest, Faction, UpdateFactionReputationRequest},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct FactionRepository {
    pool: DuckDbPool,
}

impl FactionRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(&self, campaign_id: Uuid, req: CreateFactionRequest) -> Result<Faction> {
        let faction_id = Uuid::new_v4();
        let rep_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let faction_id_str = faction_id.to_string();
        let rep_id_str = rep_id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let name = req.name.clone();
        let description = req.description.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO factions (id, campaign_id, name, description, created_at) \
                 VALUES (?, ?, ?, ?, ?)",
                duckdb::params![faction_id_str, campaign_id_str, name, description, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;

            conn.execute(
                "INSERT INTO faction_reputation \
                 (id, faction_id, campaign_id, standing, notes, updated_at) \
                 VALUES (?, ?, ?, 'Neutral', NULL, ?)",
                duckdb::params![rep_id_str, faction_id_str, campaign_id_str, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(faction_id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Faction> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT f.id, f.campaign_id, f.name, f.description, f.created_at, \
                        r.standing, r.notes \
                 FROM factions f \
                 LEFT JOIN faction_reputation r ON r.faction_id = f.id \
                 WHERE f.id = ?",
                [&id_str],
                row_to_faction,
                format!("Faction {id}"),
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<Faction>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT f.id, f.campaign_id, f.name, f.description, f.created_at, \
                        r.standing, r.notes \
                 FROM factions f \
                 LEFT JOIN faction_reputation r ON r.faction_id = f.id \
                 WHERE f.campaign_id = ? ORDER BY f.created_at ASC",
                [&id_str],
                row_to_faction,
            )
        })
        .await
    }

    pub async fn update_reputation(
        &self,
        faction_id: Uuid,
        req: UpdateFactionReputationRequest,
    ) -> Result<Faction> {
        let now = Utc::now().to_rfc3339();
        let id_str = faction_id.to_string();
        let standing = req.standing.clone();
        let notes = req.notes.clone();

        with_db(&self.pool, move |conn| {
            let n = conn
                .execute(
                    "UPDATE faction_reputation SET standing = ?, notes = ?, updated_at = ? \
                     WHERE faction_id = ?",
                    duckdb::params![standing, notes, now, id_str],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Faction {faction_id}")));
            }
            Ok(())
        })
        .await?;

        self.get_by_id(faction_id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM factions WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Faction {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_faction(row: &duckdb::Row) -> duckdb::Result<Faction> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let created_at_str: String = row.get("created_at")?;

    Ok(Faction {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        name: row.get("name")?,
        description: row.get("description")?,
        standing: row.get::<_, Option<String>>("standing")?.unwrap_or_else(|| "Neutral".to_string()),
        notes: row.get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
