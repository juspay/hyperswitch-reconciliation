ALTER TABLE entries RENAME TO entry;
ALTER TABLE ingestion_configs RENAME TO ingestion_config;
ALTER TABLE metadata_schemas RENAME TO metadata_schema;
ALTER TABLE search_duplicates RENAME TO search_duplicate;
ALTER TABLE search_expectations RENAME TO search_expectation;
ALTER TABLE search_groups RENAME TO search_group;
ALTER TABLE staging_entries RENAME TO staging_entry;
ALTER TABLE transactions RENAME TO transaction;
ALTER TABLE transformation_configs RENAME TO transformation_config;

ALTER TABLE account RENAME COLUMN account_id TO id;
ALTER TABLE account RENAME COLUMN account_type TO balance_direction;
ALTER TABLE account RENAME COLUMN account_name TO name;

ALTER TABLE entry RENAME COLUMN entry_id TO id;
ALTER TABLE entry RENAME COLUMN transaction_id TO logical_transaction_id;
ALTER TABLE entry RENAME COLUMN entry_type TO balance_direction;
ALTER TABLE entry RENAME COLUMN version TO transaction_version;
ALTER TABLE entry RENAME COLUMN staging_entry_id TO primary_staging_entry_id;
ALTER TABLE entry RENAME COLUMN transformation_id TO transformation_config_id;
ALTER TABLE entry RENAME COLUMN order_id TO entry_identifier;

ALTER TABLE ingestion_config RENAME COLUMN ingestion_id TO id;

ALTER TABLE ingestion_history RENAME COLUMN ingestion_id TO ingestion_config_id;

ALTER TABLE recon_rule RENAME COLUMN rule_id TO id;
ALTER TABLE recon_rule RENAME COLUMN rule_name TO name;
ALTER TABLE recon_rule RENAME COLUMN rule_description TO description;

ALTER TABLE search_duplicate RENAME COLUMN search_duplicate_id TO id;
ALTER TABLE search_duplicate RENAME COLUMN staging_entry_id TO logical_staging_entry_id;
ALTER TABLE search_duplicate RENAME COLUMN transformation_id TO transformation_config_id;

ALTER TABLE search_expectation RENAME COLUMN search_expectation_id TO id;
ALTER TABLE search_expectation RENAME COLUMN transaction_id TO logical_transaction_id;

ALTER TABLE staging_entry RENAME COLUMN entry_type TO balance_direction;
ALTER TABLE staging_entry RENAME COLUMN transformation_id TO transformation_config_id;
ALTER TABLE staging_entry RENAME COLUMN order_id TO entry_identifier;

ALTER TABLE transaction RENAME COLUMN entry_id TO entry_ids;

ALTER TABLE transformation_config RENAME COLUMN transformation_id TO id;
ALTER TABLE transformation_config RENAME COLUMN ingestion_id TO ingestion_config_id;

ALTER TABLE transformation_history RENAME COLUMN transformation_history_id TO id;
ALTER TABLE transformation_history RENAME COLUMN transformation_id TO transformation_config_id;
