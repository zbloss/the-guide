CREATE TABLE IF NOT EXISTS character_relationships (
    id TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    from_character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    to_character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    relationship_type TEXT NOT NULL DEFAULT 'ally',
    notes TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(campaign_id, from_character_id, to_character_id)
);

CREATE INDEX IF NOT EXISTS idx_relationships_campaign ON character_relationships(campaign_id);
CREATE INDEX IF NOT EXISTS idx_relationships_from ON character_relationships(from_character_id);
CREATE INDEX IF NOT EXISTS idx_relationships_to ON character_relationships(to_character_id);
