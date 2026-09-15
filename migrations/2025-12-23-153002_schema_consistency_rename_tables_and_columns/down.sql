ALTER TABLE transformation_history RENAME COLUMN transformation_config_id TO transformation_id;
ALTER TABLE transformation_history RENAME COLUMN id TO transformation_history_id;

ALTER TABLE transformation_config RENAME COLUMN ingestion_config_id TO ingestion_id;
ALTER TABLE transformation_config RENAME COLUMN id TO transformation_id;

ALTER TABLE transaction RENAME COLUMN entry_ids TO entry_id;

ALTER TABLE staging_entry RENAME COLUMN entry_identifier TO order_id;
ALTER TABLE staging_entry RENAME COLUMN transformation_config_id TO transformation_id;
ALTER TABLE staging_entry RENAME COLUMN balance_direction TO entry_type;

ALTER TABLE search_expectation RENAME COLUMN logical_transaction_id TO transaction_id;
ALTER TABLE search_expectation RENAME COLUMN id TO search_expectation_id;

ALTER TABLE search_duplicate RENAME COLUMN transformation_config_id TO transformation_id;
ALTER TABLE search_duplicate RENAME COLUMN logical_staging_entry_id TO staging_entry_id;
ALTER TABLE search_duplicate RENAME COLUMN id TO search_duplicate_id;

ALTER TABLE recon_rule RENAME COLUMN description TO rule_description;
ALTER TABLE recon_rule RENAME COLUMN name TO rule_name;
ALTER TABLE recon_rule RENAME COLUMN id TO rule_id;

ALTER TABLE ingestion_history RENAME COLUMN ingestion_config_id TO ingestion_id;

ALTER TABLE ingestion_config RENAME COLUMN id TO ingestion_id;

ALTER TABLE entry RENAME COLUMN entry_identifier TO order_id;
ALTER TABLE entry RENAME COLUMN transformation_config_id TO transformation_id;
ALTER TABLE entry RENAME COLUMN primary_staging_entry_id TO staging_entry_id;
ALTER TABLE entry RENAME COLUMN transaction_version TO version;
ALTER TABLE entry RENAME COLUMN balance_direction TO entry_type;
ALTER TABLE entry RENAME COLUMN logical_transaction_id TO transaction_id;
ALTER TABLE entry RENAME COLUMN id TO entry_id;

ALTER TABLE account RENAME COLUMN name TO account_name;
ALTER TABLE account RENAME COLUMN balance_direction TO account_type;
ALTER TABLE account RENAME COLUMN id TO account_id;

ALTER TABLE transformation_config RENAME TO transformation_configs;
ALTER TABLE transaction RENAME TO transactions;
ALTER TABLE staging_entry RENAME TO staging_entries;
ALTER TABLE search_group RENAME TO search_groups;
ALTER TABLE search_expectation RENAME TO search_expectations;
ALTER TABLE search_duplicate RENAME TO search_duplicates;
ALTER TABLE metadata_schema RENAME TO metadata_schemas;
ALTER TABLE ingestion_config RENAME TO ingestion_configs;
ALTER TABLE entry RENAME TO entries;
