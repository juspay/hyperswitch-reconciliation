-- Change entry_identifier column type from VARCHAR(64) to TEXT in entries table
-- This allows for more flexible identifier storage without length restrictions
ALTER TABLE entry ALTER COLUMN entry_identifier TYPE TEXT;

-- Change entry_identifier column type from VARCHAR(64) to TEXT in staging_entries table
ALTER TABLE staging_entry ALTER COLUMN entry_identifier TYPE TEXT;
