-- Remove data JSONB field from staging_entries table
ALTER TABLE staging_entries DROP COLUMN data;

-- Remove data JSONB field from transactions table  
ALTER TABLE transactions DROP COLUMN data;

-- Remove discarded_data JSONB field from transactions table
ALTER TABLE transactions DROP COLUMN discarded_data;
