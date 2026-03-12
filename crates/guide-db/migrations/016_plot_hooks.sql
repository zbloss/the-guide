CREATE TABLE IF NOT EXISTS plot_hooks (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    hook_text TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    session_resolved_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
