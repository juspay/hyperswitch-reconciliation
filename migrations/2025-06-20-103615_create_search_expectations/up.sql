-- Create search expectations table
CREATE TABLE search_expectations (
    id VARCHAR(64) PRIMARY KEY,
    expectation_id VARCHAR(64) NOT NULL,
    rule_id VARCHAR(64) NOT NULL,
    source_account_id VARCHAR(64) NOT NULL,
    target_account_id VARCHAR(64) NOT NULL,
    source_key VARCHAR(128) NOT NULL,
    source_value VARCHAR(128) NOT NULL,
    target_key VARCHAR(128) NOT NULL,
    status "ExpectationStatus" NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create index for source lookups
CREATE INDEX idx_expectation_source ON search_expectations (
    source_account_id, source_key, source_value
);

-- Create index for matching expectations with Expected status
CREATE INDEX idx_expectation_match ON search_expectations (
    target_account_id, rule_id, target_key, source_value
)
WHERE status = 'expected';
