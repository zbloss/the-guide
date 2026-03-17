use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use guide_core::{
    models::{DmPrepResult, PrepType},
    GuideError, Result,
};

pub struct DmPrepRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> DmPrepRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Upsert a prep result, replacing any existing entry with the same
    /// (campaign_id, prep_type, character_id) key.
    pub async fn upsert(
        &self,
        campaign_id: Uuid,
        prep_type: PrepType,
        content: String,
        character_id: Option<Uuid>,
    ) -> Result<DmPrepResult> {
        let campaign_id_str = campaign_id.to_string();
        let prep_type_str = prep_type.as_str();
        let character_id_str = character_id.map(|id| id.to_string());

        // Delete existing row with same (campaign_id, prep_type, character_id)
        if let Some(char_id) = &character_id_str {
            sqlx::query(
                "DELETE FROM dm_prep_results \
                 WHERE campaign_id = ? AND prep_type = ? AND character_id = ?",
            )
            .bind(&campaign_id_str)
            .bind(prep_type_str)
            .bind(char_id)
            .execute(self.pool)
            .await?;
        } else {
            sqlx::query(
                "DELETE FROM dm_prep_results \
                 WHERE campaign_id = ? AND prep_type = ? AND character_id IS NULL",
            )
            .bind(&campaign_id_str)
            .bind(prep_type_str)
            .execute(self.pool)
            .await?;
        }

        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO dm_prep_results \
             (id, campaign_id, prep_type, content, character_id, generated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&campaign_id_str)
        .bind(prep_type_str)
        .bind(&content)
        .bind(&character_id_str)
        .bind(&now)
        .execute(self.pool)
        .await?;

        Ok(DmPrepResult {
            id,
            campaign_id,
            prep_type,
            content,
            character_id,
            generated_at: now.parse().unwrap_or_else(|_| Utc::now()),
        })
    }

    /// Retrieve a cached prep result, or `None` if not yet generated.
    pub async fn get(
        &self,
        campaign_id: Uuid,
        prep_type: PrepType,
        character_id: Option<Uuid>,
    ) -> Result<Option<DmPrepResult>> {
        let campaign_id_str = campaign_id.to_string();
        let prep_type_str = prep_type.as_str();
        let character_id_str = character_id.map(|id| id.to_string());

        let row = if let Some(char_id) = &character_id_str {
            sqlx::query(
                "SELECT id, campaign_id, prep_type, content, character_id, generated_at \
                 FROM dm_prep_results \
                 WHERE campaign_id = ? AND prep_type = ? AND character_id = ?",
            )
            .bind(&campaign_id_str)
            .bind(prep_type_str)
            .bind(char_id)
            .fetch_optional(self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, campaign_id, prep_type, content, character_id, generated_at \
                 FROM dm_prep_results \
                 WHERE campaign_id = ? AND prep_type = ? AND character_id IS NULL",
            )
            .bind(&campaign_id_str)
            .bind(prep_type_str)
            .fetch_optional(self.pool)
            .await?
        };

        row.map(row_to_prep_result).transpose()
    }

    /// List all prep results for a campaign, newest first.
    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<DmPrepResult>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, prep_type, content, character_id, generated_at \
             FROM dm_prep_results \
             WHERE campaign_id = ? \
             ORDER BY generated_at DESC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_prep_result).collect()
    }
}

fn row_to_prep_result(row: SqliteRow) -> Result<DmPrepResult> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let prep_type_str: String = row.try_get("prep_type")?;
    let character_id_str: Option<String> = row.try_get("character_id")?;
    let generated_at_str: String = row.try_get("generated_at")?;

    let character_id = character_id_str
        .as_deref()
        .map(|s| Uuid::parse_str(s).map_err(|e| GuideError::Internal(e.to_string())))
        .transpose()?;

    Ok(DmPrepResult {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        prep_type: PrepType::from_str(&prep_type_str).map_err(GuideError::Internal)?,
        content: row.try_get("content")?,
        character_id,
        generated_at: generated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
