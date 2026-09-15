-- Rollback staging entry tracking fields from entries table

-- Remove indexes first
DROP INDEX IF EXISTS idx_entries_transformation_id;
DROP INDEX IF EXISTS idx_entries_staging_entry_id;

-- Remove columns
ALTER TABLE entries DROP COLUMN IF EXISTS transformation_id;
ALTER TABLE entries DROP COLUMN IF EXISTS staging_entry_id;
