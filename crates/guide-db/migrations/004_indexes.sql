-- Migration 004: Add missing indexes and unique constraint on session numbers
CREATE INDEX IF NOT EXISTS idx_session_events_campaign ON session_events(campaign_id);
CREATE INDEX IF NOT EXISTS idx_encounters_campaign ON encounters(campaign_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_campaign_number ON sessions(campaign_id, session_number);
