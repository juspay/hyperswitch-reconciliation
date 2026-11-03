CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entry_linking_pending_created_at_id
    ON entry (created_at, id)
    WHERE linking_status = 'pending';
