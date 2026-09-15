-- Collapse the granular manual review statuses back into the coarse enum.

UPDATE staging_entry
SET
    status = 'needs_manual_review'
WHERE
    status LIKE 'needs_manual_review%';

UPDATE staging_entry
SET
    discarded_status = 'needs_manual_review'
WHERE
    discarded_status LIKE 'needs_manual_review%';

CREATE TYPE "StagingEntryStatus" AS ENUM (
    'pending',
    'needs_manual_review',
    'processed',
    'void',
    'archived'
);

ALTER TABLE staging_entry
ALTER COLUMN status TYPE "StagingEntryStatus" USING status::"StagingEntryStatus";

ALTER TABLE staging_entry
ALTER COLUMN discarded_status TYPE "StagingEntryStatus" USING discarded_status::"StagingEntryStatus";
