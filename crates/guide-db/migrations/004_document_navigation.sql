-- Table of Contents entries extracted from ingested documents.
CREATE TABLE IF NOT EXISTS document_toc_entries (
    id          TEXT    NOT NULL PRIMARY KEY,
    document_id TEXT    NOT NULL,
    title       TEXT    NOT NULL,
    page_num    INTEGER NOT NULL,
    depth       INTEGER NOT NULL DEFAULT 0
);

-- Back-of-book Index entries extracted from ingested documents.
-- `pages` is a JSON array of u32 page numbers.
CREATE TABLE IF NOT EXISTS document_index_entries (
    id          TEXT NOT NULL PRIMARY KEY,
    document_id TEXT NOT NULL,
    term        TEXT NOT NULL,
    pages       TEXT NOT NULL
);
