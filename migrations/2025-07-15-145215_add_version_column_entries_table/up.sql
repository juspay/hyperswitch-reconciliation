-- Add version column to entries table
ALTER TABLE entries ADD COLUMN version INT4 NOT NULL DEFAULT 1;

-- Create index for version field
CREATE INDEX idx_entries_version ON entries(version);
