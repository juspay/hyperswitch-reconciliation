ALTER TABLE transactions ALTER COLUMN discarded_status TYPE VARCHAR(64);

DROP INDEX IF EXISTS idx_transaction_profile_status;

ALTER TABLE transactions DROP COLUMN IF EXISTS transaction_status;

DROP TYPE IF EXISTS "TransactionStatus";
