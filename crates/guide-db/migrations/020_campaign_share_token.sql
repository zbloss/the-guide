-- Migration 020: Add public share token to campaigns
ALTER TABLE campaigns ADD COLUMN share_token TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_campaigns_share_token ON campaigns(share_token) WHERE share_token IS NOT NULL;
