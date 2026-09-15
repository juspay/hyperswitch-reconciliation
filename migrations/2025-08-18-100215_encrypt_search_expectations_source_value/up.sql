-- Drop old indexes that used source_value
DROP INDEX IF EXISTS idx_expectation_source;
DROP INDEX IF EXISTS idx_expectation_match;

-- Remove the plain text source_value column
ALTER TABLE search_expectations DROP COLUMN source_value;

-- Add encrypted and hashed source value fields after source_key
ALTER TABLE search_expectations 
ADD COLUMN encrypted_source_value BYTEA NOT NULL,
ADD COLUMN hashed_source_value VARCHAR(64) NOT NULL;

-- Add merchant_id at the end
ALTER TABLE search_expectations 
ADD COLUMN merchant_id VARCHAR(64) NOT NULL;

CREATE INDEX idx_search_expectations_match_lookup ON search_expectations(target_account_id, rule_id, target_key, hashed_source_value, status);
