-- Revert entry_identifier column type from TEXT to VARCHAR(64) in staging_entries table
ALTER TABLE staging_entry ALTER COLUMN entry_identifier TYPE VARCHAR(64);

-- Revert entry_identifier column type from TEXT to VARCHAR(64) in entries table
ALTER TABLE entry ALTER COLUMN entry_identifier TYPE VARCHAR(64);
