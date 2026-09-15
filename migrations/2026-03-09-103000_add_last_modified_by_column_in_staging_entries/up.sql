-- Add last_modified_by column to staging_entry table
ALTER TABLE staging_entry ADD COLUMN modified_by VARCHAR(64);
