-- Add PartiallyReconciled status to TransactionStatus enum
ALTER TYPE "TransactionStatus" ADD VALUE 'partially_reconciled';
