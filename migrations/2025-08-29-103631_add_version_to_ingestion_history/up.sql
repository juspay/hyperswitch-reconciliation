-- Add id column as new primary key and version columns for audit tracking
ALTER TABLE ingestion_history ADD COLUMN id VARCHAR(64) NOT NULL;
ALTER TABLE ingestion_history ADD COLUMN version INT4 NOT NULL;
ALTER TABLE ingestion_history ADD COLUMN discarded_at TIMESTAMP;
ALTER TABLE ingestion_history ADD COLUMN discarded_status "IngestionStatus";

ALTER TYPE "IngestionStatus" ADD VALUE 'discarded';

-- Change primary key from ingestion_history_id to id
ALTER TABLE ingestion_history DROP CONSTRAINT ingestion_history_pkey;
ALTER TABLE ingestion_history ADD PRIMARY KEY (id);

-- Create unique index on (ingestion_history_id, version) for versioning support
CREATE UNIQUE INDEX uniq_idx_ingestion_history_id_version 
ON ingestion_history (ingestion_history_id, version);

-- Create index for version field for query optimization
CREATE INDEX idx_ingestion_history_version ON ingestion_history(version);
