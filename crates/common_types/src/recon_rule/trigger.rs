use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;

pub mod archived;

/// Versioned trigger enum to support future trigger types
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "trigger_version", rename_all = "snake_case")]
pub enum Trigger {
    V1(archived::TriggerV1),
    V2(TriggerV2),
}

/// V2: Composite trigger condition using flat boolean logic over multiple conditions
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TriggerV2 {
    pub logic: TriggerLogic,
    pub conditions: Vec<TriggerCondition>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TriggerLogic {
    All,
    Any,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TriggerCondition {
    pub field: EntryField,
    pub operator: TriggerOperator,
    pub value: String,
}

/// Versioned trigger operator enum to support future operator types
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "operator_version", rename_all = "snake_case")]
pub enum TriggerOperator {
    #[serde(rename = "v1")]
    V1(TriggerOperatorV1),
}

/// Versioned trigger operator with value field
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct TriggerOperatorV1 {
    pub value: BasicComparisonOperator,
}

/// Basic comparison operators for field matching
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BasicComparisonOperator {
    Equals,
    NotEquals,
}

// Use the macro from common_utils to implement JSON serialization/deserialization
common_utils::impl_to_sql_from_sql_json!(Trigger);

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_trigger_v2_all() {
        let trigger: Trigger = serde_json::from_str(
            r#"{
                "trigger_version": "v2",
                "logic": "all",
                "conditions": [
                    {
                        "field": "metadata.payment_type",
                        "operator": { "operator_version": "v1", "value": "equals" },
                        "value": "card"
                    },
                    {
                        "field": "currency",
                        "operator": { "operator_version": "v1", "value": "equals" },
                        "value": "USD"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert!(matches!(
            trigger,
            Trigger::V2(TriggerV2 {
                logic: TriggerLogic::All,
                conditions,
            }) if conditions.len() == 2
        ));
    }

    #[test]
    fn test_deserialize_trigger_v2_any() {
        let trigger: Trigger = serde_json::from_str(
            r#"{
                "trigger_version": "v2",
                "logic": "any",
                "conditions": [
                    {
                        "field": "metadata.region",
                        "operator": { "operator_version": "v1", "value": "equals" },
                        "value": "EU"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert!(matches!(
            trigger,
            Trigger::V2(TriggerV2 {
                logic: TriggerLogic::Any,
                conditions,
            }) if conditions.len() == 1
        ));
    }
}
