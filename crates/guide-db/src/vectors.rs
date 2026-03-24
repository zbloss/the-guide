use chrono::Utc;
use guide_core::{
    models::{DocumentKind, RankedChunk},
    GuideError, Result,
};
use uuid::Uuid;

use crate::{with_db, DuckDbPool};

pub struct LoreChunkInsert {
    pub id: Uuid,
    pub campaign_id: Option<Uuid>,
    pub source_document_id: Uuid,
    pub document_kind: DocumentKind,
    pub content: String,
    pub lore_type: String,
    pub significance: String,
    pub entities: Vec<String>,
    pub is_player_visible: bool,
    pub page_range: (u32, u32),
    pub section_path: String,
    pub doc_title: String,
    pub vector: Vec<f32>,
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn blob_to_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

fn doc_kind_to_str(kind: &DocumentKind) -> &'static str {
    match kind {
        DocumentKind::DmGuide => "dm_guide",
        DocumentKind::MonsterManual => "monster_manual",
        DocumentKind::Srd => "srd",
        DocumentKind::Campaign => "campaign",
        DocumentKind::Supplemental => "supplemental",
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Insert (or replace) a batch of lore chunks into DuckDB.
///
/// All existing chunks for the same `source_document_id` values are deleted
/// first, making this operation idempotent for re-ingestion.
pub async fn upsert_chunks(pool: &DuckDbPool, chunks: Vec<LoreChunkInsert>) -> Result<()> {
    if chunks.is_empty() {
        return Ok(());
    }
    with_db(pool, move |conn| {
        // Collect unique doc IDs to delete before re-inserting.
        let mut seen = std::collections::HashSet::new();
        for chunk in &chunks {
            if seen.insert(chunk.source_document_id) {
                conn.execute(
                    "DELETE FROM lore_chunks WHERE source_document_id = ?",
                    duckdb::params![chunk.source_document_id.to_string()],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            }
        }

        let mut stmt = conn
            .prepare(
                "INSERT INTO lore_chunks \
                 (id, campaign_id, source_document_id, document_kind, content, \
                  lore_type, significance, entities, is_player_visible, \
                  page_start, page_end, section_path, doc_title, embedding, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;

        let now = Utc::now().to_rfc3339();

        for chunk in &chunks {
            let doc_kind = doc_kind_to_str(&chunk.document_kind);
            let entities_json =
                serde_json::to_string(&chunk.entities).unwrap_or_else(|_| "[]".into());
            let embedding_blob = vec_to_blob(&chunk.vector);
            let campaign_id_str = chunk.campaign_id.map(|id| id.to_string());

            stmt.execute(duckdb::params![
                chunk.id.to_string(),
                campaign_id_str,
                chunk.source_document_id.to_string(),
                doc_kind,
                chunk.content,
                chunk.lore_type,
                chunk.significance,
                entities_json,
                chunk.is_player_visible as i32,
                chunk.page_range.0 as i64,
                chunk.page_range.1 as i64,
                chunk.section_path,
                chunk.doc_title,
                embedding_blob,
                now,
            ])
            .map_err(|e| GuideError::Internal(e.to_string()))?;
        }

        Ok(())
    })
    .await
}

/// Retrieve the top-k lore chunks most similar to `embedding` by cosine score.
///
/// If `campaign_id` is `Some`, queries only that campaign's chunks.
/// If `campaign_id` is `None`, queries only global (rulebook) chunks.
pub async fn query_chunks(
    pool: &DuckDbPool,
    campaign_id: Option<Uuid>,
    embedding: &[f32],
    limit: usize,
    player_visible_only: bool,
) -> Result<Vec<RankedChunk>> {
    let embedding: Vec<f32> = embedding.to_vec();
    let player_filter = player_visible_only as i32;

    with_db(pool, move |conn| {
        struct Row {
            content: String,
            section_path: String,
            doc_title: String,
            embedding_blob: Vec<u8>,
        }

        let rows: Vec<Row> = if let Some(cid) = campaign_id {
            let mut stmt = conn
                .prepare(
                    "SELECT content, section_path, doc_title, embedding \
                     FROM lore_chunks \
                     WHERE campaign_id = ? \
                       AND (? = 0 OR is_player_visible = 1)",
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            stmt.query_map(duckdb::params![cid.to_string(), player_filter], |row| {
                Ok(Row {
                    content: row.get(0)?,
                    section_path: row.get(1)?,
                    doc_title: row.get(2)?,
                    embedding_blob: row.get(3)?,
                })
            })
            .map_err(|e| GuideError::Internal(e.to_string()))?
            .collect::<duckdb::Result<Vec<Row>>>()
            .map_err(|e| GuideError::Internal(e.to_string()))?
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT content, section_path, doc_title, embedding \
                     FROM lore_chunks \
                     WHERE campaign_id IS NULL \
                       AND (? = 0 OR is_player_visible = 1)",
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            stmt.query_map(duckdb::params![player_filter], |row| {
                Ok(Row {
                    content: row.get(0)?,
                    section_path: row.get(1)?,
                    doc_title: row.get(2)?,
                    embedding_blob: row.get(3)?,
                })
            })
            .map_err(|e| GuideError::Internal(e.to_string()))?
            .collect::<duckdb::Result<Vec<Row>>>()
            .map_err(|e| GuideError::Internal(e.to_string()))?
        };

        let mut scored: Vec<RankedChunk> = rows
            .into_iter()
            .map(|row| {
                let row_vec = blob_to_vec(&row.embedding_blob);
                let score = cosine_similarity(&embedding, &row_vec);
                RankedChunk {
                    content: row.content,
                    section_path: row.section_path,
                    doc_title: row.doc_title,
                    score,
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    })
    .await
}

/// Delete all lore chunks associated with a specific source document.
pub async fn delete_document_vectors(pool: &DuckDbPool, doc_id: Uuid) -> Result<()> {
    with_db(pool, move |conn| {
        conn.execute(
            "DELETE FROM lore_chunks WHERE source_document_id = ?",
            duckdb::params![doc_id.to_string()],
        )
        .map_err(|e| GuideError::Internal(e.to_string()))?;
        Ok(())
    })
    .await
}

/// Delete all lore chunks belonging to a campaign (used when a campaign is deleted).
pub async fn delete_campaign_vectors(pool: &DuckDbPool, campaign_id: Uuid) -> Result<()> {
    with_db(pool, move |conn| {
        conn.execute(
            "DELETE FROM lore_chunks WHERE campaign_id = ?",
            duckdb::params![campaign_id.to_string()],
        )
        .map_err(|e| GuideError::Internal(e.to_string()))?;
        Ok(())
    })
    .await
}
