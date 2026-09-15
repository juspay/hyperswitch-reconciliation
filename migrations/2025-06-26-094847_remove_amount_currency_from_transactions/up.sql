-- Remove amount and currency columns from transactions table (if they exist)
ALTER TABLE transactions DROP COLUMN IF EXISTS amount;
ALTER TABLE transactions DROP COLUMN IF EXISTS currency;
