CREATE TABLE link_config (
    id VARCHAR(64) PRIMARY KEY,
    profile_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    transaction_rule_id VARCHAR(64) NOT NULL,
    confirmation_rule_id VARCHAR(64) NOT NULL,
    transaction_trigger JSONB NOT NULL,
    confirmation_trigger JSONB NOT NULL,
    link_search_identifier JSONB NOT NULL,
    priority SMALLINT NOT NULL,
    is_active BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_link_config_active_profile_account
    ON link_config (profile_id, account_id)
    WHERE is_active = true;
