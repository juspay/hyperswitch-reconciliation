-- Drop recon_rule_source and recon_rule_target tables
-- These tables are deprecated as part of V1 recon engine removal

-- Drop indexes first
DROP INDEX IF EXISTS idx_recon_rule_source_rule_id;
DROP INDEX IF EXISTS idx_recon_rule_source_account_id;
DROP INDEX IF EXISTS idx_recon_rule_target_rule_id;
DROP INDEX IF EXISTS idx_recon_rule_target_account_id;

-- Drop the tables
DROP TABLE IF EXISTS recon_rule_source;
DROP TABLE IF EXISTS recon_rule_target;