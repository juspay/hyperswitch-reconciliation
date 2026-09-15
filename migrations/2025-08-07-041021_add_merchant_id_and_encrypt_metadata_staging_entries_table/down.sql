-- Change metadata column type back from BYTEA to JSONB
ALTER TABLE staging_entries
ALTER COLUMN metadata TYPE JSONB
USING convert_from(metadata, 'UTF8')::jsonb;

-- Remove merchant_id column
ALTER TABLE staging_entries
DROP COLUMN merchant_id;
