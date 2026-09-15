-- Add merchant_id column to search_duplicates table
ALTER TABLE search_duplicates ADD COLUMN merchant_id VARCHAR(64) NOT NULL;
