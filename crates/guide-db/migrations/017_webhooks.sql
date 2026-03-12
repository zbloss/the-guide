CREATE TABLE IF NOT EXISTS campaign_webhooks (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    events TEXT NOT NULL DEFAULT '["session_start","session_end","encounter_end"]',
    created_at TEXT NOT NULL
);
