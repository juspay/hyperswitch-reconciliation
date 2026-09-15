-- Revert merchant_name column type from BYTEA back to TEXT
ALTER TABLE merchant_account
ALTER COLUMN merchant_name TYPE TEXT
USING convert_from(merchant_name, 'UTF8');