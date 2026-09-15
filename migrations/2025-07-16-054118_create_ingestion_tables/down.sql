-- Drop indexes
DROP INDEX IF EXISTS idx_ingestion_history_created_at;
DROP INDEX IF EXISTS idx_ingestion_history_status;
DROP INDEX IF EXISTS idx_ingestion_history_ingestion_id;

DROP INDEX IF EXISTS idx_ingestion_configs_is_active;
DROP INDEX IF EXISTS idx_ingestion_configs_account_id;
DROP INDEX IF EXISTS idx_ingestion_configs_profile_id;

-- Drop tables
DROP TABLE IF EXISTS ingestion_history;
DROP TABLE IF EXISTS ingestion_configs;

-- Drop enum types
DROP TYPE IF EXISTS "IngestionStatus";
