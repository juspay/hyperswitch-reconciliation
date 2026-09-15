-- Add versioning support to staging_entries table

-- Step 1: Add 'archived' status to StagingEntryStatus enum (required for versioning)
ALTER TYPE "StagingEntryStatus" ADD VALUE 'archived';

-- Step 2: Add new columns needed for versioning
ALTER TABLE staging_entries 
ADD COLUMN id VARCHAR(64) NOT NULL,
ADD COLUMN version INT NOT NULL,
ADD COLUMN discarded_at TIMESTAMP,
ADD COLUMN discarded_status "StagingEntryStatus",
ADD COLUMN discarded_data JSONB;

-- Step 3: Drop the old primary key constraint
ALTER TABLE staging_entries DROP CONSTRAINT staging_entries_pkey;

-- Step 4: Add new primary key on id column
ALTER TABLE staging_entries ADD PRIMARY KEY (id);

-- Step 5: Create unique index on (staging_entry_id, version) for logical ID versioning
CREATE UNIQUE INDEX uniq_idx_staging_entry_id_version
ON staging_entries (staging_entry_id, version);
