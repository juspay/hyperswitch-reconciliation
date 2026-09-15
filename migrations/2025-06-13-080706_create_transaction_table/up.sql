-- Create transaction table
CREATE TABLE transactions (
    id VARCHAR(64) PRIMARY KEY,
    transaction_id VARCHAR(64) NOT NULL,
    entry_id TEXT[] NOT NULL,
    created_at TIMESTAMP NOT NULL,
    discarded_at TIMESTAMP,
    last_modified_at TIMESTAMP NOT NULL,
    discarded_status "TransactionStatus",
    transaction_status "TransactionStatus" NOT NULL DEFAULT 'expected',
    amount BIGINT NOT NULL,
    currency "Currency" NOT NULL,
    version INT NOT NULL,
    profile_id VARCHAR(64) NOT NULL
);

-- Create unique index on (transaction_id, version)
CREATE UNIQUE INDEX idx_transaction_id_version
ON transactions (transaction_id, version);

-- Create index for common query patterns
CREATE INDEX idx_transaction_profile_status
ON transactions (profile_id, transaction_status);
