-- Add profile_id column to staging_entries table as NOT NULL
ALTER TABLE staging_entries ADD COLUMN profile_id VARCHAR(64) NOT NULL;

-- Create index for efficient querying
CREATE INDEX idx_staging_entries_profile_id ON staging_entries(profile_id);
