-- Drop partial indexes
DROP INDEX IF EXISTS idx_search_duplicates_lookup_active;
DROP INDEX IF EXISTS idx_search_duplicates_staging_entry_active;

-- Drop unique index created in up.sql
DROP INDEX IF EXISTS uniq_idx_primary_staging_entry_id;

-- Recreate original indexes
CREATE INDEX IF NOT EXISTS idx_search_duplicates_lookup 
ON search_duplicates (transformation_id, field_path, hashed_field_value);

CREATE INDEX IF NOT EXISTS idx_search_duplicates_staging_entry 
ON search_duplicates (staging_entry_id);

-- Remove status column from search_duplicates table
ALTER TABLE search_duplicates
DROP COLUMN IF EXISTS status;

-- Drop SearchDuplicateStatus enum type
DROP TYPE IF EXISTS "SearchDuplicateStatus";
