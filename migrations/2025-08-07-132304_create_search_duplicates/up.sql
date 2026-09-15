CREATE TABLE IF NOT EXISTS search_duplicates (
  search_duplicate_id VARCHAR(64) PRIMARY KEY,
  staging_entry_id VARCHAR(64) NOT NULL,
  account_id VARCHAR(64) NOT NULL,
  transformation_id VARCHAR(64) NOT NULL,
  field_path TEXT NOT NULL,
  encrypted_field_value BYTEA NOT NULL,
  hashed_field_value VARCHAR(64) NOT NULL,
  created_at TIMESTAMP NOT NULL,
  last_modified_at TIMESTAMP NOT NULL
);

-- Index for efficient lookups on staging_entry_id
CREATE INDEX IF NOT EXISTS idx_search_duplicates_staging_entry ON search_duplicates (staging_entry_id);

-- Unique constraint for transformation_id, field_path, hashed_field_value combination
CREATE UNIQUE INDEX IF NOT EXISTS uniq_idx_search_duplicates ON search_duplicates (
  transformation_id, field_path, hashed_field_value
);
