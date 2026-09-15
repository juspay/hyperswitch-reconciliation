-- Revert the metadata column to allow NULL values
ALTER TABLE staging_entries ALTER COLUMN metadata DROP NOT NULL;
