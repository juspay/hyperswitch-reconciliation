-- Add metadata_schema_id column to transformation_configs table
-- This allows direct referencing of metadata schemas from transformation configs
-- eliminating the need to store metadata_schema_id in the config JSON

ALTER TABLE transformation_configs 
ADD COLUMN metadata_schema_id VARCHAR(64) NOT NULL;

-- Create index for better query performance on joins
CREATE INDEX idx_transformation_configs_metadata_schema_id 
ON transformation_configs(metadata_schema_id);