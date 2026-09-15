use common_types::recon_rule::{
    entry_field::ExtractFromEntryField,
    trigger::{
        archived::TriggerV1, BasicComparisonOperator, Trigger, TriggerCondition, TriggerLogic,
        TriggerOperator, TriggerOperatorV1, TriggerV2,
    },
};

pub trait TriggerExt {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool;
}

impl TriggerExt for Trigger {
    /// Validate the trigger against provided data
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        match self {
            Self::V1(field_based_trigger) => field_based_trigger.validate(data),
            Self::V2(composite_trigger) => composite_trigger.validate(data),
        }
    }
}

impl TriggerExt for TriggerV1 {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        let Some(field_value) = data.extract_field(&self.field) else {
            return false;
        };

        self.operator.check(&field_value, &self.value)
    }
}

impl TriggerExt for TriggerV2 {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        match self.logic {
            TriggerLogic::All => self
                .conditions
                .iter()
                .all(|condition| condition.validate(data)),
            TriggerLogic::Any => self
                .conditions
                .iter()
                .any(|condition| condition.validate(data)),
        }
    }
}

impl TriggerExt for TriggerCondition {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        let Some(field_value) = data.extract_field(&self.field) else {
            return false;
        };

        self.operator.check(&field_value, &self.value)
    }
}

trait TriggerOperatorExt {
    fn check(&self, field_value: &str, expected_value: &str) -> bool;
}

impl TriggerOperatorExt for TriggerOperator {
    fn check(&self, field_value: &str, expected_value: &str) -> bool {
        match self {
            Self::V1(operator_v1) => operator_v1.check(field_value, expected_value),
        }
    }
}

impl TriggerOperatorExt for TriggerOperatorV1 {
    fn check(&self, field_value: &str, expected_value: &str) -> bool {
        match self.value {
            BasicComparisonOperator::Equals => field_value == expected_value,
            BasicComparisonOperator::NotEquals => field_value != expected_value,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use common_enums::{BalanceDirection, Currency};
    use common_types::{
        recon_rule::{
            entry_field::{EntryField, ExtractAccountId},
            trigger::{TriggerCondition, TriggerLogic, TriggerV2},
        },
        units::MinorUnit,
    };
    use common_utils::crypto::{Encryptable, EncryptableValue};
    use masking::Secret;
    use serde_json::json;
    use time::PrimitiveDateTime;

    struct TestEntryData {
        fields: HashMap<EntryField, String>,
        metadata: EncryptableValue,
    }

    impl ExtractAccountId for TestEntryData {
        fn extract_account_id(&self) -> String {
            "acct_test".to_string()
        }
    }

    impl ExtractFromEntryField for TestEntryData {
        fn extract_field(&self, field_name: &EntryField) -> Option<String> {
            self.fields.get(field_name).cloned()
        }

        fn currency(&self) -> Currency {
            Currency::USD
        }

        fn entry_type(&self) -> BalanceDirection {
            BalanceDirection::Credit
        }

        fn amount(&self) -> MinorUnit {
            MinorUnit::new(100)
        }

        fn metadata(&self) -> &EncryptableValue {
            &self.metadata
        }

        fn effective_at(&self) -> PrimitiveDateTime {
            common_utils::date_time::now()
        }

        fn order_id(&self) -> String {
            "order_test".to_string()
        }
    }

    fn test_entry(fields: Vec<(EntryField, &str)>) -> TestEntryData {
        TestEntryData {
            fields: fields
                .into_iter()
                .map(|(field, value)| (field, value.to_string()))
                .collect(),
            metadata: Encryptable::new(Secret::new(json!({})), Secret::new(Vec::new())),
        }
    }

    fn equals_operator() -> TriggerOperator {
        TriggerOperator::V1(TriggerOperatorV1 {
            value: BasicComparisonOperator::Equals,
        })
    }

    #[test]
    fn trigger_v2_all_returns_true_when_all_conditions_match() {
        let trigger = Trigger::V2(TriggerV2 {
            logic: TriggerLogic::All,
            conditions: vec![
                TriggerCondition {
                    field: EntryField::Currency,
                    operator: equals_operator(),
                    value: "USD".to_string(),
                },
                TriggerCondition {
                    field: EntryField::Metadata("payment_type".to_string()),
                    operator: equals_operator(),
                    value: "card".to_string(),
                },
            ],
        });

        let entry = test_entry(vec![
            (EntryField::Currency, "USD"),
            (EntryField::Metadata("payment_type".to_string()), "card"),
        ]);

        assert!(trigger.validate(&entry));
    }

    #[test]
    fn trigger_v2_any_returns_true_when_any_condition_matches() {
        let trigger = Trigger::V2(TriggerV2 {
            logic: TriggerLogic::Any,
            conditions: vec![
                TriggerCondition {
                    field: EntryField::Currency,
                    operator: equals_operator(),
                    value: "EUR".to_string(),
                },
                TriggerCondition {
                    field: EntryField::Metadata("payment_type".to_string()),
                    operator: equals_operator(),
                    value: "card".to_string(),
                },
            ],
        });

        let entry = test_entry(vec![(
            EntryField::Metadata("payment_type".to_string()),
            "card",
        )]);

        assert!(trigger.validate(&entry));
    }

    #[test]
    fn trigger_v2_returns_false_when_required_field_is_missing() {
        let trigger = Trigger::V2(TriggerV2 {
            logic: TriggerLogic::All,
            conditions: vec![TriggerCondition {
                field: EntryField::Metadata("channel".to_string()),
                operator: equals_operator(),
                value: "upi".to_string(),
            }],
        });

        let entry = test_entry(vec![]);

        assert!(!trigger.validate(&entry));
    }
}
