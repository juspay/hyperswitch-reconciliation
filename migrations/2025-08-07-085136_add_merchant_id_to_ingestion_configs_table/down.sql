-- Remove merchant_id column from ingestion_configs table
ALTER TABLE ingestion_configs
DROP COLUMN merchant_id;
