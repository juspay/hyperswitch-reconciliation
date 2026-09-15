-- Remove index for version field
DROP INDEX IF EXISTS idx_entries_version;

-- Remove version column from entries table
ALTER TABLE entries DROP COLUMN version;
