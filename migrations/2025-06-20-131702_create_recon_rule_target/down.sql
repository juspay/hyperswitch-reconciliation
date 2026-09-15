-- Drop indexes
DROP INDEX IF EXISTS idx_recon_rule_target_rule_id;
DROP INDEX IF EXISTS idx_recon_rule_target_account_id;

-- Drop recon_target table
DROP TABLE IF EXISTS recon_rule_target;
