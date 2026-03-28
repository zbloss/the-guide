use guide_core::{
    models::{IndexEntry, TocEntry},
    GuideError, Result,
};
use uuid::Uuid;

use crate::{query_all, with_db, DuckDbPool};

pub struct NavigationRepository<'a> {
    pool: &'a DuckDbPool,
}

impl<'a> NavigationRepository<'a> {
    pub fn new(pool: &'a DuckDbPool) -> Self {
        Self { pool }
    }

    /// Persist ToC entries for a document, replacing any previously stored entries.
    pub async fn insert_toc_entries(
        &self,
        document_id: Uuid,
        entries: Vec<TocEntry>,
    ) -> Result<()> {
        let doc_id = document_id.to_string();
        with_db(self.pool, move |conn| {
            conn.execute(
                "DELETE FROM document_toc_entries WHERE document_id = ?",
                duckdb::params![doc_id],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;

            for entry in &entries {
                let id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO document_toc_entries (id, document_id, title, page_num, depth)
                     VALUES (?, ?, ?, ?, ?)",
                    duckdb::params![id, doc_id, entry.title, entry.page, entry.depth],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            }
            Ok(())
        })
        .await
    }

    /// Persist Index entries for a document, replacing any previously stored entries.
    pub async fn insert_index_entries(
        &self,
        document_id: Uuid,
        entries: Vec<IndexEntry>,
    ) -> Result<()> {
        let doc_id = document_id.to_string();
        with_db(self.pool, move |conn| {
            conn.execute(
                "DELETE FROM document_index_entries WHERE document_id = ?",
                duckdb::params![doc_id],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;

            for entry in &entries {
                let id = Uuid::new_v4().to_string();
                let pages_json = serde_json::to_string(&entry.pages)
                    .map_err(|e| GuideError::Internal(e.to_string()))?;
                conn.execute(
                    "INSERT INTO document_index_entries (id, document_id, term, pages)
                     VALUES (?, ?, ?, ?)",
                    duckdb::params![id, doc_id, entry.term, pages_json],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            }
            Ok(())
        })
        .await
    }

    /// Retrieve all ToC entries for a document ordered by their natural row order (insertion order).
    pub async fn get_toc(&self, document_id: Uuid) -> Result<Vec<TocEntry>> {
        let doc_id = document_id.to_string();
        with_db(self.pool, move |conn| {
            query_all(
                conn,
                "SELECT title, page_num, depth FROM document_toc_entries
                 WHERE document_id = ?
                 ORDER BY rowid",
                duckdb::params![doc_id],
                |row| {
                    Ok(TocEntry {
                        title: row.get(0)?,
                        page: row.get::<_, u32>(1)?,
                        depth: row.get::<_, u8>(2)?,
                    })
                },
            )
        })
        .await
    }

    /// Load the entire Index for a document as a lowercase-term → page-numbers map.
    pub async fn get_all_index_entries(
        &self,
        document_id: Uuid,
    ) -> Result<std::collections::HashMap<String, Vec<u32>>> {
        let doc_id = document_id.to_string();
        with_db(self.pool, move |conn| {
            let rows: Vec<(String, String)> = query_all(
                conn,
                "SELECT lower(term), pages FROM document_index_entries WHERE document_id = ?",
                duckdb::params![doc_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            let mut map = std::collections::HashMap::with_capacity(rows.len());
            for (term, pages_json) in rows {
                let pages: Vec<u32> = serde_json::from_str(&pages_json)
                    .map_err(|e| GuideError::Internal(e.to_string()))?;
                map.insert(term, pages);
            }
            Ok(map)
        })
        .await
    }

    /// Look up page numbers for a term from the document Index (case-insensitive).
    /// Returns an empty Vec if the term is not found.
    pub async fn lookup_index_term(
        &self,
        document_id: Uuid,
        term: &str,
    ) -> Result<Vec<u32>> {
        let doc_id = document_id.to_string();
        let term = term.to_lowercase();
        with_db(self.pool, move |conn| {
            let rows: Vec<String> = query_all(
                conn,
                "SELECT pages FROM document_index_entries
                 WHERE document_id = ? AND lower(term) = ?",
                duckdb::params![doc_id, term],
                |row| row.get(0),
            )?;

            let mut all_pages: Vec<u32> = Vec::new();
            for json in rows {
                let pages: Vec<u32> = serde_json::from_str(&json)
                    .map_err(|e| GuideError::Internal(e.to_string()))?;
                all_pages.extend(pages);
            }
            all_pages.sort_unstable();
            all_pages.dedup();
            Ok(all_pages)
        })
        .await
    }
}
