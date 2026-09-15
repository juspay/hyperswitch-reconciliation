-- Remove index and rule_id column from transactions table
DROP INDEX IF EXISTS idx_transactions_rule_id;
ALTER TABLE transactions DROP COLUMN rule_id;
