-- Add account_id column to ingestion_history table
ALTER TABLE ingestion_history 
ADD COLUMN account_id VARCHAR(64) NOT NULL;

-- Create index for the new account_id column
CREATE INDEX idx_ingestion_history_account_id ON ingestion_history(account_id);
