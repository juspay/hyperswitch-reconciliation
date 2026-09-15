-- Add merchant_id column to ingestion_configs table
ALTER TABLE ingestion_configs
ADD COLUMN merchant_id VARCHAR(64) NOT NULL;