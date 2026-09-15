-- Change profile_name column type from TEXT to BYTEA
ALTER TABLE business_profile
ALTER COLUMN profile_name TYPE BYTEA
USING convert_to(profile_name, 'UTF8');
