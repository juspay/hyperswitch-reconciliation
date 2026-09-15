-- Add back amount and currency columns to transactions table
ALTER TABLE transactions ADD COLUMN amount BIGINT NOT NULL;
ALTER TABLE transactions ADD COLUMN currency "Currency" NOT NULL;
