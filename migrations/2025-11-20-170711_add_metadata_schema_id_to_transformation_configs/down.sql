-- Remove metadata_schema_id column and related constraints from transformation_configs table
-- This reverts the migration that moved metadata_schema_id from config JSON to a dedicated column

-- Drop the index first
DROP INDEX IF EXISTS idx_transformation_configs_metadata_schema_id;

-- Drop the metadata_schema_id column
ALTER TABLE transformation_configs 
DROP COLUMN IF EXISTS metadata_schema_id;