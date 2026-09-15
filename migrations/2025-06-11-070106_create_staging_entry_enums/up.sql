-- Create StagingEntryStatus enum for staging entry processing states
CREATE TYPE "StagingEntryStatus" AS ENUM (
    'pending',
    'needs_manual_review',
    'processed'
);

CREATE TYPE "StagingEntryProcessingMode" AS ENUM (
    'transaction',
    'confirmation'
);
