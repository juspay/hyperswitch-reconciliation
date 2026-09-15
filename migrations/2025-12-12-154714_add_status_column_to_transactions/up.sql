ALTER TABLE transactions ADD COLUMN status VARCHAR(64) NOT NULL;

CREATE INDEX idx_transaction_status ON transactions(status);