-- Add transformation_id column to staging_entries table as NOT NULL
ALTER TABLE staging_entries ADD COLUMN transformation_id VARCHAR(64) NOT NULL;

-- Add transformation_history_id column to staging_entries table as nullable
ALTER TABLE staging_entries ADD COLUMN transformation_history_id VARCHAR(64);

-- Create index for efficient querying by transformation_id
CREATE INDEX idx_staging_entries_transformation_id ON staging_entries(transformation_id);

-- Create index for efficient querying by transformation_history_id
CREATE INDEX idx_staging_entries_transformation_history_id ON staging_entries(transformation_history_id);
