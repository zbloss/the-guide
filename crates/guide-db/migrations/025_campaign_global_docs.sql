CREATE TABLE IF NOT EXISTS campaign_global_docs (
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    global_doc_id TEXT NOT NULL REFERENCES global_documents(id) ON DELETE CASCADE,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (campaign_id, global_doc_id)
);
