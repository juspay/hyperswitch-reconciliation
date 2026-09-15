-- Drop indexes first
DROP INDEX IF EXISTS idx_expectation_match;
DROP INDEX IF EXISTS idx_expectation_source;

-- Drop expectations table
DROP TABLE IF EXISTS search_expectations;
