CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entry_trx_id_and_version
    ON entry (logical_transaction_id, transaction_version);
