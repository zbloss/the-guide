ALTER TABLE character_relationships ADD COLUMN updated_at TEXT;
UPDATE character_relationships SET updated_at = created_at WHERE updated_at IS NULL;
