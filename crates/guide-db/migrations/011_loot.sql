-- FEAT-013: Loot & Treasure Log
CREATE TABLE IF NOT EXISTS loot_items (
    id                  TEXT PRIMARY KEY NOT NULL,
    session_id          TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    campaign_id         TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    item_type           TEXT NOT NULL DEFAULT 'misc',   -- weapon, armor, magic, currency, misc
    quantity            INTEGER NOT NULL DEFAULT 1,
    value_gp            REAL NOT NULL DEFAULT 0,
    assigned_to_char_id TEXT REFERENCES characters(id) ON DELETE SET NULL,
    notes               TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_loot_items_session ON loot_items(session_id);
CREATE INDEX IF NOT EXISTS idx_loot_items_campaign ON loot_items(campaign_id);
