-- FEAT-012: In-World Calendar Tracker
CREATE TABLE IF NOT EXISTS campaign_calendar (
    id              TEXT PRIMARY KEY NOT NULL,
    campaign_id     TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    session_id      TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    in_game_date    TEXT NOT NULL,           -- free-form in-world date string
    real_date       TEXT NOT NULL,           -- ISO8601 real-world date
    notes           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_campaign_calendar_campaign ON campaign_calendar(campaign_id);
