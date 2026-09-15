-- Drop indexes
DROP INDEX IF EXISTS idx_recon_rule_source_rule_id;
DROP INDEX IF EXISTS idx_recon_rule_source_account_id;

-- Drop the recon_rule_source table
DROP TABLE recon_rule_source;
