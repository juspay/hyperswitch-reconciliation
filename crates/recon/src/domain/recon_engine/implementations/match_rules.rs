use common_types::{
    entry::{FieldMismatchDetail, FieldMismatchDetailV1},
    recon_rule::{
        entry_field::ExtractFromEntryField,
        match_rules::{MatchOperator, MatchRules, MatchRulesV1},
        tolerance_config::ToleranceConfig,
    },
};

use crate::domain::recon_engine::{
    AmountMismatch, AmountMismatchDetail, BalanceDirectionMismatch, CurrencyMismatch,
    MatchRuleValidation, MetadataMismatch, ProcessingError, ProcessingResult, ToleranceDetail,
};

pub trait MatchRulesExt {
    fn extract_expected_entry_metadata_from_source(
        &self,
        source_entry: &impl ExtractFromEntryField,
    ) -> ProcessingResult<serde_json::Map<String, serde_json::Value>>;

    fn extract_expected_entry_metadata_from_multiple_sources(
        &self,
        source_entries: &[impl ExtractFromEntryField],
    ) -> ProcessingResult<serde_json::Map<String, serde_json::Value>>;

    fn validate_metadata(
        &self,
        actual_entry: &dyn ExtractFromEntryField,
        expected_entry: &dyn ExtractFromEntryField,
    ) -> ProcessingResult<MatchRuleValidation<MetadataMismatch>>;

    fn validate_metadata_with_source(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
    ) -> ProcessingResult<MatchRuleValidation<MetadataMismatch>>;

    fn validate_currency(
        &self,
        actual_entry: &dyn ExtractFromEntryField,
        expected_entry: &dyn ExtractFromEntryField,
    ) -> MatchRuleValidation<CurrencyMismatch> {
        if actual_entry.currency() != expected_entry.currency() {
            return MatchRuleValidation::Failed(CurrencyMismatch {
                expected: expected_entry.currency(),
                actual: actual_entry.currency(),
            });
        }
        MatchRuleValidation::Passed
    }

    fn validate_balance_direction(
        &self,
        actual_entry: &dyn ExtractFromEntryField,
        expected_entry: &dyn ExtractFromEntryField,
    ) -> MatchRuleValidation<BalanceDirectionMismatch> {
        if actual_entry.entry_type() != expected_entry.entry_type() {
            return MatchRuleValidation::Failed(BalanceDirectionMismatch {
                expected: expected_entry.entry_type(),
                actual: actual_entry.entry_type(),
            });
        }

        MatchRuleValidation::Passed
    }

    fn validate_balance_direction_with_source_and_target(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
    ) -> MatchRuleValidation<BalanceDirectionMismatch> {
        if source_entry.entry_type() == target_entry.entry_type() {
            return MatchRuleValidation::Failed(BalanceDirectionMismatch {
                expected: source_entry.entry_type().opposite(),
                actual: target_entry.entry_type(),
            });
        }

        MatchRuleValidation::Passed
    }

    fn validate_underamount(
        &self,
        actual_entry: &dyn ExtractFromEntryField,
        expected_entry: &dyn ExtractFromEntryField,
    ) -> MatchRuleValidation<AmountMismatchDetail> {
        if actual_entry.amount() < expected_entry.amount() {
            return MatchRuleValidation::Failed(AmountMismatchDetail {
                expected: expected_entry.amount(),
                actual: actual_entry.amount(),
                currency: expected_entry.currency(),
            });
        }

        MatchRuleValidation::Passed
    }

    fn validate_underamount_with_source_and_target(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
    ) -> MatchRuleValidation<AmountMismatchDetail> {
        if target_entry.amount() < source_entry.amount() {
            return MatchRuleValidation::Failed(AmountMismatchDetail {
                expected: source_entry.amount(),
                actual: target_entry.amount(),
                currency: source_entry.currency(),
            });
        }

        MatchRuleValidation::Passed
    }

    fn validate_underamount_with_source_and_target_and_tolerance(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
        tolerance_config: Option<&ToleranceConfig>,
    ) -> MatchRuleValidation<AmountMismatch> {
        if target_entry.amount() >= source_entry.amount() {
            return MatchRuleValidation::Passed;
        }

        let Some(tolerance_config) = tolerance_config else {
            return MatchRuleValidation::Failed(AmountMismatch::Failed(AmountMismatchDetail {
                expected: source_entry.amount(),
                actual: target_entry.amount(),
                currency: source_entry.currency(),
            }));
        };

        let ToleranceConfig::V1(tolerance_config) = tolerance_config;
        let lower_tolerance = tolerance_config.lower_tolerance;

        let shortfall = source_entry.amount() - target_entry.amount();

        if shortfall <= lower_tolerance {
            MatchRuleValidation::Failed(AmountMismatch::WithinTolerance(ToleranceDetail {
                tolerance_amount: shortfall,
                currency: source_entry.currency(),
                tolerance_balance_direction: target_entry.entry_type(),
                tolerance_account_id: tolerance_config.tolerance_account_id.clone(),
            }))
        } else {
            MatchRuleValidation::Failed(AmountMismatch::Failed(AmountMismatchDetail {
                expected: source_entry.amount(),
                actual: target_entry.amount(),
                currency: source_entry.currency(),
            }))
        }
    }

    fn validate_overamount(
        &self,
        actual_entry: &dyn ExtractFromEntryField,
        expected_entry: &dyn ExtractFromEntryField,
    ) -> MatchRuleValidation<AmountMismatchDetail> {
        if actual_entry.amount() > expected_entry.amount() {
            return MatchRuleValidation::Failed(AmountMismatchDetail {
                expected: expected_entry.amount(),
                actual: actual_entry.amount(),
                currency: expected_entry.currency(),
            });
        }

        MatchRuleValidation::Passed
    }

    fn validate_overamount_with_source_and_target(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
    ) -> MatchRuleValidation<AmountMismatchDetail> {
        if target_entry.amount() > source_entry.amount() {
            return MatchRuleValidation::Failed(AmountMismatchDetail {
                expected: source_entry.amount(),
                actual: target_entry.amount(),
                currency: source_entry.currency(),
            });
        }

        MatchRuleValidation::Passed
    }

    fn validate_overamount_with_source_and_target_and_tolerance(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
        tolerance_config: Option<&ToleranceConfig>,
    ) -> MatchRuleValidation<AmountMismatch> {
        if target_entry.amount() <= source_entry.amount() {
            return MatchRuleValidation::Passed;
        }

        let Some(tolerance_config) = tolerance_config else {
            return MatchRuleValidation::Failed(AmountMismatch::Failed(AmountMismatchDetail {
                expected: source_entry.amount(),
                actual: target_entry.amount(),
                currency: source_entry.currency(),
            }));
        };

        let ToleranceConfig::V1(tolerance_config) = tolerance_config;
        let upper_tolerance = tolerance_config.upper_tolerance;

        let shortfall = target_entry.amount() - source_entry.amount();

        if shortfall <= upper_tolerance {
            MatchRuleValidation::Failed(AmountMismatch::WithinTolerance(ToleranceDetail {
                tolerance_amount: shortfall,
                currency: source_entry.currency(),
                tolerance_balance_direction: source_entry.entry_type(),
                tolerance_account_id: tolerance_config.tolerance_account_id.clone(),
            }))
        } else {
            MatchRuleValidation::Failed(AmountMismatch::Failed(AmountMismatchDetail {
                expected: source_entry.amount(),
                actual: target_entry.amount(),
                currency: source_entry.currency(),
            }))
        }
    }
}

impl MatchRulesExt for MatchRulesV1 {
    fn extract_expected_entry_metadata_from_source(
        &self,
        source_entry: &impl ExtractFromEntryField,
    ) -> ProcessingResult<serde_json::Map<String, serde_json::Value>> {
        self.rules
            .iter()
            .try_fold(serde_json::Map::new(), |mut map, rule| {
                if rule.source_field.is_metadata() {
                    let value = source_entry.extract_field(&rule.source_field).ok_or(
                        ProcessingError::MissingMatchField(rule.source_field.clone()),
                    )?;

                    let metadata_key = rule.target_field.to_field_path_without_metadata_prefix();
                    map.insert(metadata_key, serde_json::Value::String(value));
                }

                Ok(map)
            })
    }

    fn extract_expected_entry_metadata_from_multiple_sources(
        &self,
        source_entries: &[impl ExtractFromEntryField],
    ) -> ProcessingResult<serde_json::Map<String, serde_json::Value>> {
        let first_entry = source_entries
            .first()
            .ok_or(ProcessingError::InternalError(
                "No entries provided".to_string(),
            ))?;

        let metadata = self.extract_expected_entry_metadata_from_source(first_entry)?;

        for entry in source_entries.iter().skip(1) {
            let entry_metadata = self.extract_expected_entry_metadata_from_source(entry)?;

            if entry_metadata != metadata {
                return Err(ProcessingError::InternalError(
                    "Inconsistent metadata across source entries".to_string(),
                )
                .into());
            }
        }

        Ok(metadata)
    }

    fn validate_metadata(
        &self,
        actual_entry: &dyn ExtractFromEntryField,
        expected_entry: &dyn ExtractFromEntryField,
    ) -> ProcessingResult<MatchRuleValidation<MetadataMismatch>> {
        let metadata_mismatches: Vec<FieldMismatchDetail> = self.rules.iter().try_fold(
            Vec::new(),
            |mut mismatches, rule| -> ProcessingResult<Vec<FieldMismatchDetail>> {
                let actual_value = actual_entry.extract_field(&rule.target_field).ok_or(
                    ProcessingError::MissingMatchField(rule.target_field.clone()),
                )?;

                let expected_value = expected_entry.extract_field(&rule.target_field).ok_or(
                    ProcessingError::MissingMatchField(rule.target_field.clone()),
                )?;

                if !rule.operator.check(&actual_value, &expected_value) {
                    mismatches.push(FieldMismatchDetail::V1(FieldMismatchDetailV1 {
                        field_name: rule.target_field.clone(),
                        label: rule.label.clone(),
                        expected_value,
                        actual_value,
                    }));
                }

                Ok(mismatches)
            },
        )?;

        if !metadata_mismatches.is_empty() {
            return Ok(MatchRuleValidation::Failed(MetadataMismatch {
                mismatched_fields: metadata_mismatches,
            }));
        }

        Ok(MatchRuleValidation::Passed)
    }

    fn validate_metadata_with_source(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
    ) -> ProcessingResult<MatchRuleValidation<MetadataMismatch>> {
        let metadata_mismatches: Vec<FieldMismatchDetail> = self.rules.iter().try_fold(
            Vec::new(),
            |mut mismatches, rule| -> ProcessingResult<Vec<FieldMismatchDetail>> {
                let source_value = source_entry.extract_field(&rule.source_field).ok_or(
                    ProcessingError::MissingMatchField(rule.source_field.clone()),
                )?;

                let target_value = target_entry.extract_field(&rule.target_field).ok_or(
                    ProcessingError::MissingMatchField(rule.target_field.clone()),
                )?;

                if !rule.operator.check(&target_value, &source_value) {
                    mismatches.push(FieldMismatchDetail::V1(FieldMismatchDetailV1 {
                        field_name: rule.target_field.clone(),
                        label: rule.label.clone(),
                        expected_value: source_value,
                        actual_value: target_value,
                    }));
                }

                Ok(mismatches)
            },
        )?;

        if !metadata_mismatches.is_empty() {
            return Ok(MatchRuleValidation::Failed(MetadataMismatch {
                mismatched_fields: metadata_mismatches,
            }));
        }

        Ok(MatchRuleValidation::Passed)
    }
}

impl MatchRulesExt for MatchRules {
    fn extract_expected_entry_metadata_from_source(
        &self,
        staging_entry: &impl ExtractFromEntryField,
    ) -> ProcessingResult<serde_json::Map<String, serde_json::Value>> {
        match self {
            Self::V1(match_rules_v1) => {
                match_rules_v1.extract_expected_entry_metadata_from_source(staging_entry)
            }
        }
    }

    fn extract_expected_entry_metadata_from_multiple_sources(
        &self,
        source_entries: &[impl ExtractFromEntryField],
    ) -> ProcessingResult<serde_json::Map<String, serde_json::Value>> {
        match self {
            Self::V1(match_rules_v1) => {
                match_rules_v1.extract_expected_entry_metadata_from_multiple_sources(source_entries)
            }
        }
    }

    fn validate_metadata(
        &self,
        actual_entry: &dyn ExtractFromEntryField,
        expected_entry: &dyn ExtractFromEntryField,
    ) -> ProcessingResult<MatchRuleValidation<MetadataMismatch>> {
        match self {
            Self::V1(match_rules_v1) => {
                match_rules_v1.validate_metadata(actual_entry, expected_entry)
            }
        }
    }

    fn validate_metadata_with_source(
        &self,
        source_entry: &dyn ExtractFromEntryField,
        target_entry: &dyn ExtractFromEntryField,
    ) -> ProcessingResult<MatchRuleValidation<MetadataMismatch>> {
        match self {
            Self::V1(match_rules_v1) => {
                match_rules_v1.validate_metadata_with_source(source_entry, target_entry)
            }
        }
    }
}

trait MatchOperatorExt {
    fn check(&self, actual_value: &str, expected_value: &str) -> bool;
}

impl MatchOperatorExt for MatchOperator {
    fn check(&self, actual_value: &str, expected_value: &str) -> bool {
        match self {
            Self::Equals => actual_value == expected_value,
        }
    }
}
