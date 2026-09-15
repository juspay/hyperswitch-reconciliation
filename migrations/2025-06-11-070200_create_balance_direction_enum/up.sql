-- Create BalanceDirection enum for CREDIT/DEBIT transactions
CREATE TYPE "BalanceDirection" AS ENUM (
    'credit',
    'debit'
);
