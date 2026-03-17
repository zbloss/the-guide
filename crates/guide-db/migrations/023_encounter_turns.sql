CREATE TABLE IF NOT EXISTS encounter_turns (
    id TEXT PRIMARY KEY NOT NULL,
    encounter_id TEXT NOT NULL REFERENCES encounters(id) ON DELETE CASCADE,
    turn_number INTEGER NOT NULL,
    round_number INTEGER NOT NULL,
    snapshot_json TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    UNIQUE(encounter_id, turn_number)
);

CREATE INDEX IF NOT EXISTS idx_encounter_turns_enc ON encounter_turns(encounter_id);
