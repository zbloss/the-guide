use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{CreateLootItemRequest, LootItem},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct LootRepository {
    pool: DuckDbPool,
}

impl LootRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(
        &self,
        session_id: Uuid,
        campaign_id: Uuid,
        req: CreateLootItemRequest,
    ) -> Result<LootItem> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let session_id_str = session_id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let name = req.name.clone();
        let item_type = req.item_type.clone().unwrap_or_else(|| "misc".to_string());
        let quantity = req.quantity.unwrap_or(1);
        let value_gp = req.value_gp.unwrap_or(0.0);
        let assigned = req.assigned_to_char_id.map(|u| u.to_string());
        let notes = req.notes.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO loot_items \
                 (id, session_id, campaign_id, name, item_type, quantity, value_gp, \
                  assigned_to_char_id, notes, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    id_str, session_id_str, campaign_id_str, name, item_type,
                    quantity, value_gp, assigned, notes, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<LootItem> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, session_id, campaign_id, name, item_type, quantity, value_gp, \
                 assigned_to_char_id, notes, created_at \
                 FROM loot_items WHERE id = ?",
                [&id_str],
                row_to_item,
                format!("LootItem {id}"),
            )
        })
        .await
    }

    pub async fn list_by_session(&self, session_id: Uuid) -> Result<Vec<LootItem>> {
        let id_str = session_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, session_id, campaign_id, name, item_type, quantity, value_gp, \
                 assigned_to_char_id, notes, created_at \
                 FROM loot_items WHERE session_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_item,
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<LootItem>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, session_id, campaign_id, name, item_type, quantity, value_gp, \
                 assigned_to_char_id, notes, created_at \
                 FROM loot_items WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_item,
            )
        })
        .await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM loot_items WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("LootItem {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_item(row: &duckdb::Row) -> duckdb::Result<LootItem> {
    let id_str: String = row.get("id")?;
    let session_id_str: String = row.get("session_id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let assigned_str: Option<String> = row.get("assigned_to_char_id")?;
    let created_at_str: String = row.get("created_at")?;

    Ok(LootItem {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        session_id: Uuid::parse_str(&session_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(2, duckdb::types::Type::Text, Box::new(e)))?,
        name: row.get("name")?,
        item_type: row.get("item_type")?,
        quantity: row.get("quantity")?,
        value_gp: row.get("value_gp")?,
        assigned_to_char_id: assigned_str.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        notes: row.get("notes")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
