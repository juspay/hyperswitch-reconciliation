-- Create merchant_account table
CREATE TABLE merchant_account (
    merchant_id VARCHAR(64) PRIMARY KEY,
    merchant_name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);
