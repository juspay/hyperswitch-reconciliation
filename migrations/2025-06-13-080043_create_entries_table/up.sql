-- Create entries table
CREATE TABLE entries (
    entry_id VARCHAR(64) PRIMARY KEY,
    account_id VARCHAR(64) NOT NULL,
    transaction_id VARCHAR(64) NOT NULL,
    entry_type "BalanceDirection" NOT NULL,
    amount BIGINT NOT NULL,
    currency "Currency" NOT NULL,
    status "EntryStatus" NOT NULL,
    effective_at TIMESTAMP NOT NULL,
    metadata JSONB,
    discarded_at TIMESTAMP,
    discarded_status "EntryStatus",
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create indexes for common query patterns
CREATE INDEX idx_entries_account_id ON entries(account_id);
CREATE INDEX idx_entries_transaction_id ON entries(transaction_id);
CREATE INDEX idx_entries_status ON entries(status);