UPDATE api_key
SET expires_at = created_at + INTERVAL '1 year'
WHERE expires_at IS NULL;

ALTER TABLE api_key
    ALTER COLUMN expires_at SET NOT NULL;
