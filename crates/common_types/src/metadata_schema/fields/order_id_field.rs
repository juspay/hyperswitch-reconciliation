use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    metadata_schema::{
        datatypes::string::{
            StringConfig, StringConfigV1, StringTransformationRule, StringValidationRule,
            StringValidationRuleV1,
        },
        FieldTypeConfig, FieldTypeConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    },
    recon_rule::entry_field::EntryField,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "order_id_field_config_version", rename_all = "snake_case")]
pub enum OrderIdFieldConfig {
    V1(OrderIdFieldConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OrderIdFieldConfigV1 {
    pub identifier: String,
    #[serde(default)]
    pub transformation_rules: Vec<StringTransformationRule>,
}

impl From<&OrderIdFieldConfig> for ReconMetadataFieldConfig {
    fn from(config: &OrderIdFieldConfig) -> Self {
        match config {
            OrderIdFieldConfig::V1(config_v1) => config_v1.into(),
        }
    }
}

impl From<&OrderIdFieldConfigV1> for ReconMetadataFieldConfig {
    fn from(config_v1: &OrderIdFieldConfigV1) -> Self {
        Self::V1(config_v1.into())
    }
}

impl From<&OrderIdFieldConfigV1> for ReconMetadataFieldConfigV1 {
    fn from(config_v1: &OrderIdFieldConfigV1) -> Self {
        Self {
            field_name: EntryField::OrderId,
            identifier: config_v1.identifier.clone(),
            field_config: FieldTypeConfig::V1(FieldTypeConfigV1::String(config_v1.into())),
            required: true,
            description: "Main order_id field".to_string(),
            is_exposed: None,
        }
    }
}

impl From<&OrderIdFieldConfig> for StringConfig {
    fn from(value: &OrderIdFieldConfig) -> Self {
        match value {
            OrderIdFieldConfig::V1(config_v1) => config_v1.into(),
        }
    }
}

impl From<&OrderIdFieldConfigV1> for StringConfig {
    fn from(config_v1: &OrderIdFieldConfigV1) -> Self {
        Self::V1(StringConfigV1 {
            validation_rules: vec![StringValidationRule::V1(
                StringValidationRuleV1::MaxLength { value: 64 },
            )],
            transformation_rules: config_v1.transformation_rules.clone(),
        })
    }
}
