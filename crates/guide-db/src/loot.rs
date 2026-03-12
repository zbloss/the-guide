use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{CreateLootItemRequest, LootItem},
    GuideError, Result,
};

pub struct LootRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> LootRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        session_id: Uuid,
        campaign_id: Uuid,
        req: CreateLootItemRequest,
    ) -> Result<LootItem> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO loot_items \
             (id, session_id, campaign_id, name, item_type, quantity, value_gp, \
              assigned_to_char_id, notes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(session_id.to_string())
        .bind(campaign_id.to_string())
        .bind(&req.name)
        .bind(req.item_type.as_deref().unwrap_or("misc"))
        .bind(req.quantity.unwrap_or(1))
        .bind(req.value_gp.unwrap_or(0.0))
        .bind(req.assigned_to_char_id.map(|u| u.to_string()))
        .bind(&req.notes)
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<LootItem> {
        let row = sqlx::query(
            "SELECT id, session_id, campaign_id, name, item_type, quantity, value_gp, \
             assigned_to_char_id, notes, created_at \
             FROM loot_items WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("LootItem {id}")))?;

        row_to_item(row)
    }

    pub async fn list_by_session(&self, session_id: Uuid) -> Result<Vec<LootItem>> {
        let rows = sqlx::query(
            "SELECT id, session_id, campaign_id, name, item_type, quantity, value_gp, \
             assigned_to_char_id, notes, created_at \
             FROM loot_items WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_item).collect()
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<LootItem>> {
        let rows = sqlx::query(
            "SELECT id, session_id, campaign_id, name, item_type, quantity, value_gp, \
             assigned_to_char_id, notes, created_at \
             FROM loot_items WHERE campaign_id = ? ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_item).collect()
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM loot_items WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("LootItem {id}")));
        }
        Ok(())
    }
}

fn row_to_item(row: SqliteRow) -> Result<LootItem> {
    let id_str: String = row.try_get("id")?;
    let session_id_str: String = row.try_get("session_id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let assigned_str: Option<String> = row.try_get("assigned_to_char_id")?;
    let created_at_str: String = row.try_get("created_at")?;

    Ok(LootItem {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        session_id: Uuid::parse_str(&session_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        item_type: row.try_get("item_type")?,
        quantity: row.try_get("quantity")?,
        value_gp: row.try_get("value_gp")?,
        assigned_to_char_id: assigned_str.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        notes: row.try_get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
