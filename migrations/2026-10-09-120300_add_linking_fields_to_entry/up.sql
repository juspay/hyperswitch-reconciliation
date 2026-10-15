ALTER TABLE entry
    ADD COLUMN linking_status VARCHAR(64),
    ADD COLUMN linking_data JSONB;

UPDATE entry
SET linking_status = CASE
        WHEN status = 'expected' THEN 'not_applicable'
        ELSE 'failed'
    END,
    linking_data = CASE
        WHEN status = 'expected' THEN '{"data_version":"v1","status":"not_applicable"}'::jsonb
        ELSE '{"data_version":"v1","status":"failed","failure_data_version":"v1","error_type":{"error_type_version":"v1","error_type":"no_link_config_found"}}'::jsonb
    END;

ALTER TABLE entry
    ALTER COLUMN linking_status SET NOT NULL,
    ALTER COLUMN linking_data SET NOT NULL;

CREATE INDEX idx_entry_linking_pending_created_at_id
    ON entry (created_at, id)
    WHERE linking_status = 'pending';

