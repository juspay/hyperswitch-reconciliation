-- Drop staging_entries table and its indexes
DROP INDEX IF EXISTS idx_staging_entries_status;
DROP INDEX IF EXISTS idx_staging_entries_account_id;
DROP TABLE IF EXISTS staging_entries;
