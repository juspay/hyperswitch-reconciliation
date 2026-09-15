-- Add transaction_id and transaction_version columns to search_expectations table
ALTER TABLE search_expectations
    ADD COLUMN transaction_id VARCHAR(64) NOT NULL,
    ADD COLUMN transaction_version INT4 NOT NULL;

ALTER TABLE search_expectations
    DROP COLUMN source_key,
    DROP COLUMN source_account_id;

-- Create index for transaction lookups
CREATE INDEX idx_search_expectation_transaction ON search_expectations (transaction_id, transaction_version);
