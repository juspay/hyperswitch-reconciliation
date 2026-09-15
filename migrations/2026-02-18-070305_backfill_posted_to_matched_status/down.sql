ALTER TABLE account
DROP COLUMN posted_debits,
DROP COLUMN posted_credits;

ALTER TABLE account RENAME COLUMN matched_debits TO posted_debits;

ALTER TABLE account RENAME COLUMN matched_credits TO posted_credits;

UPDATE transaction
SET
    status = 'posted_auto',
    data = jsonb_build_object(
        'data_version',
        'v3',
        'status',
        'posted',
        'posted_version',
        'v1',
        'posted_type',
        'reconciled'
    )
WHERE
    status = 'matched_auto';

UPDATE transaction
SET
    status = 'posted_manual',
    data = jsonb_build_object(
        'data_version',
        'v3',
        'status',
        'posted',
        'posted_version',
        'v1',
        'posted_type',
        'manually_reconciled',
        'manually_reconciled_version',
        'v1',
        'reason',
        data ->> 'reason'
    )
WHERE
    status = 'matched_manual';

UPDATE transaction
SET
    status = 'posted_force',
    data = jsonb_build_object(
        'data_version',
        'v3',
        'status',
        'posted',
        'posted_version',
        'v1',
        'posted_type',
        'force_reconciled',
        'force_reconciled_version',
        'v1',
        'reason',
        data ->> 'reason'
    )
WHERE
    status = 'matched_force';

UPDATE entry
SET
    status = 'posted',
    data = jsonb_build_object(
        'data_version',
        'v2',
        'status',
        'posted'
    )
WHERE
    status = 'matched';
