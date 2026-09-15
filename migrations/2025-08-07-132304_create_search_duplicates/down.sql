-- Drop the indexes first
DROP INDEX IF EXISTS idx_search_duplicates_staging_entry;
DROP INDEX IF EXISTS uniq_idx_search_duplicates;

-- Drop the table
DROP TABLE IF EXISTS search_duplicates;