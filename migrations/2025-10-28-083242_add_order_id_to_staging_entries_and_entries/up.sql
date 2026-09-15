-- Add order_id column to staging_entries table
-- This will store the real-world order/transaction identifier from external systems
ALTER TABLE staging_entries ADD COLUMN order_id VARCHAR(64) NOT NULL;

-- Add index for filtering staging entries by order_id
CREATE INDEX idx_staging_entries_order_id ON staging_entries(order_id);

-- Add order_id column to entries table
-- This will store the real-world order/transaction identifier, inherited from staging entries
ALTER TABLE entries ADD COLUMN order_id VARCHAR(64) NOT NULL;

-- Add index for filtering entries by order_id
CREATE INDEX idx_entries_order_id ON entries(order_id);
