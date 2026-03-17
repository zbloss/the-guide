use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CharacterRelationship, CreateRelationshipRequest},
    GuideError, Result,
};

pub struct RelationshipRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> RelationshipRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: CreateRelationshipRequest,
    ) -> Result<CharacterRelationship> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_relationships \
             (id, campaign_id, from_character_id, to_character_id, relationship_type, notes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(req.from_character_id.to_string())
        .bind(req.to_character_id.to_string())
        .bind(&req.relationship_type)
        .bind(&req.notes)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<CharacterRelationship> {
        let row = sqlx::query(
            "SELECT id, campaign_id, from_character_id, to_character_id, \
             relationship_type, notes, created_at \
             FROM character_relationships WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("Relationship {id}")))?;

        row_to_relationship(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<CharacterRelationship>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, from_character_id, to_character_id, \
             relationship_type, notes, created_at \
             FROM character_relationships WHERE campaign_id = ? ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_relationship).collect()
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM character_relationships WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Relationship {id}")));
        }
        Ok(())
    }
}

fn row_to_relationship(row: SqliteRow) -> Result<CharacterRelationship> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let from_str: String = row.try_get("from_character_id")?;
    let to_str: String = row.try_get("to_character_id")?;
    let created_at_str: String = row.try_get("created_at")?;

    Ok(CharacterRelationship {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        from_character_id: Uuid::parse_str(&from_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        to_character_id: Uuid::parse_str(&to_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        relationship_type: row.try_get("relationship_type")?,
        notes: row.try_get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
