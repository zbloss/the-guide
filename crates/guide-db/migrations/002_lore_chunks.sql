-- 002_lore_chunks.sql
-- Stores embedded text chunks for RAG search, replacing Qdrant.
-- Embeddings are stored as BLOBs: 768 little-endian IEEE 754 f32 values = 3072 bytes.

CREATE TABLE IF NOT EXISTS lore_chunks (
    id                  TEXT PRIMARY KEY NOT NULL,
    campaign_id         TEXT,                        -- NULL = global rulebook chunk
    source_document_id  TEXT NOT NULL,
    document_kind       TEXT NOT NULL,
    content             TEXT NOT NULL,
    lore_type           TEXT NOT NULL DEFAULT 'plot',
    significance        TEXT NOT NULL DEFAULT 'minor',
    entities            TEXT NOT NULL DEFAULT '[]',
    is_player_visible   INTEGER NOT NULL DEFAULT 1,
    page_start          INTEGER NOT NULL DEFAULT 0,
    page_end            INTEGER NOT NULL DEFAULT 0,
    section_path        TEXT NOT NULL DEFAULT '',
    doc_title           TEXT NOT NULL DEFAULT '',
    embedding           BLOB NOT NULL,
    created_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_lore_chunks_document ON lore_chunks(source_document_id);
CREATE INDEX IF NOT EXISTS idx_lore_chunks_campaign ON lore_chunks(campaign_id);
