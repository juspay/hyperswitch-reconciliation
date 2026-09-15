-- Change merchant_name column type from TEXT to BYTEA
ALTER TABLE merchant_account
ALTER COLUMN merchant_name TYPE BYTEA
USING convert_to(merchant_name, 'UTF8');