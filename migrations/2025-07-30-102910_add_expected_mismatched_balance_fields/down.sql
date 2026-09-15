-- Remove the added balance fields
ALTER TABLE account 
DROP COLUMN expected_debits,
DROP COLUMN expected_credits,
DROP COLUMN mismatched_debits,
DROP COLUMN mismatched_credits;
