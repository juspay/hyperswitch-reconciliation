-- Drop entries table and its indexes
DROP INDEX IF EXISTS idx_entries_account_id;
DROP INDEX IF EXISTS idx_entries_transaction_id;
DROP INDEX IF EXISTS idx_entries_status;
DROP TABLE IF EXISTS entries;
