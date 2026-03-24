use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{CharacterRelationship, CreateRelationshipRequest, UpdateRelationshipRequest},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct RelationshipRepository {
    pool: DuckDbPool,
}

impl RelationshipRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: CreateRelationshipRequest,
    ) -> Result<CharacterRelationship> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let from_str = req.from_character_id.to_string();
        let to_str = req.to_character_id.to_string();
        let rel_type = req.relationship_type.clone();
        let notes = req.notes.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO character_relationships \
                 (id, campaign_id, from_character_id, to_character_id, relationship_type, notes, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![id_str, campaign_id_str, from_str, to_str, rel_type, notes, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CharacterRelationship> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, from_character_id, to_character_id, \
                 relationship_type, notes, created_at, updated_at \
                 FROM character_relationships WHERE id = ?",
                [&id_str],
                row_to_relationship,
                format!("Relationship {id}"),
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CharacterRelationship>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, from_character_id, to_character_id, \
                 relationship_type, notes, created_at, updated_at \
                 FROM character_relationships WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_relationship,
            )
        })
        .await
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateRelationshipRequest,
    ) -> Result<CharacterRelationship> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let rel_type = req.relationship_type.clone();
        let notes = req.notes.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE character_relationships \
                 SET relationship_type = COALESCE(?, relationship_type), \
                     notes = COALESCE(?, notes), \
                     updated_at = ? \
                 WHERE id = ?",
                duckdb::params![rel_type, notes, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM character_relationships WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Relationship {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_relationship(row: &duckdb::Row) -> duckdb::Result<CharacterRelationship> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let from_str: String = row.get("from_character_id")?;
    let to_str: String = row.get("to_character_id")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: Option<String> = row.get("updated_at").unwrap_or(None);

    Ok(CharacterRelationship {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        from_character_id: Uuid::parse_str(&from_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(2, duckdb::types::Type::Text, Box::new(e)))?,
        to_character_id: Uuid::parse_str(&to_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(3, duckdb::types::Type::Text, Box::new(e)))?,
        relationship_type: row.get("relationship_type")?,
        notes: row.get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.and_then(|s| s.parse().ok()),
    })
}
