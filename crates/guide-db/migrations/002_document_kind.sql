-- 002_document_kind.sql: Global rulebook support

-- Add document_kind column to existing campaign documents
ALTER TABLE campaign_documents
    ADD COLUMN document_kind TEXT NOT NULL DEFAULT 'campaign';

-- New table for globally-scoped rulebooks (PHB, DMG, SRD, etc.)
CREATE TABLE IF NOT EXISTS global_documents (
    id               TEXT PRIMARY KEY NOT NULL,
    title            TEXT NOT NULL,
    filename         TEXT NOT NULL,
    file_size_bytes  INTEGER NOT NULL DEFAULT 0,
    stored_path      TEXT NOT NULL,
    page_count       INTEGER,
    ingestion_status TEXT NOT NULL DEFAULT 'pending',
    ingestion_error  TEXT,
    uploaded_at      TEXT NOT NULL DEFAULT (datetime('now')),
    ingested_at      TEXT
);

CREATE INDEX IF NOT EXISTS idx_global_docs_status ON global_documents(ingestion_status);
