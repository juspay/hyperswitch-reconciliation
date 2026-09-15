-- Create recon_target table
CREATE TABLE recon_rule_target (
    id VARCHAR(64) PRIMARY KEY,
    rule_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    match_rules JSONB NOT NULL,
    search_identifier JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create indexes for performance
CREATE INDEX idx_recon_rule_target_rule_id ON recon_rule_target(rule_id);
CREATE INDEX idx_recon_rule_target_account_id ON recon_rule_target(account_id);
