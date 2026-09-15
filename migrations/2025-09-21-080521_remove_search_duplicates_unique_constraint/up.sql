-- Remove unique constraint to allow multiple duplicate entries
DROP INDEX IF EXISTS uniq_idx_search_duplicates;

-- Add primary_staging_entry_id field to track logical staging entry ID
ALTER TABLE search_duplicates ADD COLUMN primary_staging_entry_id VARCHAR(64) NOT NULL;

-- Add staging_entry_version field to track staging entry version
ALTER TABLE search_duplicates ADD COLUMN staging_entry_version INT4 NOT NULL;

-- Add non-unique index for efficient duplicate detection lookups
CREATE INDEX IF NOT EXISTS idx_search_duplicates_lookup 
ON search_duplicates (transformation_id, field_path, hashed_field_value);
