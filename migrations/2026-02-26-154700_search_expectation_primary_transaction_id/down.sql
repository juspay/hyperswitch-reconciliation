-- Add nullable, backfill, then enforce NOT NULL — surfaces backfill misses.
ALTER TABLE search_expectation
    ADD COLUMN expectation_id VARCHAR(64);

ALTER TABLE search_expectation
    ADD COLUMN logical_transaction_id VARCHAR(64);

ALTER TABLE search_expectation
    ADD COLUMN transaction_version INT4;

-- account_id disambiguates which Expected entry maps to which expectation
-- (a transaction can have multiple Expected entries on different accounts).
UPDATE search_expectation se
  SET
      expectation_id         = e.id,
      logical_transaction_id = t.transaction_id,
      transaction_version    = t.version
  FROM transaction t
  JOIN entry e ON e.logical_transaction_id = t.transaction_id
              AND e.transaction_version    = t.version
              AND (e.status = 'expected' OR e.discarded_status = 'expected')
  WHERE se.primary_transaction_id = t.id
    AND e.account_id              = se.target_account_id;

ALTER TABLE search_expectation ALTER COLUMN expectation_id SET NOT NULL;
ALTER TABLE search_expectation ALTER COLUMN logical_transaction_id SET NOT NULL;
ALTER TABLE search_expectation ALTER COLUMN transaction_version SET NOT NULL;

DROP INDEX IF EXISTS idx_search_expectation_primary_transaction_target_account;

ALTER TABLE search_expectation DROP COLUMN primary_transaction_id;

CREATE INDEX idx_search_expectation_transaction
    ON search_expectation (logical_transaction_id, transaction_version);
