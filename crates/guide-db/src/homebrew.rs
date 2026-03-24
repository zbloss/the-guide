use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{CreateHomebrewRuleRequest, HomebrewRule},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct HomebrewRepository {
    pool: DuckDbPool,
}

impl HomebrewRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: CreateHomebrewRuleRequest,
    ) -> Result<HomebrewRule> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let category = req.category.unwrap_or_else(|| "general".to_string());
        let title = req.title.clone();
        let description = req.description.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO homebrew_rules \
                 (id, campaign_id, title, description, category, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                duckdb::params![id_str, campaign_id_str, title, description, category, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<HomebrewRule> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, title, description, category, created_at \
                 FROM homebrew_rules WHERE id = ?",
                [&id_str],
                row_to_rule,
                format!("HomebrewRule {id}"),
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<HomebrewRule>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, title, description, category, created_at \
                 FROM homebrew_rules WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_rule,
            )
        })
        .await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM homebrew_rules WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("HomebrewRule {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_rule(row: &duckdb::Row) -> duckdb::Result<HomebrewRule> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let created_at_str: String = row.get("created_at")?;

    Ok(HomebrewRule {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        title: row.get("title")?,
        description: row.get("description")?,
        category: row.get("category")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
