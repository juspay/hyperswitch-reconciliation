-- Create business_profile table
CREATE TABLE business_profile (
    profile_id VARCHAR(64) PRIMARY KEY,
    profile_name TEXT NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create unique index on merchant_id and profile_id
CREATE UNIQUE INDEX idx_merchant_profile ON business_profile(merchant_id, profile_id);
