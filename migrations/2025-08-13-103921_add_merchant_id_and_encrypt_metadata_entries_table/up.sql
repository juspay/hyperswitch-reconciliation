-- Add merchant_id column
ALTER TABLE entries
ADD COLUMN merchant_id VARCHAR(64) NOT NULL;

-- Change metadata column type from JSONB to BYTEA
ALTER TABLE entries
ALTER COLUMN metadata TYPE BYTEA
USING convert_to(metadata::text, 'UTF8');
