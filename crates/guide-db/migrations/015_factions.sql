CREATE TABLE IF NOT EXISTS factions (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS faction_reputation (
    id TEXT PRIMARY KEY,
    faction_id TEXT NOT NULL,
    campaign_id TEXT NOT NULL,
    standing TEXT NOT NULL DEFAULT 'Neutral',
    notes TEXT,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (faction_id) REFERENCES factions(id) ON DELETE CASCADE
);
