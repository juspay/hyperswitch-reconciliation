-- Create ExpectationStatus enum for expectation states
CREATE TYPE "ExpectationStatus" AS ENUM (
    'expected',
    'archived'
);
