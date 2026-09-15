-- Create staging_entries table
CREATE TABLE staging_entries (
    staging_entry_id VARCHAR(64) PRIMARY KEY,
    account_id VARCHAR(64) NOT NULL,
    entry_type "BalanceDirection" NOT NULL,
    amount BIGINT NOT NULL,
    currency "Currency" NOT NULL,
    status "StagingEntryStatus" NOT NULL,
    effective_at TIMESTAMP NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Create indexes for common query patterns
CREATE INDEX idx_staging_entries_account_id ON staging_entries(account_id);
CREATE INDEX idx_staging_entries_status ON staging_entries(status);
