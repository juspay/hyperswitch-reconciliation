-- Add modified_by column to transaction table
ALTER TABLE transaction ADD COLUMN modified_by VARCHAR(64);
