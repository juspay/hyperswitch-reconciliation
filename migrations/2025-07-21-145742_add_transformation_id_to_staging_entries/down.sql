-- Remove transformation_id and transformation_history_id columns from staging_entries table
DROP INDEX IF EXISTS idx_staging_entries_transformation_id;
DROP INDEX IF EXISTS idx_staging_entries_transformation_history_id;
ALTER TABLE staging_entries DROP COLUMN transformation_id;
ALTER TABLE staging_entries DROP COLUMN transformation_history_id;
