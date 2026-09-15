UPDATE recon_rule
SET strategy = jsonb_set(
    strategy,
    '{source_account, grouping_field}',
    jsonb_build_object(
        'grouping_field_version', 'v1',
        'grouping_field_v1_type', 'single',
        'field', strategy #> '{source_account, grouping_field}'
    ),
    true
)
WHERE strategy ? 'source_account'
  AND (strategy #> '{source_account, grouping_field}') IS NOT NULL
  AND jsonb_typeof(strategy #> '{source_account, grouping_field}') = 'string';
