CREATE TABLE entry_link (
    id VARCHAR(64) PRIMARY KEY,
    profile_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    link_config_id VARCHAR(64) NOT NULL,
    transaction_entry_id VARCHAR(64) NOT NULL,
    confirmation_entry_id VARCHAR(64) NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX uniq_idx_entry_link_config_conf_tran
    ON entry_link (link_config_id, confirmation_entry_id, transaction_entry_id);

CREATE INDEX idx_entry_link_transaction_entry_id
    ON entry_link (transaction_entry_id);

CREATE INDEX idx_entry_link_confirmation_entry_id
    ON entry_link (confirmation_entry_id);
