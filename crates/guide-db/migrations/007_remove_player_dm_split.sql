-- Remove player/DM split columns (project is DM-only)
ALTER TABLE session_events DROP COLUMN IF EXISTS is_player_visible;
ALTER TABLE document_page_ocr DROP COLUMN IF EXISTS is_dm_only;
ALTER TABLE campaign_chat DROP COLUMN IF EXISTS perspective;
ALTER TABLE story_events DROP COLUMN IF EXISTS is_dm_only;
ALTER TABLE story_npcs DROP COLUMN IF EXISTS is_dm_only;
ALTER TABLE lore_chunks DROP COLUMN IF EXISTS is_player_visible;
