-- Add data JSONB field to entries table
ALTER TABLE entries
    ADD COLUMN data JSONB NOT NULL,
    ADD COLUMN discarded_data JSONB;

