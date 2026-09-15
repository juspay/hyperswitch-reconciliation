-- Change metadata column type back from BYTEA to JSONB
ALTER TABLE entries
ALTER COLUMN metadata TYPE JSONB
USING convert_from(metadata, 'UTF8')::jsonb;

-- Remove merchant_id column
ALTER TABLE entries
DROP COLUMN merchant_id;
