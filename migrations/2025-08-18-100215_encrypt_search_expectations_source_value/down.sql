-- Drop indexes
DROP INDEX IF EXISTS idx_search_expectations_match_lookup;

-- Remove encrypted fields and merchant_id
ALTER TABLE search_expectations DROP COLUMN merchant_id;
ALTER TABLE search_expectations DROP COLUMN hashed_source_value;
ALTER TABLE search_expectations DROP COLUMN encrypted_source_value;

-- Restore plain text source_value column
ALTER TABLE search_expectations ADD COLUMN source_value VARCHAR(128) NOT NULL;

-- Recreate original indexes
CREATE INDEX idx_expectation_source ON search_expectations(source_account_id, source_key, source_value);
CREATE INDEX idx_expectation_match ON search_expectations(target_account_id, rule_id, target_key, source_value, status);
