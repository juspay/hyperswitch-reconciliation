CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entries_transaction_id
    ON entry (logical_transaction_id);
