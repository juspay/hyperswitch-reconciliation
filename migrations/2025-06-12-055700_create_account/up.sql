-- Your SQL goes here

-- Create account table using existing BalanceDirection enum
CREATE TABLE account (
    account_id VARCHAR(64) PRIMARY KEY,
    account_type "BalanceDirection" NOT NULL,
    account_name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL,
    initial_balance BIGINT NOT NULL,
    pending_balance BIGINT NOT NULL,
    posted_balance  BIGINT NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    currency "Currency" NOT NULL
);

-- Create indexes
CREATE INDEX idx_account_profile_id ON account(profile_id);