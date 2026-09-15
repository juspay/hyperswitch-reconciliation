-- Reverse the account balance field changes

-- Add back the original balance fields
ALTER TABLE account 
ADD COLUMN pending_balance BIGINT NOT NULL,
ADD COLUMN posted_balance BIGINT NOT NULL;

-- Drop the new debit/credit columns
ALTER TABLE account 
DROP COLUMN posted_debits,
DROP COLUMN posted_credits,
DROP COLUMN pending_debits,
DROP COLUMN pending_credits;
