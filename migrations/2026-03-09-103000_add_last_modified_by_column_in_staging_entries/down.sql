-- Remove modified_by columns
ALTER TABLE staging_entry DROP COLUMN IF EXISTS modified_by;
