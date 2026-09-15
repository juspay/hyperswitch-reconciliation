-- Create enum types
CREATE TYPE "IngestionStatus" AS ENUM ('pending', 'processing', 'processed', 'failed');

-- Create ingestion_configs table
CREATE TABLE IF NOT EXISTS ingestion_configs (
    ingestion_id VARCHAR(64) PRIMARY KEY,
    profile_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    config JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create ingestion_history table
CREATE TABLE IF NOT EXISTS ingestion_history (
    ingestion_history_id VARCHAR(64) PRIMARY KEY,
    ingestion_id VARCHAR(64) NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    status "IngestionStatus" NOT NULL,
    data JSONB,
    processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL,
    trigger_data JSONB,
    raw_payload JSONB NOT NULL
);

-- Create indexes
CREATE INDEX idx_ingestion_configs_profile_id ON ingestion_configs(profile_id);
CREATE INDEX idx_ingestion_configs_account_id ON ingestion_configs(account_id);
CREATE INDEX idx_ingestion_configs_is_active ON ingestion_configs(is_active);

CREATE INDEX idx_ingestion_history_ingestion_id ON ingestion_history(ingestion_id);
CREATE INDEX idx_ingestion_history_status ON ingestion_history(status);
CREATE INDEX idx_ingestion_history_created_at ON ingestion_history(created_at);
