-- Your SQL goes here
-- Add nullable JSONB config column to recon_rule to store ReconStrategy
ALTER TABLE recon_rule
ADD COLUMN strategy JSONB NOT NULL;

-- Create ReconRuleAccountType enum to represent account role in a rule
CREATE TYPE "ReconRuleAccountType" AS ENUM (
	'source',
	'target'
);

-- Create recon_rule_account table to map rules to accounts with a role
CREATE TABLE recon_rule_account (
    id VARCHAR(64) PRIMARY KEY,
    account_id VARCHAR(64) NOT NULL,
    rule_id VARCHAR(64) NOT NULL,
    account_type "ReconRuleAccountType" NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Indexes to speed up lookups by rule and account
CREATE INDEX idx_recon_rule_account_id_account_type ON recon_rule_account(account_id, account_type);
