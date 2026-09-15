-- Drop indexes and columns added in up.sql
DROP INDEX idx_entries_order_id;
ALTER TABLE entries DROP COLUMN order_id;

DROP INDEX idx_staging_entries_order_id;
ALTER TABLE staging_entries DROP COLUMN order_id;
