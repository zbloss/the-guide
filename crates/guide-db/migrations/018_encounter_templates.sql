CREATE TABLE IF NOT EXISTS encounter_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    participant_data TEXT NOT NULL,  -- JSON array of {name, max_hp, armor_class, speed}
    created_at TEXT NOT NULL
);
