use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{Campaign, CreateCampaignRequest, GameSystem, UpdateCampaignRequest},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

/// Query a single column of TEXT ids from `sql` (one `?` param = `id_str`).
fn collect_ids(
    conn: &duckdb::Connection,
    sql: &str,
    id_str: &str,
) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| GuideError::Internal(format!("collect_ids prepare: {e}")))?;
    let ids: std::result::Result<Vec<String>, _> = stmt
        .query_map([id_str], |row| row.get::<_, String>(0))
        .map_err(|e| GuideError::Internal(format!("collect_ids query: {e}")))?
        .collect();
    ids.map_err(|e| GuideError::Internal(format!("collect_ids row: {e}")))
}

pub struct CampaignRepository {
    pool: DuckDbPool,
}

impl CampaignRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(&self, req: CreateCampaignRequest) -> Result<Campaign> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let game_system_str = game_system_to_str(&req.game_system.unwrap_or_default()).to_string();
        let id_str = id.to_string();
        let name = req.name.clone();
        let desc = req.description.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO campaigns (id, name, description, game_system, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                duckdb::params![id_str, name, desc, game_system_str, now, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Campaign> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, name, description, game_system, world_state, \
                 current_chapter, created_at, updated_at \
                 FROM campaigns WHERE id = ?",
                [&id_str],
                row_to_campaign,
                format!("Campaign {id}"),
            )
        })
        .await
    }

    pub async fn list(&self) -> Result<Vec<Campaign>> {
        with_db(&self.pool, |conn| {
            query_all(
                conn,
                "SELECT id, name, description, game_system, world_state, \
                 current_chapter, created_at, updated_at \
                 FROM campaigns ORDER BY created_at DESC",
                [],
                row_to_campaign,
            )
        })
        .await
    }

    pub async fn update(&self, id: Uuid, req: UpdateCampaignRequest) -> Result<Campaign> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();

        if let Some(name) = &req.name {
            let name = name.clone();
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE campaigns SET name = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![name, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        if let Some(desc) = &req.description {
            let desc = desc.clone();
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE campaigns SET description = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![desc, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        if let Some(ws) = &req.world_state {
            let ws_json = serde_json::to_string(ws)?;
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE campaigns SET world_state = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![ws_json, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        if let Some(chapter) = &req.current_chapter {
            let chapter = chapter.clone();
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE campaigns SET current_chapter = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![chapter, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM campaigns WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Campaign {id}")));
            }
            Ok(())
        })
        .await
    }

    pub async fn cascade_delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            // Pre-collect child IDs to avoid DuckDB MVCC read-write conflicts
            // that occur when DELETE subqueries read from tables we later write to.
            let doc_ids = collect_ids(conn, "SELECT id FROM campaign_documents WHERE campaign_id = ?", &id_str)?;
            let enc_ids = collect_ids(conn, "SELECT id FROM encounters WHERE campaign_id = ?", &id_str)?;
            let char_ids = collect_ids(conn, "SELECT id FROM characters WHERE campaign_id = ?", &id_str)?;
            let faction_ids = collect_ids(conn, "SELECT id FROM factions WHERE campaign_id = ?", &id_str)?;

            // Leaf tables that reference campaign_documents
            for doc_id in &doc_ids {
                conn.execute("DELETE FROM document_page_ocr WHERE document_id = ?", [doc_id])
                    .map_err(|e| GuideError::Internal(format!("cascade delete document_page_ocr: {e}")))?;
            }

            // Leaf tables that reference encounters
            for enc_id in &enc_ids {
                conn.execute("DELETE FROM encounter_turns WHERE encounter_id = ?", [enc_id])
                    .map_err(|e| GuideError::Internal(format!("cascade delete encounter_turns: {e}")))?;
                conn.execute("DELETE FROM combat_participants WHERE encounter_id = ?", [enc_id])
                    .map_err(|e| GuideError::Internal(format!("cascade delete combat_participants: {e}")))?;
            }

            // Leaf tables that reference characters
            for char_id in &char_ids {
                conn.execute("DELETE FROM plot_hooks WHERE character_id = ?", [char_id])
                    .map_err(|e| GuideError::Internal(format!("cascade delete plot_hooks: {e}")))?;
            }

            // Leaf tables that reference factions
            for faction_id in &faction_ids {
                conn.execute("DELETE FROM faction_reputation WHERE faction_id = ?", [faction_id])
                    .map_err(|e| GuideError::Internal(format!("cascade delete faction_reputation: {e}")))?;
            }

            // Direct campaign_id tables, in FK-safe dependency order:
            // Must delete loot_items before sessions (loot_items.session_id FK)
            //             and before characters (loot_items.assigned_to_char_id FK)
            // Must delete dm_prep_results before characters (dm_prep_results.character_id FK)
            // Must delete character_relationships before characters
            // Must delete campaign_calendar before sessions
            // Must delete session_events before sessions
            // Must delete encounters before sessions
            // Must delete prepopulated_encounters before story_events
            // Must delete story_subplots, story_events before story_arcs
            let ordered: &[&str] = &[
                "DELETE FROM lore_chunks WHERE campaign_id = ?",
                "DELETE FROM campaign_global_docs WHERE campaign_id = ?",
                // Anything referencing sessions or characters must come first
                "DELETE FROM loot_items WHERE campaign_id = ?",
                "DELETE FROM dm_prep_results WHERE campaign_id = ?",
                "DELETE FROM character_relationships WHERE campaign_id = ?",
                "DELETE FROM campaign_calendar WHERE campaign_id = ?",
                "DELETE FROM session_events WHERE campaign_id = ?",
                "DELETE FROM encounters WHERE campaign_id = ?",
                "DELETE FROM sessions WHERE campaign_id = ?",
                "DELETE FROM characters WHERE campaign_id = ?",
                "DELETE FROM campaign_documents WHERE campaign_id = ?",
                // Story tables: children before parents
                "DELETE FROM prepopulated_encounters WHERE campaign_id = ?",
                "DELETE FROM story_subplots WHERE campaign_id = ?",
                "DELETE FROM story_events WHERE campaign_id = ?",
                "DELETE FROM story_arcs WHERE campaign_id = ?",
                "DELETE FROM character_arcs WHERE campaign_id = ?",
                "DELETE FROM story_npcs WHERE campaign_id = ?",
                "DELETE FROM story_locations WHERE campaign_id = ?",
                "DELETE FROM story_factions WHERE campaign_id = ?",
                // Remaining direct campaign tables
                "DELETE FROM playstyle_profiles WHERE campaign_id = ?",
                "DELETE FROM campaign_chat WHERE campaign_id = ?",
                "DELETE FROM homebrew_rules WHERE campaign_id = ?",
                "DELETE FROM campaign_webhooks WHERE campaign_id = ?",
                "DELETE FROM factions WHERE campaign_id = ?",
                // Campaign itself last
                "DELETE FROM campaigns WHERE id = ?",
            ];

            for sql in ordered {
                conn.execute(sql, [&id_str])
                    .map_err(|e| GuideError::Internal(format!("cascade delete ({sql}): {e}")))?;
            }

            Ok(())
        })
        .await
    }

    pub async fn analytics(&self, campaign_id: Uuid) -> Result<serde_json::Value> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            let sessions_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sessions WHERE campaign_id = ?",
                    [&id_str],
                    |r| r.get(0),
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            let encounters_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM encounters WHERE campaign_id = ?",
                    [&id_str],
                    |r| r.get(0),
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            let characters_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM characters WHERE campaign_id = ?",
                    [&id_str],
                    |r| r.get(0),
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            let sessions_by_month = query_all(
                conn,
                "SELECT strftime('%Y-%m', created_at::TIMESTAMP) AS month, COUNT(*) AS cnt \
                 FROM sessions WHERE campaign_id = ? GROUP BY month ORDER BY month",
                [&id_str],
                |row| {
                    let month: String = row.get("month")?;
                    let count: i64 = row.get("cnt")?;
                    Ok(serde_json::json!({ "month": month, "count": count }))
                },
            )?;

            let encounter_difficulty = query_all(
                conn,
                "SELECT CASE WHEN round <= 1 THEN 'easy' WHEN round <= 3 THEN 'medium' \
                 WHEN round <= 6 THEN 'hard' ELSE 'deadly' END AS difficulty, COUNT(*) AS cnt \
                 FROM encounters WHERE campaign_id = ? AND status = 'completed' GROUP BY difficulty",
                [&id_str],
                |row| {
                    let diff: String = row.get("difficulty")?;
                    let count: i64 = row.get("cnt")?;
                    Ok(serde_json::json!({ "difficulty": diff, "count": count }))
                },
            )?;

            Ok(serde_json::json!({
                "sessions_count": sessions_count,
                "encounters_count": encounters_count,
                "characters_count": characters_count,
                "sessions_by_month": sessions_by_month,
                "encounter_difficulty": encounter_difficulty,
            }))
        })
        .await
    }
}

fn row_to_campaign(row: &duckdb::Row) -> duckdb::Result<Campaign> {
    let id_str: String = row.get("id")?;
    let game_system_str: String = row.get("game_system")?;
    let world_state_str: Option<String> = row.get("world_state")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    Ok(Campaign {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        name: row.get("name")?,
        description: row.get("description")?,
        game_system: parse_game_system(&game_system_str),
        world_state: world_state_str
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok()),
        current_chapter: row.get("current_chapter")?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn game_system_to_str(s: &GameSystem) -> &'static str {
    match s {
        GameSystem::Dnd5e => "dnd5e",
        GameSystem::Pathfinder2e => "pathfinder2e",
    }
}

fn parse_game_system(s: &str) -> GameSystem {
    match s {
        "pathfinder2e" => GameSystem::Pathfinder2e,
        _ => GameSystem::Dnd5e,
    }
}
