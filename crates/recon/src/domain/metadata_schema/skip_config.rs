use crate::domain::metadata_schema::ParsedRow;
use crate::domain::{
    metadata_schema::data_validator::ValueTransformer, transformation::types::SourceRecord,
};
use common_types::transformation::{
    RowSkipConfig, SkipCondition, SkipConfig, SkipConfigV1, SkipOperator, ValueSkipConfig,
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum SkipDecision {
    Keep,
    Skip(SkipReason),
}

#[derive(Debug, Clone, Serialize)]
pub struct SkipReason {
    pub matched_conditions: Vec<MatchedCondition>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "matched_condition_type", rename_all = "snake_case")]
pub enum MatchedCondition {
    RowSkip {
        config: RowSkipConfig,
    },
    ValueSkip {
        config: ValueSkipConfig,
        field_value: Option<String>,
    },
}

pub fn evaluate_skip(configs: &[SkipConfig], row: &ParsedRow) -> SkipDecision {
    configs
        .iter()
        .find_map(|config| config.match_row(row))
        .map_or(SkipDecision::Keep, |matched_conditions| {
            SkipDecision::Skip(SkipReason { matched_conditions })
        })
}

trait ConfigSkipEvaluator {
    fn match_row(&self, row: &ParsedRow) -> Option<Vec<MatchedCondition>>;
}

impl ConfigSkipEvaluator for SkipConfig {
    fn match_row(&self, row: &ParsedRow) -> Option<Vec<MatchedCondition>> {
        match self {
            Self::V1(skip_config_v1) => skip_config_v1.match_row(row),
        }
    }
}

impl ConfigSkipEvaluator for SkipConfigV1 {
    fn match_row(&self, row: &ParsedRow) -> Option<Vec<MatchedCondition>> {
        self.conditions
            .iter()
            .map(|condition| condition.match_row(row))
            .collect()
    }
}

trait ConditionSkipEvaluator {
    fn match_row(&self, row: &ParsedRow) -> Option<MatchedCondition>;
}

impl ConditionSkipEvaluator for SkipCondition {
    fn match_row(&self, row: &ParsedRow) -> Option<MatchedCondition> {
        match self {
            Self::RowSkip(row_skip_config) => row_skip_config.match_row(row),
            Self::ValueSkip(value_skip_config) => value_skip_config.match_row(row),
        }
    }
}

impl ConditionSkipEvaluator for RowSkipConfig {
    fn match_row(&self, row: &ParsedRow) -> Option<MatchedCondition> {
        match self {
            Self::LineNo { line_number } => {
                let file_line_number = match row.source {
                    SourceRecord::FileRow { line_number } => line_number,
                    SourceRecord::ApiRecord { .. } => return None,
                };

                (file_line_number == *line_number).then(|| MatchedCondition::RowSkip {
                    config: self.clone(),
                })
            }
        }
    }
}

impl ConditionSkipEvaluator for ValueSkipConfig {
    fn match_row(&self, row: &ParsedRow) -> Option<MatchedCondition> {
        let row_value = row.fields.get(&self.identifier)?;

        let mut transformed_value = row_value.value.clone();
        self.transformation_rules
            .iter()
            .try_for_each(|rule| rule.transform_value(&mut transformed_value))
            .ok()?;

        self.operator
            .check(&transformed_value, &self.value)
            .then(|| MatchedCondition::ValueSkip {
                config: self.clone(),
                field_value: row_value.value.clone(),
            })
    }
}

trait SkipOperatorExt {
    fn check(&self, field_value: &Option<String>, expected_value: &Option<String>) -> bool;
}

fn matches_when_both_present(
    field_value: &Option<String>,
    expected_value: &Option<String>,
    predicate: impl Fn(&str, &str) -> bool,
) -> bool {
    field_value
        .as_ref()
        .zip(expected_value.as_ref())
        .is_some_and(|(field, expected)| predicate(field, expected))
}

impl SkipOperatorExt for SkipOperator {
    fn check(&self, field_value: &Option<String>, expected_value: &Option<String>) -> bool {
        match self {
            Self::Equals => field_value == expected_value,
            Self::NotEquals => field_value != expected_value,
            Self::Contains => {
                matches_when_both_present(field_value, expected_value, |f, e| f.contains(e))
            }
            Self::NotContains => {
                matches_when_both_present(field_value, expected_value, |f, e| !f.contains(e))
            }
            Self::StartsWith => {
                matches_when_both_present(field_value, expected_value, |f, e| f.starts_with(e))
            }
            Self::NotStartsWith => {
                matches_when_both_present(field_value, expected_value, |f, e| !f.starts_with(e))
            }
            Self::EndsWith => {
                matches_when_both_present(field_value, expected_value, |f, e| f.ends_with(e))
            }
            Self::NotEndsWith => {
                matches_when_both_present(field_value, expected_value, |f, e| !f.ends_with(e))
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::domain::transformation::types::{LocatedValue, SourceLocation};
    use rstest::rstest;

    // ---- Case builders (kept tiny so new #[case] entries stay readable) ----

    /// One row in a test case. `fields` is an owned vec so cases can hold it
    /// directly in rstest attribute args without fighting lifetimes.
    type RowSpec = (usize, Vec<(&'static str, Option<&'static str>)>);

    fn skip_v1(conditions: Vec<SkipCondition>) -> SkipConfig {
        SkipConfig::V1(SkipConfigV1 { conditions })
    }

    fn row_skip_line(line_number: usize) -> SkipCondition {
        SkipCondition::RowSkip(RowSkipConfig::LineNo { line_number })
    }

    fn value_skip(identifier: &str, operator: SkipOperator, value: &str) -> SkipCondition {
        value_skip_opt(identifier, operator, Some(value))
    }

    fn value_skip_opt(
        identifier: &str,
        operator: SkipOperator,
        value: Option<&str>,
    ) -> SkipCondition {
        SkipCondition::ValueSkip(ValueSkipConfig {
            identifier: identifier.to_string(),
            operator,
            value: value.map(str::to_string),
            transformation_rules: vec![],
        })
    }

    /// Builds a row as it would come off a file parser: identified by its file line, with each
    /// field carrying that line's location.
    fn parsed_row(
        line_number: usize,
        fields: Vec<(&'static str, Option<&'static str>)>,
    ) -> ParsedRow {
        ParsedRow {
            source: SourceRecord::FileRow { line_number },
            fields: fields
                .into_iter()
                .map(|(field_name, value)| {
                    (
                        field_name.to_string(),
                        LocatedValue {
                            value: value.map(str::to_string),
                            location: SourceLocation::FileRow { line_number },
                        },
                    )
                })
                .collect(),
        }
    }

    /// Shared three-row fixture. Cases that need different data pass their own
    /// Vec<RowSpec> instead of calling this.
    fn fixture_rows() -> Vec<RowSpec> {
        vec![
            (
                2,
                vec![("status", Some("ACTIVE")), ("reference", Some("A"))],
            ),
            (
                3,
                vec![("status", Some("IGNORED")), ("reference", Some("B"))],
            ),
            (4, vec![("status", Some("PENDING")), ("reference", None)]),
        ]
    }

    /// Expected outcome for a case: the file line numbers that survived
    /// the skip pass. Order-independent — the assertion sorts before comparing.
    #[derive(Debug)]
    enum Expect {
        Kept(Vec<usize>),
    }

    // Skip semantics under test:
    //   skip(row) := any_config(all_conditions_match(row))
    // i.e. multiple SkipConfigs are OR-ed; conditions inside one SkipConfig are AND-ed.

    #[rstest]
    // ---- No-op / single-condition baselines ----
    #[case::no_skip_configs_keeps_all_rows(
        vec![],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::empty_conditions_skips_all_rows_vacuously(
        vec![skip_v1(vec![])],
        fixture_rows(),
        Expect::Kept(vec![])
    )]
    #[case::row_skip_removes_matching_line(
        vec![skip_v1(vec![row_skip_line(3)])],
        fixture_rows(),
        Expect::Kept(vec![2, 4])
    )]
    #[case::row_skip_no_matching_line_keeps_all(
        vec![skip_v1(vec![row_skip_line(999)])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::value_skip_equals_removes_matching_rows(
        vec![skip_v1(vec![value_skip("status", SkipOperator::Equals, "IGNORED")])],
        fixture_rows(),
        Expect::Kept(vec![2, 4])
    )]
    #[case::value_skip_not_equals_removes_non_matching_rows(
        vec![skip_v1(vec![value_skip("status", SkipOperator::NotEquals, "ACTIVE")])],
        fixture_rows(),
        Expect::Kept(vec![2])
    )]
    #[case::value_skip_none_value_matches_only_none_fields(
        // ValueSkip with value=None matches a row only when the field itself is
        // None. Row 4's reference is None → skipped. Rows 2, 3 have Some(_) → kept.
        vec![skip_v1(vec![value_skip_opt("reference", SkipOperator::Equals, None)])],
        fixture_rows(),
        Expect::Kept(vec![2, 3])
    )]
    #[case::value_skip_empty_string_does_not_match_none_field(
        // value=Some("") is distinct from value=None. Row 4's reference=None
        // does not match Some("") → no row is dropped.
        vec![skip_v1(vec![value_skip("reference", SkipOperator::Equals, "")])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::value_skip_missing_identifier_does_not_skip(
        // Identifier not present in any row's headers → condition evaluates to
        // false → row is kept. Misconfigured identifiers no-op rather than abort.
        vec![skip_v1(vec![value_skip("unknown_column", SkipOperator::Equals, "x")])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    // ---- Conjunction within a single SkipConfig (AND) ----
    #[case::conjunction_all_conditions_match_skips_row(
        // Row 3 is the only row matching BOTH line=3 AND status=IGNORED.
        vec![skip_v1(vec![
            row_skip_line(3),
            value_skip("status", SkipOperator::Equals, "IGNORED"),
        ])],
        fixture_rows(),
        Expect::Kept(vec![2, 4])
    )]
    #[case::conjunction_one_condition_fails_keeps_row(
        // line=2 matches row 2, but row 2's status is ACTIVE not IGNORED.
        // No row satisfies both conditions, so nothing is skipped. This is
        // the case that catches an OR-vs-AND regression.
        vec![skip_v1(vec![
            row_skip_line(2),
            value_skip("status", SkipOperator::Equals, "IGNORED"),
        ])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::conjunction_with_missing_column_does_not_skip(
        // A missing-column ValueSkip evaluates to false, so the AND chain is
        // false for every row. Combined with row_skip_line(999) (also false
        // everywhere), no row matches both conditions → all rows kept.
        vec![skip_v1(vec![
            row_skip_line(999),
            value_skip("missing_column", SkipOperator::Equals, "x"),
        ])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::conjunction_three_conditions_all_match(
        // All three conditions are satisfied only by row 3.
        vec![skip_v1(vec![
            row_skip_line(3),
            value_skip("status", SkipOperator::Equals, "IGNORED"),
            value_skip("reference", SkipOperator::NotEquals, ""),
        ])],
        fixture_rows(),
        Expect::Kept(vec![2, 4])
    )]
    // ---- Disjunction across SkipConfigs (OR) ----
    #[case::any_matching_config_skips_row(
        vec![
            skip_v1(vec![row_skip_line(2)]),
            skip_v1(vec![value_skip("status", SkipOperator::Equals, "IGNORED")]),
        ],
        fixture_rows(),
        Expect::Kept(vec![4])
    )]
    #[case::disjunction_of_conjunctions_full_pattern(
        // (status=IGNORED AND reference=B) OR (status=PENDING AND reference=None)
        // Row 2 (ACTIVE,A):       neither group matches → kept
        // Row 3 (IGNORED,B):      first group matches   → skipped
        // Row 4 (PENDING,None):   second group matches  → skipped
        vec![
            skip_v1(vec![
                value_skip("status", SkipOperator::Equals, "IGNORED"),
                value_skip("reference", SkipOperator::Equals, "B"),
            ]),
            skip_v1(vec![
                value_skip("status", SkipOperator::Equals, "PENDING"),
                value_skip_opt("reference", SkipOperator::Equals, None),
            ]),
        ],
        fixture_rows(),
        Expect::Kept(vec![2])
    )]
    #[case::disjunction_first_group_misses_second_group_skips(
        // First group can't match anything (line=999 ∧ ...); second group
        // matches row 3. Verifies a non-matching group doesn't poison the OR.
        vec![
            skip_v1(vec![
                row_skip_line(999),
                value_skip("status", SkipOperator::Equals, "IGNORED"),
            ]),
            skip_v1(vec![
                value_skip("status", SkipOperator::Equals, "IGNORED"),
                value_skip("reference", SkipOperator::Equals, "B"),
            ]),
        ],
        fixture_rows(),
        Expect::Kept(vec![2, 4])
    )]
    // ---- Contains operator ----
    #[case::value_skip_contains_substring_removes_matching_rows(
        vec![skip_v1(vec![value_skip("status", SkipOperator::Contains, "TIV")])],
        fixture_rows(),
        Expect::Kept(vec![3, 4])
    )]
    #[case::value_skip_contains_none_field_keeps_row(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::Contains, "X")])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::value_skip_contains_empty_string_matches_every_some_field(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::Contains, "")])],
        fixture_rows(),
        Expect::Kept(vec![4])
    )]
    // ---- NotContains operator ----
    #[case::value_skip_not_contains_keeps_only_rows_with_substring(
        vec![skip_v1(vec![value_skip("status", SkipOperator::NotContains, "TIVE")])],
        fixture_rows(),
        Expect::Kept(vec![2])
    )]
    #[case::value_skip_not_contains_none_field_keeps_row(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::NotContains, "X")])],
        fixture_rows(),
        Expect::Kept(vec![4])
    )]
    #[case::contains_or_not_contains_disjunction(
        vec![
            skip_v1(vec![value_skip("status", SkipOperator::Contains, "PEN")]),
            skip_v1(vec![value_skip("status", SkipOperator::NotContains, "TIVE")]),
        ],
        fixture_rows(),
        Expect::Kept(vec![2])
    )]
    // ---- StartsWith operator ----
    #[case::value_skip_starts_with_prefix_removes_matching_rows(
        vec![skip_v1(vec![value_skip("status", SkipOperator::StartsWith, "ACT")])],
        fixture_rows(),
        Expect::Kept(vec![3, 4])
    )]
    #[case::value_skip_starts_with_is_anchored_not_substring(
        vec![skip_v1(vec![value_skip("status", SkipOperator::StartsWith, "TIV")])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::value_skip_starts_with_none_field_keeps_row(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::StartsWith, "X")])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::value_skip_starts_with_empty_string_matches_every_some_field(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::StartsWith, "")])],
        fixture_rows(),
        Expect::Kept(vec![4])
    )]
    // ---- NotStartsWith operator ----
    #[case::value_skip_not_starts_with_keeps_only_rows_with_prefix(
        vec![skip_v1(vec![value_skip("status", SkipOperator::NotStartsWith, "ACT")])],
        fixture_rows(),
        Expect::Kept(vec![2])
    )]
    #[case::value_skip_not_starts_with_none_field_keeps_row(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::NotStartsWith, "X")])],
        fixture_rows(),
        Expect::Kept(vec![4])
    )]
    // ---- EndsWith operator ----
    #[case::value_skip_ends_with_suffix_removes_matching_rows(
        vec![skip_v1(vec![value_skip("status", SkipOperator::EndsWith, "IVE")])],
        fixture_rows(),
        Expect::Kept(vec![3, 4])
    )]
    #[case::value_skip_ends_with_is_anchored_not_substring(
        // "ACTIVE" contains "TIV" but does not end with it.
        vec![skip_v1(vec![value_skip("status", SkipOperator::EndsWith, "TIV")])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::value_skip_ends_with_none_field_keeps_row(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::EndsWith, "X")])],
        fixture_rows(),
        Expect::Kept(vec![2, 3, 4])
    )]
    #[case::value_skip_ends_with_empty_string_matches_every_some_field(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::EndsWith, "")])],
        fixture_rows(),
        Expect::Kept(vec![4])
    )]
    // ---- NotEndsWith operator ----
    #[case::value_skip_not_ends_with_keeps_only_rows_with_suffix(
        vec![skip_v1(vec![value_skip("status", SkipOperator::NotEndsWith, "ED")])],
        fixture_rows(),
        Expect::Kept(vec![3])
    )]
    #[case::value_skip_not_ends_with_none_field_keeps_row(
        vec![skip_v1(vec![value_skip("reference", SkipOperator::NotEndsWith, "X")])],
        fixture_rows(),
        Expect::Kept(vec![4])
    )]
    // ---- Edge ----
    #[case::empty_input_rows_returns_empty(
        vec![skip_v1(vec![row_skip_line(2)])],
        vec![],
        Expect::Kept(vec![])
    )]
    fn test_skip_rows_based_on_config(
        #[case] skip_configs: Vec<SkipConfig>,
        #[case] rows_with_lines: Vec<RowSpec>,
        #[case] expected: Expect,
    ) {
        let remaining: Vec<ParsedRow> = rows_with_lines
            .into_iter()
            .map(|(line_number, fields)| parsed_row(line_number, fields))
            .filter(|row| matches!(evaluate_skip(&skip_configs, row), SkipDecision::Keep))
            .collect();

        match expected {
            Expect::Kept(mut expected_lines) => {
                let mut actual_lines: Vec<usize> = remaining
                    .iter()
                    .map(|row| match row.source {
                        SourceRecord::FileRow { line_number } => line_number,
                        SourceRecord::ApiRecord { .. } => {
                            panic!("surviving row was not sourced from a file line")
                        }
                    })
                    .collect();
                actual_lines.sort_unstable();
                expected_lines.sort_unstable();
                assert_eq!(actual_lines, expected_lines);
            }
        }
    }

    #[test]
    fn value_skip_applies_string_transformations_before_comparison() {
        let skip_configs = serde_json::from_value::<Vec<SkipConfig>>(serde_json::json!([
            {
                "skip_config_version": "v1",
                "conditions": [{
                    "skip_type": "value_skip",
                    "identifier": "MERCHANT NUM",
                    "operator": "not_equals",
                    "value": "pro_example_profile_id",
                    "transformation_rules": [{
                        "string_transformation_rule_version": "v1",
                        "string_transformation_rule_type": "trim"
                    }]
                }]
            }
        ]))
        .expect("valid skip config");

        let matching_row = parsed_row(
            3,
            vec![(
                "MERCHANT NUM",
                Some("pro_example_profile_id                                        "),
            )],
        );
        let non_matching_row = parsed_row(
            4,
            vec![(
                "MERCHANT NUM",
                Some("another_merchant                                        "),
            )],
        );

        let remaining: Vec<ParsedRow> = vec![matching_row, non_matching_row]
            .into_iter()
            .filter(|row| matches!(evaluate_skip(&skip_configs, row), SkipDecision::Keep))
            .collect();

        assert_eq!(remaining.len(), 1);
        assert_eq!(
            remaining[0].source,
            SourceRecord::FileRow { line_number: 3 }
        );
    }
}
