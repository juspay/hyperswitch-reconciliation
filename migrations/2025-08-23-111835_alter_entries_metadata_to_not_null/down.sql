-- Revert the metadata column to allow NULL values
ALTER TABLE entries ALTER COLUMN metadata DROP NOT NULL;
