-- Create the recon_rule_source table
CREATE TABLE recon_rule_source (
    id VARCHAR(64) PRIMARY KEY,
    account_id VARCHAR(64) NOT NULL,
    rule_id VARCHAR(64) NOT NULL,
    trigger JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create indexes for performance
CREATE INDEX idx_recon_rule_source_account_id ON recon_rule_source(account_id);
CREATE INDEX idx_recon_rule_source_rule_id ON recon_rule_source(rule_id);
