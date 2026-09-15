CREATE TYPE "TransactionStatus" AS ENUM (
    'expected',
    'posted',
    'mismatched',
    'archived',
    'void',
    'partially_reconciled'
);

ALTER TABLE transactions ALTER COLUMN discarded_status TYPE "TransactionStatus" USING discarded_status::"TransactionStatus";

ALTER TABLE transactions ADD COLUMN transaction_status "TransactionStatus" NOT NULL;

CREATE INDEX idx_transaction_profile_status ON transactions (profile_id, transaction_status);
