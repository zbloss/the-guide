DROP INDEX IF EXISTS idx_campaigns_share_token;
ALTER TABLE campaigns DROP COLUMN share_token;
