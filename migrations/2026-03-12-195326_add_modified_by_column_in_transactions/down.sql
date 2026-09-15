-- Remove modified_by column
ALTER TABLE transaction DROP COLUMN IF EXISTS modified_by;
