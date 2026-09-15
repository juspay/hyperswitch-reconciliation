-- Add processing_mode column to staging_entries table
ALTER TABLE staging_entries ADD COLUMN processing_mode "StagingEntryProcessingMode" NOT NULL;
