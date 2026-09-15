-- Add data JSONB field to staging_entries table
ALTER TABLE staging_entries ADD COLUMN data JSONB NOT NULL;

-- Add data JSONB field to transactions table
ALTER TABLE transactions ADD COLUMN data JSONB NOT NULL;

-- Add discarded_data JSONB field to transactions table for storing original data when archiving
ALTER TABLE transactions ADD COLUMN discarded_data JSONB;
