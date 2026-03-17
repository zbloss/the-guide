CREATE TABLE IF NOT EXISTS story_arcs (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    source_doc_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    arc_order INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'open',
    dm_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS story_events (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    arc_id TEXT REFERENCES story_arcs(id) ON DELETE SET NULL,
    source_doc_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    event_type TEXT NOT NULL DEFAULT 'combat',
    significance TEXT NOT NULL DEFAULT 'minor',
    location TEXT,
    involved_characters TEXT NOT NULL DEFAULT '[]',
    event_order INTEGER NOT NULL DEFAULT 0,
    is_dm_only INTEGER NOT NULL DEFAULT 0,
    dm_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS story_subplots (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    arc_id TEXT REFERENCES story_arcs(id) ON DELETE SET NULL,
    source_doc_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    dm_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS character_arcs (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    character_name TEXT NOT NULL,
    character_id TEXT,
    source_doc_id TEXT NOT NULL,
    description TEXT NOT NULL,
    arc_points TEXT NOT NULL DEFAULT '[]',
    dm_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS prepopulated_encounters (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    story_event_id TEXT REFERENCES story_events(id) ON DELETE SET NULL,
    source_doc_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    location TEXT,
    difficulty_hint TEXT,
    monsters TEXT NOT NULL DEFAULT '[]',
    dm_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
