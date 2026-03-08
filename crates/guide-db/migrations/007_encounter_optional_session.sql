-- Make encounters.session_id optional (nullable).
-- SQLite does not support DROP NOT NULL via ALTER TABLE, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE encounters_new (
    id                  TEXT PRIMARY KEY NOT NULL,
    session_id          TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    campaign_id         TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    name                TEXT,
    description         TEXT,
    status              TEXT NOT NULL DEFAULT 'pending',
    round               INTEGER NOT NULL DEFAULT 0,
    current_turn_index  INTEGER NOT NULL DEFAULT 0,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO encounters_new
    SELECT id, session_id, campaign_id, name, description, status,
           round, current_turn_index, created_at, updated_at
    FROM encounters;

DROP TABLE encounters;
ALTER TABLE encounters_new RENAME TO encounters;

CREATE INDEX IF NOT EXISTS idx_encounters_session ON encounters(session_id);

PRAGMA foreign_keys = ON;
