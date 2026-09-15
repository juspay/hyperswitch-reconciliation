ALTER TABLE entry
    ADD COLUMN exposed_metadata JSONB NOT NULL DEFAULT '{}'::jsonb;
