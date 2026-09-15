-- Add rule_id column to transactions table
ALTER TABLE transactions ADD COLUMN rule_id VARCHAR(64) NOT NULL;

-- Create index for query performance
CREATE INDEX IF NOT EXISTS idx_transactions_rule_id ON transactions(rule_id);
