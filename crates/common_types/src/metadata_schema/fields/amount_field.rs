use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    metadata_schema::{
        datatypes::{
            majorunit::MajorUnitConfig,
            minorunit::{
                MinorUnitConfig, MinorUnitConfigV1, MinorUnitValidationRule,
                MinorUnitValidationRuleV1,
            },
        },
        fields::amount_field::archived::AmountFieldConfigV1,
        FieldTypeConfig, FieldTypeConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    },
    recon_rule::entry_field::EntryField,
};

pub mod archived;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "amount_field_config_version", rename_all = "snake_case")]
pub enum AmountFieldConfig {
    V1(AmountFieldConfigV1),
    V2(AmountFieldConfigV2),
}

impl From<&AmountFieldConfig> for ReconMetadataFieldConfig {
    fn from(config: &AmountFieldConfig) -> Self {
        match config {
            AmountFieldConfig::V1(config_v1) => config_v1.into(),
            AmountFieldConfig::V2(config_v2) => config_v2.into(),
        }
    }
}

impl From<&AmountFieldConfig> for MinorUnitConfig {
    fn from(config: &AmountFieldConfig) -> Self {
        match config {
            AmountFieldConfig::V1(config_v1) => config_v1.into(),
            AmountFieldConfig::V2(config_v2) => config_v2.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AmountFieldConfigV2 {
    pub identifier: String,
    pub unit_config: AmountUnitConfig,
}

impl From<&AmountFieldConfigV2> for ReconMetadataFieldConfig {
    fn from(config_v2: &AmountFieldConfigV2) -> Self {
        Self::V1(config_v2.into())
    }
}

impl From<&AmountFieldConfigV2> for ReconMetadataFieldConfigV1 {
    fn from(config_v2: &AmountFieldConfigV2) -> Self {
        Self {
            field_name: EntryField::Amount,
            identifier: config_v2.identifier.clone(),
            field_config: (&config_v2.unit_config).into(),
            required: true,
            description: "Main amount field".to_string(),
            is_exposed: None,
        }
    }
}

impl From<&AmountFieldConfigV2> for MinorUnitConfig {
    fn from(_: &AmountFieldConfigV2) -> Self {
        Self::V1(MinorUnitConfigV1 {
            validation_rules: vec![MinorUnitValidationRule::V1(
                MinorUnitValidationRuleV1::PositiveOnly,
            )],
            transformation_rules: vec![],
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "unit_type", rename_all = "snake_case")]
pub enum AmountUnitConfig {
    MinorUnit(MinorUnitConfig),
    MajorUnit(MajorUnitConfig),
}

impl From<&AmountUnitConfig> for FieldTypeConfig {
    fn from(config: &AmountUnitConfig) -> Self {
        match config {
            AmountUnitConfig::MinorUnit(minor_config) => {
                Self::V1(FieldTypeConfigV1::MinorUnit(minor_config.clone()))
            }
            AmountUnitConfig::MajorUnit(major_config) => {
                Self::V1(FieldTypeConfigV1::MajorUnit(major_config.clone()))
            }
        }
    }
}
