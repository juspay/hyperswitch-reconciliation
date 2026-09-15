-- Add nullable, backfill, then enforce NOT NULL — surfaces backfill misses.
ALTER TABLE search_expectation
    ADD COLUMN primary_transaction_id VARCHAR(64);

UPDATE search_expectation se
  SET primary_transaction_id = t.id
  FROM transaction t
  WHERE se.logical_transaction_id = t.transaction_id
    AND se.transaction_version = t.version;

ALTER TABLE search_expectation
    ALTER COLUMN primary_transaction_id SET NOT NULL;

DROP INDEX IF EXISTS idx_search_expectation_transaction;

ALTER TABLE search_expectation DROP COLUMN expectation_id;
ALTER TABLE search_expectation DROP COLUMN logical_transaction_id;
ALTER TABLE search_expectation DROP COLUMN transaction_version;

CREATE INDEX idx_search_expectation_primary_transaction_target_account
    ON search_expectation (primary_transaction_id, target_account_id);
