DROP INDEX IF EXISTS idx_entry_linking_pending_created_at_id;

ALTER TABLE entry
    DROP COLUMN linking_data,
    DROP COLUMN linking_status;
