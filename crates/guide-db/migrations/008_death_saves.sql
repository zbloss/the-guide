-- Add death save tracking columns to combat_participants
ALTER TABLE combat_participants ADD COLUMN death_saves_success INTEGER NOT NULL DEFAULT 0;
ALTER TABLE combat_participants ADD COLUMN death_saves_failure INTEGER NOT NULL DEFAULT 0;
