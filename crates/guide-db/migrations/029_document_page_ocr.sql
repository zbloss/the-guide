CREATE TABLE IF NOT EXISTS document_page_ocr (
    id           TEXT    PRIMARY KEY NOT NULL,
    document_id  TEXT    NOT NULL REFERENCES campaign_documents(id) ON DELETE CASCADE,
    page_num     INTEGER NOT NULL,
    raw_text     TEXT    NOT NULL DEFAULT '',
    is_dm_only   INTEGER NOT NULL DEFAULT 0,
    UNIQUE (document_id, page_num)
);
CREATE INDEX IF NOT EXISTS idx_doc_page_ocr_doc ON document_page_ocr(document_id);
