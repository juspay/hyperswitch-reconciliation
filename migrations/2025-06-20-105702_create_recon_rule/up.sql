-- Your SQL goes here

-- Create recon_rule table
CREATE TABLE recon_rule (
    rule_id VARCHAR(64) PRIMARY KEY,
    rule_name VARCHAR(255) NOT NULL,
    rule_description TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL,
    is_active BOOLEAN NOT NULL,
    priority SMALLINT NOT NULL,
    profile_id VARCHAR(64) NOT NULL
);

-- Create composite index for profile_id and priority (covers queries without is_active filter)
CREATE INDEX idx_recon_rule_profile_priority ON recon_rule(profile_id, priority);

-- Create index including is_active for efficient filtered queries
CREATE INDEX idx_recon_rule_profile_active_priority ON recon_rule(profile_id, is_active, priority);
