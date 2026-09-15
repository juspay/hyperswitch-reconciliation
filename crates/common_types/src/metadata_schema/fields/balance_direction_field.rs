use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    metadata_schema::{
        datatypes::balance_direction::{
            BalanceDirectionConfig, BalanceDirectionConfigV1, BalanceDirectionTransformationRule,
        },
        FieldTypeConfig, FieldTypeConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    },
    recon_rule::entry_field::EntryField,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "balance_direction_field_config_version",
    rename_all = "snake_case"
)]
pub enum BalanceDirectionFieldConfig {
    V1(BalanceDirectionFieldConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BalanceDirectionFieldConfigV1 {
    pub identifier: String,
    pub credit_values: Vec<String>,
    pub debit_values: Vec<String>,
    #[serde(default)]
    pub transformation_rules: Vec<BalanceDirectionTransformationRule>,
}

impl From<&BalanceDirectionFieldConfig> for ReconMetadataFieldConfig {
    fn from(config: &BalanceDirectionFieldConfig) -> Self {
        match config {
            BalanceDirectionFieldConfig::V1(config_v1) => config_v1.into(),
        }
    }
}

impl From<&BalanceDirectionFieldConfigV1> for ReconMetadataFieldConfig {
    fn from(config_v1: &BalanceDirectionFieldConfigV1) -> Self {
        Self::V1(config_v1.into())
    }
}

impl From<&BalanceDirectionFieldConfigV1> for ReconMetadataFieldConfigV1 {
    fn from(config_v1: &BalanceDirectionFieldConfigV1) -> Self {
        Self {
            field_name: EntryField::EntryType,
            identifier: config_v1.identifier.clone(),
            field_config: FieldTypeConfig::V1(FieldTypeConfigV1::BalanceDirection(
                config_v1.into(),
            )),
            required: true,
            description: "Main balance_direction field".to_string(),
            is_exposed: None,
        }
    }
}

impl From<&BalanceDirectionFieldConfig> for BalanceDirectionConfig {
    fn from(value: &BalanceDirectionFieldConfig) -> Self {
        match value {
            BalanceDirectionFieldConfig::V1(config_v1) => config_v1.into(),
        }
    }
}

impl From<&BalanceDirectionFieldConfigV1> for BalanceDirectionConfig {
    fn from(config_v1: &BalanceDirectionFieldConfigV1) -> Self {
        Self::V1(BalanceDirectionConfigV1 {
            credit_values: config_v1.credit_values.clone(),
            debit_values: config_v1.debit_values.clone(),
            transformation_rules: config_v1.transformation_rules.clone(),
        })
    }
}
