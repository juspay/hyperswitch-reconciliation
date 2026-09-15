-- Drop the index first
DROP INDEX IF EXISTS idx_staging_entries_profile_id;

-- Remove the profile_id column
ALTER TABLE staging_entries DROP COLUMN profile_id;
