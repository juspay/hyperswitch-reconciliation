-- Rename id columns to more descriptive names

-- Rename id to recon_rule_source_id in recon_rule_source table
ALTER TABLE recon_rule_source RENAME COLUMN id TO recon_rule_source_id;

-- Rename id to recon_rule_target_id in recon_rule_target table
ALTER TABLE recon_rule_target RENAME COLUMN id TO recon_rule_target_id;

-- Rename id to search_expectation_id in search_expectations table
ALTER TABLE search_expectations RENAME COLUMN id TO search_expectation_id;

-- Rename id to transformation_id in transformation_configs table
ALTER TABLE transformation_configs RENAME COLUMN id TO transformation_id;

-- Rename id to transformation_history_id in transformation_history table
ALTER TABLE transformation_history RENAME COLUMN id TO transformation_history_id;
