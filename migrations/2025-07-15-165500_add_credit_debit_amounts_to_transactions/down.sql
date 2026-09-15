-- Remove credit_amount, debit_amount, and currency columns from transactions table
ALTER TABLE transactions 
DROP COLUMN IF EXISTS credit_amount,
DROP COLUMN IF EXISTS debit_amount,
DROP COLUMN IF EXISTS currency;