-- Remove index for account_id column
DROP INDEX IF EXISTS idx_ingestion_history_account_id;

-- Remove account_id column from ingestion_history table
ALTER TABLE ingestion_history 
DROP COLUMN account_id;
