-- Flatten the manual review staging entry status into granular, indexable values stored as text.
-- The reason previously lived only inside the `data` JSONB blob.

ALTER TABLE staging_entry ALTER COLUMN status TYPE VARCHAR(64);

ALTER TABLE staging_entry ALTER COLUMN discarded_status TYPE VARCHAR(64);

UPDATE staging_entry
SET
    status = CASE data -> 'error_type' ->> 'error_type'
        WHEN 'no_rules_found' THEN 'needs_manual_review_no_rules_found'
        WHEN 'staging_entry_currency_mismatch' THEN 'needs_manual_review_currency_mismatch'
        WHEN 'missing_search_identifier_value' THEN 'needs_manual_review_missing_search_identifier_value'
        WHEN 'duplicate_entry' THEN 'needs_manual_review_duplicate_entry'
        WHEN 'no_expectation_entry_found' THEN 'needs_manual_review_no_expectation_entry_found'
        WHEN 'multiple_expected_entries_found' THEN 'needs_manual_review_multiple_expected_entries_found'
        WHEN 'missing_match_field' THEN 'needs_manual_review_missing_match_field'
        WHEN 'missing_unique_field' THEN 'needs_manual_review_missing_unique_field'
        WHEN 'missing_grouping_field' THEN 'needs_manual_review_missing_grouping_field'
        ELSE 'needs_manual_review_internal_error'
    END
WHERE
    status = 'needs_manual_review';

UPDATE staging_entry
SET
    discarded_status = CASE discarded_data -> 'error_type' ->> 'error_type'
        WHEN 'no_rules_found' THEN 'needs_manual_review_no_rules_found'
        WHEN 'staging_entry_currency_mismatch' THEN 'needs_manual_review_currency_mismatch'
        WHEN 'missing_search_identifier_value' THEN 'needs_manual_review_missing_search_identifier_value'
        WHEN 'duplicate_entry' THEN 'needs_manual_review_duplicate_entry'
        WHEN 'no_expectation_entry_found' THEN 'needs_manual_review_no_expectation_entry_found'
        WHEN 'multiple_expected_entries_found' THEN 'needs_manual_review_multiple_expected_entries_found'
        WHEN 'missing_match_field' THEN 'needs_manual_review_missing_match_field'
        WHEN 'missing_unique_field' THEN 'needs_manual_review_missing_unique_field'
        WHEN 'missing_grouping_field' THEN 'needs_manual_review_missing_grouping_field'
        ELSE 'needs_manual_review_internal_error'
    END
WHERE
    discarded_status = 'needs_manual_review';

DROP TYPE IF EXISTS "StagingEntryStatus";
