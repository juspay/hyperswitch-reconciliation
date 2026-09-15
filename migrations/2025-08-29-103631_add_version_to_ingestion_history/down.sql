-- Remove versioning and soft delete columns from ingestion_history table
DROP INDEX IF EXISTS uniq_idx_ingestion_history_id_version;
DROP INDEX IF EXISTS idx_ingestion_history_version;

-- Restore original primary key
ALTER TABLE ingestion_history DROP CONSTRAINT ingestion_history_pkey;
ALTER TABLE ingestion_history ADD PRIMARY KEY (ingestion_history_id);

ALTER TABLE ingestion_history DROP COLUMN IF EXISTS id;
ALTER TABLE ingestion_history DROP COLUMN IF EXISTS version;
ALTER TABLE ingestion_history DROP COLUMN IF EXISTS discarded_at;
ALTER TABLE ingestion_history DROP COLUMN IF EXISTS discarded_status;
