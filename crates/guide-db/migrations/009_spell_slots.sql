-- Add spell slot tracking to characters
-- Stored as JSON array: [{level, total, remaining}, ...]
ALTER TABLE characters ADD COLUMN spell_slots TEXT NOT NULL DEFAULT '[]';
