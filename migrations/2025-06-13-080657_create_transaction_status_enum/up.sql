-- Create the TransactionStatus enum type for transaction status tracking
CREATE TYPE "TransactionStatus" AS ENUM ('expected', 'posted', 'mismatched', 'archived');