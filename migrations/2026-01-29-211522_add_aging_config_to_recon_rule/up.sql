-- 1) Add column as nullable
ALTER TABLE recon_rule ADD COLUMN aging_config JSONB;

-- 2) Backfill with default no aging
UPDATE recon_rule
SET
    aging_config = '{"aging_config_version": "v1", "aging_config_type": "no_aging"}'::jsonb;

-- 3) Enforce NOT NULL
ALTER TABLE recon_rule ALTER COLUMN aging_config SET NOT NULL;