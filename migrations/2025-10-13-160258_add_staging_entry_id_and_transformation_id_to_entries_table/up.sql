-- Add staging entry tracking fields to entries table
-- This creates a complete audit trail linking entries back to their source staging entries and transformations

-- Add columns
ALTER TABLE entries
ADD COLUMN staging_entry_id VARCHAR(64),
ADD COLUMN transformation_id VARCHAR(64);

-- Add indexes for query performance
-- Using partial indexes to only index non-NULL values (saves space for manual transactions)
CREATE INDEX idx_entries_staging_entry_id ON entries (staging_entry_id)
WHERE
    staging_entry_id IS NOT NULL;

CREATE INDEX idx_entries_transformation_id ON entries (transformation_id)
WHERE
    transformation_id IS NOT NULL;