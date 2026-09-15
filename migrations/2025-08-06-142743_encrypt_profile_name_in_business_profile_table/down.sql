-- Revert profile_name column type from BYTEA back to TEXT
ALTER TABLE business_profile
ALTER COLUMN profile_name TYPE TEXT
USING convert_from(profile_name, 'UTF8');