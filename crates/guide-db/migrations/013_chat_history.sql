CREATE TABLE IF NOT EXISTS campaign_chat (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    perspective TEXT NOT NULL DEFAULT 'dm',
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_chat_campaign ON campaign_chat(campaign_id, created_at);
