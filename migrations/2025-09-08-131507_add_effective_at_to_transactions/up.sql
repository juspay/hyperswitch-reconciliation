-- Add effective_at column to transactions table
-- This will store the minimum effective_at from all entries in the transaction
ALTER TABLE transactions ADD COLUMN effective_at TIMESTAMP NOT NULL;

-- Add index for filtering transactions by effective_at
CREATE INDEX idx_transactions_effective_at ON transactions(effective_at);

-- Add composite index for profile_id and effective_at for efficient filtering
CREATE INDEX idx_transactions_profile_id_effective_at ON transactions(profile_id, effective_at);
