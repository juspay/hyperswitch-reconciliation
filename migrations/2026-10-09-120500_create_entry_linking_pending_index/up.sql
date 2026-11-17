CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entry_linking_status
    ON entry (linking_status)
    WHERE linking_status = 'pending';
