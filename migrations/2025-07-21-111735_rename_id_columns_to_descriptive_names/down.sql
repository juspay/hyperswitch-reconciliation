-- Revert column names back to id

-- Revert recon_rule_source_id back to id in recon_rule_source table
ALTER TABLE recon_rule_source RENAME COLUMN recon_rule_source_id TO id;

-- Revert recon_rule_target_id back to id in recon_rule_target table
ALTER TABLE recon_rule_target RENAME COLUMN recon_rule_target_id TO id;

-- Revert search_expectation_id back to id in search_expectations table
ALTER TABLE search_expectations RENAME COLUMN search_expectation_id TO id;

-- Revert transformation_id back to id in transformation_configs table
ALTER TABLE transformation_configs RENAME COLUMN transformation_id TO id;

-- Revert transformation_history_id back to id in transformation_history table
ALTER TABLE transformation_history RENAME COLUMN transformation_history_id TO id;
