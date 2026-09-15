-- This file should undo anything in `up.sql`

-- Drop indices and table
DROP INDEX IF EXISTS idx_recon_rule_profile_active_priority;
DROP INDEX IF EXISTS idx_recon_rule_profile_priority;
DROP TABLE IF EXISTS recon_rule;
