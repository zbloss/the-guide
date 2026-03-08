CREATE TABLE IF NOT EXISTS playstyle_profiles (
    campaign_id TEXT PRIMARY KEY,
    combat_affinity REAL NOT NULL DEFAULT 0.33,
    social_affinity REAL NOT NULL DEFAULT 0.33,
    exploration_affinity REAL NOT NULL DEFAULT 0.34,
    preferred_difficulty REAL NOT NULL DEFAULT 0.5,
    sessions_sampled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
