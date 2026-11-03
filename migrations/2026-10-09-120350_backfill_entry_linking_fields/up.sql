DO $$
DECLARE
    last_id VARCHAR(64) := '';
    batch_last_id VARCHAR(64);
    rows_updated BIGINT;
BEGIN
    LOOP
        WITH batch AS (
            SELECT id
            FROM entry
            WHERE id > last_id
              AND (linking_status IS NULL OR linking_data IS NULL)
            ORDER BY id
            LIMIT 5000
            FOR UPDATE
        ), updated AS (
            UPDATE entry AS e
            SET linking_status = COALESCE(e.linking_status, CASE
                    WHEN e.status = 'expected' THEN 'not_applicable'
                    ELSE 'failed'
                END),
                linking_data = COALESCE(e.linking_data, CASE
                    WHEN e.status = 'expected' THEN '{"data_version":"v1","status":"not_applicable"}'::jsonb
                    ELSE '{"data_version":"v1","status":"failed","failure_data_version":"v1","error_type":{"error_type_version":"v1","error_type":"no_link_config_found"}}'::jsonb
                END)
            FROM batch
            WHERE e.id = batch.id
            RETURNING e.id
        )
        SELECT count(*), max(id) INTO rows_updated, batch_last_id
        FROM updated;

        COMMIT;

        RAISE NOTICE 'Updated % rows', rows_updated;
        EXIT WHEN rows_updated = 0;

        -- Advance through the primary key instead of rescanning completed rows.
        last_id := batch_last_id;
        PERFORM pg_sleep(1);
    END LOOP;
END;
$$;
