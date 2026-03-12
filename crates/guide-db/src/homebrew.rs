use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CreateHomebrewRuleRequest, HomebrewRule},
    GuideError, Result,
};

pub struct HomebrewRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> HomebrewRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        campaign_id: Uuid,
        req: CreateHomebrewRuleRequest,
    ) -> Result<HomebrewRule> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let category = req.category.unwrap_or_else(|| "general".to_string());

        sqlx::query(
            "INSERT INTO homebrew_rules \
             (id, campaign_id, title, description, category, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(&req.title)
        .bind(&req.description)
        .bind(&category)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<HomebrewRule> {
        let row = sqlx::query(
            "SELECT id, campaign_id, title, description, category, created_at \
             FROM homebrew_rules WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("HomebrewRule {id}")))?;

        row_to_rule(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<HomebrewRule>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, title, description, category, created_at \
             FROM homebrew_rules WHERE campaign_id = ? ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_rule).collect()
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM homebrew_rules WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("HomebrewRule {id}")));
        }
        Ok(())
    }
}

fn row_to_rule(row: SqliteRow) -> Result<HomebrewRule> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let created_at_str: String = row.try_get("created_at")?;

    Ok(HomebrewRule {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        category: row.try_get("category")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
