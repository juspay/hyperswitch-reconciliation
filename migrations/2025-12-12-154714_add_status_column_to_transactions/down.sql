DROP INDEX IF EXISTS idx_transaction_status;

ALTER TABLE transactions DROP COLUMN IF EXISTS status;