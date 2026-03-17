ALTER TABLE global_documents ADD COLUMN document_kind TEXT NOT NULL DEFAULT 'dm_guide';
ALTER TABLE campaign_documents ADD COLUMN description TEXT;
ALTER TABLE campaign_documents ADD COLUMN story_extraction_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE campaign_documents ADD COLUMN story_extraction_error TEXT;
