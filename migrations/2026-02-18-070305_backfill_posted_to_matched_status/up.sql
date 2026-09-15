UPDATE transaction
SET
    status = 'matched_auto',
    data = jsonb_build_object(
        'data_version',
        'v3',
        'status',
        'matched',
        'matched_data_version',
        'v1',
        'matched_data_type',
        'auto'
    )
WHERE
    status = 'posted_auto';

UPDATE transaction
SET
    status = 'matched_manual',
    data = jsonb_build_object(
        'data_version',
        'v3',
        'status',
        'matched',
        'matched_data_version',
        'v1',
        'matched_data_type',
        'manual',
        'manually_matched_data_version',
        'v1',
        'reason',
        data ->> 'reason'
    )
WHERE
    status = 'posted_manual';

UPDATE transaction
SET
    status = 'matched_force',
    data = jsonb_build_object(
        'data_version',
        'v3',
        'status',
        'matched',
        'matched_data_version',
        'v1',
        'matched_data_type',
        'force',
        'force_matched_data_version',
        'v1',
        'reason',
        data ->> 'reason'
    )
WHERE
    status = 'posted_force';

UPDATE entry
SET
    status = 'matched',
    data = jsonb_build_object(
        'data_version',
        'v2',
        'status',
        'matched'
    )
WHERE
    status = 'posted';

ALTER TABLE account RENAME COLUMN posted_debits TO matched_debits;

ALTER TABLE account RENAME COLUMN posted_credits TO matched_credits;

ALTER TABLE account
ADD COLUMN posted_debits BIGINT NOT NULL DEFAULT 0,
ADD COLUMN posted_credits BIGINT NOT NULL DEFAULT 0;
