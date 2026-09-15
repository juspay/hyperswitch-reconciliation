-- Revert versioning support from staging_entries table

-- Step 1: Drop the versioning index
DROP INDEX uniq_idx_staging_entry_id_version;

-- Step 2: Drop the new primary key constraint
ALTER TABLE staging_entries DROP CONSTRAINT staging_entries_pkey;

-- Step 3: Restore the old primary key on staging_entry_id
ALTER TABLE staging_entries ADD PRIMARY KEY (staging_entry_id);

-- Step 4: Drop the versioning columns
ALTER TABLE staging_entries 
DROP COLUMN IF EXISTS discarded_data,
DROP COLUMN IF EXISTS discarded_status,
DROP COLUMN IF EXISTS discarded_at,
DROP COLUMN IF EXISTS version,
DROP COLUMN IF EXISTS id;
