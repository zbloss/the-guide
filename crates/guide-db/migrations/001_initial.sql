-- 001_initial.sql: Core schema for The Guide
-- Note: WAL mode and FK pragmas are set programmatically by init_sqlite().

-- ── Campaigns ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS campaigns (
    id            TEXT PRIMARY KEY NOT NULL,  -- UUID v4
    name          TEXT NOT NULL,
    description   TEXT,
    game_system   TEXT NOT NULL DEFAULT 'dnd5e',
    world_state   TEXT,                       -- JSON blob
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ── Characters ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS characters (
    id              TEXT PRIMARY KEY NOT NULL,
    campaign_id     TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    character_type  TEXT NOT NULL DEFAULT 'pc',  -- pc | npc | monster
    class           TEXT,
    race            TEXT,
    level           INTEGER NOT NULL DEFAULT 1,
    max_hp          INTEGER NOT NULL DEFAULT 10,
    current_hp      INTEGER NOT NULL DEFAULT 10,
    armor_class     INTEGER NOT NULL DEFAULT 10,
    speed           INTEGER NOT NULL DEFAULT 30,
    ability_scores  TEXT NOT NULL DEFAULT '{}', -- JSON
    conditions      TEXT NOT NULL DEFAULT '[]', -- JSON array
    backstory       TEXT,                        -- JSON
    is_alive        INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_characters_campaign ON characters(campaign_id);

-- ── Sessions ─────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS sessions (
    id              TEXT PRIMARY KEY NOT NULL,
    campaign_id     TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    session_number  INTEGER NOT NULL DEFAULT 1,
    title           TEXT,
    notes           TEXT,
    started_at      TEXT,
    ended_at        TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_campaign ON sessions(campaign_id);

-- ── Session Events ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS session_events (
    id                       TEXT PRIMARY KEY NOT NULL,
    session_id               TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    campaign_id              TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    event_type               TEXT NOT NULL,
    description              TEXT NOT NULL,
    significance             TEXT NOT NULL DEFAULT 'minor',
    is_player_visible        INTEGER NOT NULL DEFAULT 1,
    involved_character_ids   TEXT NOT NULL DEFAULT '[]', -- JSON array of UUIDs
    occurred_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_session_events_session ON session_events(session_id);

-- ── Encounters ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS encounters (
    id                  TEXT PRIMARY KEY NOT NULL,
    session_id          TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    campaign_id         TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    name                TEXT,
    description         TEXT,
    status              TEXT NOT NULL DEFAULT 'pending',
    round               INTEGER NOT NULL DEFAULT 0,
    current_turn_index  INTEGER NOT NULL DEFAULT 0,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_encounters_session ON encounters(session_id);

-- ── Combat Participants ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS combat_participants (
    id                  TEXT PRIMARY KEY NOT NULL,
    encounter_id        TEXT NOT NULL REFERENCES encounters(id) ON DELETE CASCADE,
    character_id        TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    initiative_roll     INTEGER NOT NULL DEFAULT 0,
    initiative_modifier INTEGER NOT NULL DEFAULT 0,
    initiative_total    INTEGER NOT NULL DEFAULT 0,
    current_hp          INTEGER NOT NULL DEFAULT 10,
    max_hp              INTEGER NOT NULL DEFAULT 10,
    armor_class         INTEGER NOT NULL DEFAULT 10,
    conditions          TEXT NOT NULL DEFAULT '[]',  -- JSON
    action_budget       TEXT NOT NULL DEFAULT '{}',  -- JSON
    has_taken_turn      INTEGER NOT NULL DEFAULT 0,
    is_defeated         INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_combat_participants_encounter ON combat_participants(encounter_id);

-- ── Campaign Documents ────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS campaign_documents (
    id                TEXT PRIMARY KEY NOT NULL,
    campaign_id       TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    filename          TEXT NOT NULL,
    file_size_bytes   INTEGER NOT NULL DEFAULT 0,
    stored_path       TEXT NOT NULL,
    page_count        INTEGER,
    ingestion_status  TEXT NOT NULL DEFAULT 'pending',
    ingestion_error   TEXT,
    uploaded_at       TEXT NOT NULL DEFAULT (datetime('now')),
    ingested_at       TEXT
);

CREATE INDEX IF NOT EXISTS idx_documents_campaign ON campaign_documents(campaign_id);

-- ── Plot Hooks ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS plot_hooks (
    id             TEXT PRIMARY KEY NOT NULL,
    character_id   TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    campaign_id    TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    description    TEXT NOT NULL,
    priority       TEXT NOT NULL DEFAULT 'medium',
    is_active      INTEGER NOT NULL DEFAULT 1,
    llm_extracted  INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_plot_hooks_character ON plot_hooks(character_id);
