-- Add credit_amount and debit_amount columns to transactions table
ALTER TABLE transactions 
ADD COLUMN credit_amount BIGINT NOT NULL,
ADD COLUMN debit_amount BIGINT NOT NULL,
ADD COLUMN currency "Currency" NOT NULL;
