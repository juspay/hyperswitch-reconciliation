CREATE TYPE "SearchGroupStatus" AS ENUM (
    'active',
    'archived'
);

CREATE TABLE search_groups (
    id VARCHAR(64) PRIMARY KEY,
    rule_id VARCHAR(64) NOT NULL,
    primary_transaction_id VARCHAR(64) NOT NULL,
    encrypted_grouping_value BYTEA NOT NULL,
    hashed_grouping_value VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    status "SearchGroupStatus" NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_search_groups_lookup
ON search_groups (rule_id, hashed_grouping_value);

CREATE UNIQUE INDEX idx_primary_transaction_id ON search_groups (primary_transaction_id);