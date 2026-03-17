-- Add current story position to campaigns
ALTER TABLE campaigns ADD COLUMN current_chapter TEXT;

-- Persisted DM prep results (one per campaign+type or campaign+type+character)
CREATE TABLE IF NOT EXISTS dm_prep_results (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    prep_type    TEXT NOT NULL,
    content      TEXT NOT NULL,
    character_id TEXT REFERENCES characters(id) ON DELETE CASCADE,
    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_dm_prep_campaign ON dm_prep_results(campaign_id);
CREATE INDEX IF NOT EXISTS idx_dm_prep_type ON dm_prep_results(campaign_id, prep_type);
