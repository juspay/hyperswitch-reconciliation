CREATE TABLE search_entry_link_key (
    id VARCHAR(64) PRIMARY KEY,
    profile_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    link_config_id VARCHAR(64) NOT NULL,
    entry_id VARCHAR(64) NOT NULL,
    rule_id VARCHAR(64) NOT NULL,
    target_side TEXT NOT NULL,
    status TEXT NOT NULL,
    target_key TEXT NOT NULL,
    encrypted_value BYTEA NOT NULL,
    hashed_value VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX uniq_idx_search_entry_link_key_entry_config
    ON search_entry_link_key (entry_id, link_config_id);

CREATE INDEX idx_search_entry_link_key_config_target_value
    ON search_entry_link_key (link_config_id, target_key, hashed_value);
