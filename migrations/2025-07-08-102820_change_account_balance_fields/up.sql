-- Change account balance fields from pending_balance/posted_balance to separate debit/credit fields

-- Add new balance fields
ALTER TABLE account 
ADD COLUMN posted_debits BIGINT NOT NULL,
ADD COLUMN posted_credits BIGINT NOT NULL,
ADD COLUMN pending_debits BIGINT NOT NULL,
ADD COLUMN pending_credits BIGINT NOT NULL;

-- Drop old balance columns
ALTER TABLE account 
DROP COLUMN pending_balance,
DROP COLUMN posted_balance;
