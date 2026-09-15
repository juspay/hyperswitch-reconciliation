-- Drop transaction table and its indexes
DROP INDEX IF EXISTS idx_transaction_profile_status;
DROP INDEX IF EXISTS idx_transaction_id_version;
DROP TABLE IF EXISTS transactions;
