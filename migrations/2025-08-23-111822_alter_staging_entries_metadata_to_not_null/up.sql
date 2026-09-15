-- Alter the metadata column to be NOT NULL
ALTER TABLE staging_entries ALTER COLUMN metadata SET NOT NULL;
