-- Drop metadata_schemas table and associated indexes
DROP INDEX uniq_idx_metadata_schemas_schema_id_version;
DROP INDEX idx_metadata_schemas_account_id;
DROP INDEX idx_metadata_schemas_schema_id;
DROP INDEX idx_metadata_schemas_profile_id;
DROP TABLE metadata_schemas;

