CREATE TABLE search_entry_key (
    id                     VARCHAR(64) PRIMARY KEY,
    profile_id             VARCHAR(64) NOT NULL,
    entry_id               VARCHAR(64) NOT NULL,
    primary_transaction_id VARCHAR(64) NOT NULL,
    metadata_key           TEXT NOT NULL,
    metadata_value         TEXT,
    created_at             TIMESTAMP NOT NULL,
    last_modified_at       TIMESTAMP NOT NULL
);

CREATE INDEX idx_sek_profile_id_key_value ON search_entry_key (profile_id, metadata_key, metadata_value);
CREATE INDEX idx_sek_entry_id ON search_entry_key (entry_id);
