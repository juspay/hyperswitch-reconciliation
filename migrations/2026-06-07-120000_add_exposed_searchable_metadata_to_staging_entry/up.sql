ALTER TABLE staging_entry
    ADD COLUMN exposed_metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN searchable_metadata JSONB NOT NULL DEFAULT '{}'::jsonb;
