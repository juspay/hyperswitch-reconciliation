-- Drop indexes
DROP INDEX IF EXISTS idx_transformation_history_created_at;
DROP INDEX IF EXISTS idx_transformation_history_status;
DROP INDEX IF EXISTS idx_transformation_history_account_id;
DROP INDEX IF EXISTS idx_transformation_history_profile_id;
DROP INDEX IF EXISTS idx_transformation_history_transformation_id;
DROP INDEX IF EXISTS idx_transformation_history_ingestion_history_id;

DROP INDEX IF EXISTS idx_transformation_configs_is_active;
DROP INDEX IF EXISTS idx_transformation_configs_account_id;
DROP INDEX IF EXISTS idx_transformation_configs_ingestion_id;
DROP INDEX IF EXISTS idx_transformation_configs_profile_id;

-- Drop tables
DROP TABLE IF EXISTS transformation_history;
DROP TABLE IF EXISTS transformation_configs;

-- Drop enums
DROP TYPE IF EXISTS "TransformationStatus";
