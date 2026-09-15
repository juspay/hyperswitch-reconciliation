-- This file should undo anything in `up.sql`
-- Drop indexes created in up.sql
DROP INDEX IF EXISTS idx_recon_rule_account_id_account_type;

-- Drop recon_rule_account table
DROP TABLE IF EXISTS recon_rule_account;

-- Drop ReconRuleAccountType enum
DROP TYPE IF EXISTS "ReconRuleAccountType";

-- Remove config column from recon_rule
ALTER TABLE recon_rule
DROP COLUMN IF EXISTS strategy;
