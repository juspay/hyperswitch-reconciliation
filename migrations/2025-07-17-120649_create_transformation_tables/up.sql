-- Create enums
CREATE TYPE "TransformationStatus" AS ENUM ('pending', 'processing', 'processed', 'failed');

-- Create transformation_configs table
CREATE TABLE transformation_configs (
    id VARCHAR(64) PRIMARY KEY,
    profile_id VARCHAR(64) NOT NULL,
    ingestion_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    name VARCHAR(255) NOT NULL,
    config JSONB NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create transformation_history table
CREATE TABLE transformation_history (
    id VARCHAR(64) PRIMARY KEY,
    transformation_id VARCHAR(64) NOT NULL,
    ingestion_history_id VARCHAR(64) NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    status "TransformationStatus" NOT NULL,
    data JSONB,
    processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create indexes for performance
CREATE INDEX idx_transformation_configs_profile_id ON transformation_configs(profile_id);
CREATE INDEX idx_transformation_configs_ingestion_id ON transformation_configs(ingestion_id);
CREATE INDEX idx_transformation_configs_account_id ON transformation_configs(account_id);
CREATE INDEX idx_transformation_configs_is_active ON transformation_configs(is_active);

CREATE INDEX idx_transformation_history_ingestion_history_id ON transformation_history(ingestion_history_id);
CREATE INDEX idx_transformation_history_transformation_id ON transformation_history(transformation_id);
CREATE INDEX idx_transformation_history_profile_id ON transformation_history(profile_id);
CREATE INDEX idx_transformation_history_account_id ON transformation_history(account_id);
CREATE INDEX idx_transformation_history_status ON transformation_history(status);
CREATE INDEX idx_transformation_history_created_at ON transformation_history(created_at);
