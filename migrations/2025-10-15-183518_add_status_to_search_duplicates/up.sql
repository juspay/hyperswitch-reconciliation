-- Create SearchDuplicateStatus enum
CREATE TYPE "SearchDuplicateStatus" AS ENUM (
    'active',
    'archived'
);

-- Add status column to search_duplicates table
ALTER TABLE search_duplicates
ADD COLUMN status "SearchDuplicateStatus" NOT NULL DEFAULT 'active';

-- Add unique index on primary_staging_entry_id.
CREATE UNIQUE INDEX uniq_idx_primary_staging_entry_id
ON search_duplicates (primary_staging_entry_id);

-- Drop existing indexes
DROP INDEX IF EXISTS idx_search_duplicates_lookup;
DROP INDEX IF EXISTS idx_search_duplicates_staging_entry;

-- Create partial indexes for active records only
CREATE INDEX idx_search_duplicates_lookup_active 
ON search_duplicates (transformation_id, field_path, hashed_field_value) 
WHERE status = 'active';

CREATE INDEX idx_search_duplicates_staging_entry_active 
ON search_duplicates (staging_entry_id) 
WHERE status = 'active';
