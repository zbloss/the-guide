-- New tables for NPCs, locations, and factions extracted from campaign documents

CREATE TABLE IF NOT EXISTS story_npcs (
    id            TEXT NOT NULL PRIMARY KEY,
    campaign_id   TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    source_doc_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'neutral',
    description   TEXT NOT NULL DEFAULT '',
    location      TEXT,
    is_dm_only    INTEGER NOT NULL DEFAULT 0,
    dm_notes      TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS story_locations (
    id            TEXT NOT NULL PRIMARY KEY,
    campaign_id   TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    source_doc_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    location_type TEXT NOT NULL DEFAULT 'dungeon',
    dm_notes      TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS story_factions (
    id            TEXT NOT NULL PRIMARY KEY,
    campaign_id   TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    source_doc_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    alignment_hint TEXT,
    dm_notes      TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
