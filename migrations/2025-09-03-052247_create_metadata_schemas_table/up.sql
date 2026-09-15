-- Create metadata_schemas table for storing validation schemas linked to transformation configs
CREATE TABLE metadata_schemas (
    id VARCHAR(64) PRIMARY KEY,
    schema_id VARCHAR(64) NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    version INT NOT NULL,
    schema_data JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_metadata_schemas_profile_id ON metadata_schemas(profile_id);
CREATE INDEX idx_metadata_schemas_schema_id ON metadata_schemas(schema_id);
CREATE INDEX idx_metadata_schemas_account_id ON metadata_schemas(account_id);
CREATE UNIQUE INDEX uniq_idx_metadata_schemas_schema_id_version ON metadata_schemas(schema_id, version);
