-- 006_constraints.sql: Add index on plot_hooks.campaign_id
-- (CHECK constraints on characters hp would require full table recreation;
--  skipped here since existing data integrity is enforced at the application layer.)

CREATE INDEX IF NOT EXISTS idx_plot_hooks_campaign ON plot_hooks(campaign_id);
