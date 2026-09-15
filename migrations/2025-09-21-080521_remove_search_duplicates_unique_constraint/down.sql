-- Remove added columns
ALTER TABLE search_duplicates DROP COLUMN IF EXISTS staging_entry_version;
ALTER TABLE search_duplicates DROP COLUMN IF EXISTS primary_staging_entry_id;

-- Restore unique constraint (note: this will fail if duplicate entries exist)
DROP INDEX IF EXISTS idx_search_duplicates_lookup;

-- Recreate unique index
CREATE UNIQUE INDEX IF NOT EXISTS uniq_idx_search_duplicates ON search_duplicates (
  transformation_id, field_path, hashed_field_value
);
