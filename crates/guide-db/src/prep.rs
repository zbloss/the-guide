use chrono::Utc;
use std::str::FromStr;
use uuid::Uuid;

use guide_core::{
    models::{DmPrepResult, PrepType},
    GuideError, Result,
};

use crate::{query_all, query_optional, with_db, DuckDbPool};

pub struct DmPrepRepository {
    pool: DuckDbPool,
}

impl DmPrepRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn upsert(
        &self,
        campaign_id: Uuid,
        prep_type: PrepType,
        content: String,
        character_id: Option<Uuid>,
    ) -> Result<DmPrepResult> {
        let campaign_id_str = campaign_id.to_string();
        let prep_type_str = prep_type.as_str().to_string();
        let character_id_str = character_id.map(|id| id.to_string());

        // Delete existing
        {
            let campaign_id_str2 = campaign_id_str.clone();
            let prep_type_str2 = prep_type_str.clone();
            let char_id2 = character_id_str.clone();
            with_db(&self.pool, move |conn| {
                if let Some(char_id) = &char_id2 {
                    conn.execute(
                        "DELETE FROM dm_prep_results \
                         WHERE campaign_id = ? AND prep_type = ? AND character_id = ?",
                        duckdb::params![campaign_id_str2, prep_type_str2, char_id],
                    )
                } else {
                    conn.execute(
                        "DELETE FROM dm_prep_results \
                         WHERE campaign_id = ? AND prep_type = ? AND character_id IS NULL",
                        duckdb::params![campaign_id_str2, prep_type_str2],
                    )
                }
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let campaign_id_str2 = campaign_id_str.clone();
        let prep_type_str2 = prep_type_str.clone();
        let content2 = content.clone();
        let char_id2 = character_id_str.clone();
        let now2 = now.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO dm_prep_results \
                 (id, campaign_id, prep_type, content, character_id, generated_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                duckdb::params![id_str, campaign_id_str2, prep_type_str2, content2, char_id2, now2],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
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

    pub async fn get(
        &self,
        campaign_id: Uuid,
        prep_type: PrepType,
        character_id: Option<Uuid>,
    ) -> Result<Option<DmPrepResult>> {
        let campaign_id_str = campaign_id.to_string();
        let prep_type_str = prep_type.as_str().to_string();
        let character_id_str = character_id.map(|id| id.to_string());

        with_db(&self.pool, move |conn| {
            if let Some(char_id) = &character_id_str {
                query_optional(
                    conn,
                    "SELECT id, campaign_id, prep_type, content, character_id, generated_at \
                     FROM dm_prep_results \
                     WHERE campaign_id = ? AND prep_type = ? AND character_id = ?",
                    duckdb::params![campaign_id_str, prep_type_str, char_id],
                    row_to_prep_result,
                )
            } else {
                query_optional(
                    conn,
                    "SELECT id, campaign_id, prep_type, content, character_id, generated_at \
                     FROM dm_prep_results \
                     WHERE campaign_id = ? AND prep_type = ? AND character_id IS NULL",
                    duckdb::params![campaign_id_str, prep_type_str],
                    row_to_prep_result,
                )
            }
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<DmPrepResult>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, prep_type, content, character_id, generated_at \
                 FROM dm_prep_results WHERE campaign_id = ? ORDER BY generated_at DESC",
                [&id_str],
                row_to_prep_result,
            )
        })
        .await
    }
}

fn row_to_prep_result(row: &duckdb::Row) -> duckdb::Result<DmPrepResult> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let prep_type_str: String = row.get("prep_type")?;
    let character_id_str: Option<String> = row.get("character_id")?;
    let generated_at_str: String = row.get("generated_at")?;

    let character_id = character_id_str
        .as_deref()
        .map(|s| {
            Uuid::parse_str(s).map_err(|e| {
                duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e))
            })
        })
        .transpose()?;

    let prep_type = PrepType::from_str(&prep_type_str).map_err(|e| {
        duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
    })?;

    Ok(DmPrepResult {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        prep_type,
        content: row.get("content")?,
        character_id,
        generated_at: generated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
