-- Drop indexes and column added in up.sql
DROP INDEX idx_transactions_profile_id_effective_at;
DROP INDEX idx_transactions_effective_at;
ALTER TABLE transactions DROP COLUMN effective_at;
