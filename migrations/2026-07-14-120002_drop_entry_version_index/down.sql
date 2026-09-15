CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entries_version
    ON entry (transaction_version);
