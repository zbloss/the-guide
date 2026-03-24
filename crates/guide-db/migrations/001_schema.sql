-- 001_schema.sql: Full DuckDB schema for The Guide
-- Single consolidated schema replacing all SQLite incremental migrations.

CREATE TABLE IF NOT EXISTS campaigns (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    game_system     TEXT NOT NULL DEFAULT 'dnd5e',
    world_state     TEXT,
    current_chapter TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS characters (
    id              TEXT PRIMARY KEY NOT NULL,
    campaign_id     TEXT NOT NULL REFERENCES campaigns(id),
    name            TEXT NOT NULL,
    character_type  TEXT NOT NULL DEFAULT 'pc',
    class           TEXT,
    race            TEXT,
    level           INTEGER NOT NULL DEFAULT 1,
    max_hp          INTEGER NOT NULL DEFAULT 10,
    current_hp      INTEGER NOT NULL DEFAULT 10,
    armor_class     INTEGER NOT NULL DEFAULT 10,
    speed           INTEGER NOT NULL DEFAULT 30,
    ability_scores  TEXT NOT NULL DEFAULT '{}',
    conditions      TEXT NOT NULL DEFAULT '[]',
    spell_slots     TEXT NOT NULL DEFAULT '[]',
    backstory       TEXT,
    is_alive        INTEGER NOT NULL DEFAULT 1,
    portrait_url    TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_characters_campaign ON characters(campaign_id);

CREATE TABLE IF NOT EXISTS sessions (
    id              TEXT PRIMARY KEY NOT NULL,
    campaign_id     TEXT NOT NULL REFERENCES campaigns(id),
    session_number  INTEGER NOT NULL DEFAULT 1,
    title           TEXT,
    notes           TEXT,
    started_at      TEXT,
    ended_at        TEXT,
    map_url         TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_campaign ON sessions(campaign_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_campaign_number ON sessions(campaign_id, session_number);

CREATE TABLE IF NOT EXISTS session_events (
    id                      TEXT PRIMARY KEY NOT NULL,
    session_id              TEXT NOT NULL REFERENCES sessions(id),
    campaign_id             TEXT NOT NULL REFERENCES campaigns(id),
    event_type              TEXT NOT NULL,
    description             TEXT NOT NULL,
    significance            TEXT NOT NULL DEFAULT 'minor',
    is_player_visible       INTEGER NOT NULL DEFAULT 1,
    involved_character_ids  TEXT NOT NULL DEFAULT '[]',
    occurred_at             TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_session_events_session ON session_events(session_id);
CREATE INDEX IF NOT EXISTS idx_session_events_campaign ON session_events(campaign_id);

CREATE TABLE IF NOT EXISTS encounters (
    id                  TEXT PRIMARY KEY NOT NULL,
    session_id          TEXT REFERENCES sessions(id),
    campaign_id         TEXT NOT NULL REFERENCES campaigns(id),
    name                TEXT,
    description         TEXT,
    status              TEXT NOT NULL DEFAULT 'pending',
    round               INTEGER NOT NULL DEFAULT 0,
    current_turn_index  INTEGER NOT NULL DEFAULT 0,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_encounters_session ON encounters(session_id);
CREATE INDEX IF NOT EXISTS idx_encounters_campaign ON encounters(campaign_id);

CREATE TABLE IF NOT EXISTS combat_participants (
    id                    TEXT PRIMARY KEY NOT NULL,
    encounter_id          TEXT NOT NULL REFERENCES encounters(id),
    character_id          TEXT NOT NULL REFERENCES characters(id),
    name                  TEXT NOT NULL,
    initiative_roll       INTEGER NOT NULL DEFAULT 0,
    initiative_modifier   INTEGER NOT NULL DEFAULT 0,
    initiative_total      INTEGER NOT NULL DEFAULT 0,
    current_hp            INTEGER NOT NULL DEFAULT 10,
    max_hp                INTEGER NOT NULL DEFAULT 10,
    armor_class           INTEGER NOT NULL DEFAULT 10,
    conditions            TEXT NOT NULL DEFAULT '[]',
    action_budget         TEXT NOT NULL DEFAULT '{}',
    has_taken_turn        INTEGER NOT NULL DEFAULT 0,
    is_defeated           INTEGER NOT NULL DEFAULT 0,
    death_saves_success   INTEGER NOT NULL DEFAULT 0,
    death_saves_failure   INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_combat_participants_encounter ON combat_participants(encounter_id);

CREATE TABLE IF NOT EXISTS campaign_documents (
    id                        TEXT PRIMARY KEY NOT NULL,
    campaign_id               TEXT NOT NULL REFERENCES campaigns(id),
    filename                  TEXT NOT NULL,
    file_size_bytes           INTEGER NOT NULL DEFAULT 0,
    stored_path               TEXT NOT NULL,
    page_count                INTEGER,
    document_kind             TEXT NOT NULL DEFAULT 'campaign',
    description               TEXT,
    ingestion_status          TEXT NOT NULL DEFAULT 'pending',
    ingestion_error           TEXT,
    story_extraction_status   TEXT NOT NULL DEFAULT 'pending',
    story_extraction_error    TEXT,
    uploaded_at               TEXT NOT NULL,
    ingested_at               TEXT
);

CREATE INDEX IF NOT EXISTS idx_documents_campaign ON campaign_documents(campaign_id);

CREATE TABLE IF NOT EXISTS global_documents (
    id               TEXT PRIMARY KEY NOT NULL,
    title            TEXT NOT NULL,
    filename         TEXT NOT NULL,
    file_size_bytes  INTEGER NOT NULL DEFAULT 0,
    stored_path      TEXT NOT NULL,
    page_count       INTEGER,
    document_kind    TEXT NOT NULL DEFAULT 'dm_guide',
    ingestion_status TEXT NOT NULL DEFAULT 'pending',
    ingestion_error  TEXT,
    uploaded_at      TEXT NOT NULL,
    ingested_at      TEXT
);

CREATE INDEX IF NOT EXISTS idx_global_docs_status ON global_documents(ingestion_status);

CREATE TABLE IF NOT EXISTS document_page_ocr (
    id           TEXT PRIMARY KEY NOT NULL,
    document_id  TEXT NOT NULL REFERENCES campaign_documents(id),
    page_num     INTEGER NOT NULL,
    raw_text     TEXT NOT NULL DEFAULT '',
    is_dm_only   INTEGER NOT NULL DEFAULT 0,
    UNIQUE (document_id, page_num)
);

CREATE INDEX IF NOT EXISTS idx_doc_page_ocr_doc ON document_page_ocr(document_id);

CREATE TABLE IF NOT EXISTS plot_hooks (
    id                   TEXT PRIMARY KEY NOT NULL,
    character_id         TEXT NOT NULL REFERENCES characters(id),
    hook_text            TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'open',
    session_resolved_id  TEXT REFERENCES sessions(id),
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_plot_hooks_character ON plot_hooks(character_id);

CREATE TABLE IF NOT EXISTS playstyle_profiles (
    campaign_id           TEXT NOT NULL PRIMARY KEY REFERENCES campaigns(id),
    combat_affinity       REAL NOT NULL DEFAULT 0.33,
    social_affinity       REAL NOT NULL DEFAULT 0.33,
    exploration_affinity  REAL NOT NULL DEFAULT 0.34,
    preferred_difficulty  REAL NOT NULL DEFAULT 0.5,
    sessions_sampled      INTEGER NOT NULL DEFAULT 0,
    updated_at            TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS campaign_calendar (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL REFERENCES campaigns(id),
    session_id   TEXT REFERENCES sessions(id),
    in_game_date TEXT NOT NULL,
    real_date    TEXT NOT NULL,
    notes        TEXT,
    created_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_campaign_calendar_campaign ON campaign_calendar(campaign_id);

CREATE TABLE IF NOT EXISTS loot_items (
    id                   TEXT PRIMARY KEY NOT NULL,
    session_id           TEXT NOT NULL REFERENCES sessions(id),
    campaign_id          TEXT NOT NULL REFERENCES campaigns(id),
    name                 TEXT NOT NULL,
    item_type            TEXT NOT NULL DEFAULT 'misc',
    quantity             INTEGER NOT NULL DEFAULT 1,
    value_gp             REAL NOT NULL DEFAULT 0,
    assigned_to_char_id  TEXT REFERENCES characters(id),
    notes                TEXT,
    created_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_loot_items_session ON loot_items(session_id);
CREATE INDEX IF NOT EXISTS idx_loot_items_campaign ON loot_items(campaign_id);

CREATE TABLE IF NOT EXISTS campaign_chat (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL REFERENCES campaigns(id),
    role         TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content      TEXT NOT NULL,
    perspective  TEXT NOT NULL DEFAULT 'dm',
    created_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_campaign ON campaign_chat(campaign_id, created_at);

CREATE TABLE IF NOT EXISTS homebrew_rules (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL REFERENCES campaigns(id),
    title        TEXT NOT NULL,
    description  TEXT NOT NULL,
    category     TEXT NOT NULL DEFAULT 'general',
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS factions (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL REFERENCES campaigns(id),
    name         TEXT NOT NULL,
    description  TEXT,
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS faction_reputation (
    id           TEXT PRIMARY KEY NOT NULL,
    faction_id   TEXT NOT NULL REFERENCES factions(id),
    campaign_id  TEXT NOT NULL,
    standing     TEXT NOT NULL DEFAULT 'Neutral',
    notes        TEXT,
    updated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS campaign_webhooks (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL REFERENCES campaigns(id),
    url          TEXT NOT NULL,
    events       TEXT NOT NULL DEFAULT '["session_start","session_end","encounter_end"]',
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS encounter_templates (
    id               TEXT PRIMARY KEY NOT NULL,
    name             TEXT NOT NULL,
    description      TEXT,
    participant_data TEXT NOT NULL,
    created_at       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS dm_prep_results (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL REFERENCES campaigns(id),
    prep_type    TEXT NOT NULL,
    content      TEXT NOT NULL,
    character_id TEXT REFERENCES characters(id),
    generated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_dm_prep_campaign ON dm_prep_results(campaign_id);
CREATE INDEX IF NOT EXISTS idx_dm_prep_type ON dm_prep_results(campaign_id, prep_type);

CREATE TABLE IF NOT EXISTS character_relationships (
    id                  TEXT PRIMARY KEY NOT NULL,
    campaign_id         TEXT NOT NULL REFERENCES campaigns(id),
    from_character_id   TEXT NOT NULL REFERENCES characters(id),
    to_character_id     TEXT NOT NULL REFERENCES characters(id),
    relationship_type   TEXT NOT NULL DEFAULT 'ally',
    notes               TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT,
    UNIQUE(campaign_id, from_character_id, to_character_id)
);

CREATE INDEX IF NOT EXISTS idx_relationships_campaign ON character_relationships(campaign_id);
CREATE INDEX IF NOT EXISTS idx_relationships_from ON character_relationships(from_character_id);
CREATE INDEX IF NOT EXISTS idx_relationships_to ON character_relationships(to_character_id);

CREATE TABLE IF NOT EXISTS encounter_turns (
    id             TEXT PRIMARY KEY NOT NULL,
    encounter_id   TEXT NOT NULL REFERENCES encounters(id),
    turn_number    INTEGER NOT NULL,
    round_number   INTEGER NOT NULL,
    snapshot_json  TEXT NOT NULL,
    recorded_at    TEXT NOT NULL,
    UNIQUE(encounter_id, turn_number)
);

CREATE INDEX IF NOT EXISTS idx_encounter_turns_enc ON encounter_turns(encounter_id);

CREATE TABLE IF NOT EXISTS campaign_global_docs (
    campaign_id    TEXT NOT NULL REFERENCES campaigns(id),
    global_doc_id  TEXT NOT NULL REFERENCES global_documents(id),
    added_at       TEXT NOT NULL,
    PRIMARY KEY (campaign_id, global_doc_id)
);

CREATE TABLE IF NOT EXISTS story_arcs (
    id            TEXT PRIMARY KEY NOT NULL,
    campaign_id   TEXT NOT NULL REFERENCES campaigns(id),
    source_doc_id TEXT NOT NULL,
    title         TEXT NOT NULL,
    description   TEXT NOT NULL,
    arc_order     INTEGER NOT NULL DEFAULT 0,
    status        TEXT NOT NULL DEFAULT 'open',
    dm_notes      TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_events (
    id                   TEXT PRIMARY KEY NOT NULL,
    campaign_id          TEXT NOT NULL REFERENCES campaigns(id),
    arc_id               TEXT REFERENCES story_arcs(id),
    source_doc_id        TEXT NOT NULL,
    title                TEXT NOT NULL,
    description          TEXT NOT NULL,
    event_type           TEXT NOT NULL DEFAULT 'combat',
    significance         TEXT NOT NULL DEFAULT 'minor',
    location             TEXT,
    involved_characters  TEXT NOT NULL DEFAULT '[]',
    event_order          INTEGER NOT NULL DEFAULT 0,
    is_dm_only           INTEGER NOT NULL DEFAULT 0,
    dm_notes             TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_subplots (
    id            TEXT PRIMARY KEY NOT NULL,
    campaign_id   TEXT NOT NULL REFERENCES campaigns(id),
    arc_id        TEXT REFERENCES story_arcs(id),
    source_doc_id TEXT NOT NULL,
    title         TEXT NOT NULL,
    description   TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'open',
    dm_notes      TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS character_arcs (
    id             TEXT PRIMARY KEY NOT NULL,
    campaign_id    TEXT NOT NULL REFERENCES campaigns(id),
    character_name TEXT NOT NULL,
    character_id   TEXT,
    source_doc_id  TEXT NOT NULL,
    description    TEXT NOT NULL,
    arc_points     TEXT NOT NULL DEFAULT '[]',
    dm_notes       TEXT,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS prepopulated_encounters (
    id              TEXT PRIMARY KEY NOT NULL,
    campaign_id     TEXT NOT NULL REFERENCES campaigns(id),
    story_event_id  TEXT REFERENCES story_events(id),
    source_doc_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL,
    location        TEXT,
    difficulty_hint TEXT,
    monsters        TEXT NOT NULL DEFAULT '[]',
    dm_notes        TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_npcs (
    id            TEXT PRIMARY KEY NOT NULL,
    campaign_id   TEXT NOT NULL REFERENCES campaigns(id),
    source_doc_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'neutral',
    description   TEXT NOT NULL DEFAULT '',
    location      TEXT,
    is_dm_only    INTEGER NOT NULL DEFAULT 0,
    dm_notes      TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_locations (
    id             TEXT PRIMARY KEY NOT NULL,
    campaign_id    TEXT NOT NULL REFERENCES campaigns(id),
    source_doc_id  TEXT NOT NULL,
    name           TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    location_type  TEXT NOT NULL DEFAULT 'dungeon',
    dm_notes       TEXT,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_factions (
    id              TEXT PRIMARY KEY NOT NULL,
    campaign_id     TEXT NOT NULL REFERENCES campaigns(id),
    source_doc_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    alignment_hint  TEXT,
    dm_notes        TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
