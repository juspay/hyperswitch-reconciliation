UPDATE recon_rule
SET strategy = jsonb_set(
    strategy,
    '{source_account, grouping_field}',
    COALESCE(
        strategy #> '{source_account, grouping_field, field}',
        (strategy #> '{source_account, grouping_field, fields}') -> 0
    ),
    true
)
WHERE strategy ? 'source_account'
  AND (strategy #> '{source_account, grouping_field}') IS NOT NULL
  AND jsonb_typeof(strategy #> '{source_account, grouping_field}') = 'object'
  AND (strategy #>> '{source_account, grouping_field, grouping_field_version}') = 'v1';
