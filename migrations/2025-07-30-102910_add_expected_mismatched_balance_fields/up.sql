-- Add new balance fields for enhanced balance calculation
ALTER TABLE account 
ADD COLUMN expected_debits BIGINT NOT NULL,
ADD COLUMN expected_credits BIGINT NOT NULL,
ADD COLUMN mismatched_debits BIGINT NOT NULL,
ADD COLUMN mismatched_credits BIGINT NOT NULL;
