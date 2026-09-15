-- This file should undo anything in `up.sql`

-- Drop index for transaction lookups
DROP INDEX IF EXISTS idx_search_expectation_transaction;

ALTER TABLE search_expectations
    ADD COLUMN source_key VARCHAR(128) NOT NULL,
    ADD COLUMN source_account_id VARCHAR(64) NOT NULL;

-- Remove transaction_id and transaction_version columns from search_expectations table
ALTER TABLE search_expectations DROP COLUMN IF EXISTS transaction_version;
ALTER TABLE search_expectations DROP COLUMN IF EXISTS transaction_id;
