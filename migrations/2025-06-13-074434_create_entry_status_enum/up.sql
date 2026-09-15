-- Create EntryStatus enum for entry processing states
CREATE TYPE "EntryStatus" AS ENUM (
    'pending',
    'expected',
    'posted',
    'mismatched',
    'archived'
);
