-- Remove data field from entries table
ALTER TABLE entries DROP COLUMN data;
ALTER TABLE entries DROP COLUMN discarded_data;
