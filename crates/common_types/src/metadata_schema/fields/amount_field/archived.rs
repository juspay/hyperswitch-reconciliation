use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    metadata_schema::{
        datatypes::minorunit::{
            MinorUnitConfig, MinorUnitConfigV1, MinorUnitValidationRule, MinorUnitValidationRuleV1,
        },
        FieldTypeConfig, FieldTypeConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    },
    recon_rule::entry_field::EntryField,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AmountFieldConfigV1 {
    pub identifier: String,
}

impl From<&AmountFieldConfigV1> for ReconMetadataFieldConfig {
    fn from(config_v1: &AmountFieldConfigV1) -> Self {
        Self::V1(config_v1.into())
    }
}

impl From<&AmountFieldConfigV1> for ReconMetadataFieldConfigV1 {
    fn from(config_v1: &AmountFieldConfigV1) -> Self {
        Self {
            field_name: EntryField::Amount,
            identifier: config_v1.identifier.clone(),
            field_config: FieldTypeConfig::V1(FieldTypeConfigV1::MinorUnit(config_v1.into())),
            required: true,
            description: "Main amount field".to_string(),
            is_exposed: None,
        }
    }
}

impl From<&AmountFieldConfigV1> for MinorUnitConfig {
    fn from(_: &AmountFieldConfigV1) -> Self {
        Self::V1(MinorUnitConfigV1 {
            validation_rules: vec![MinorUnitValidationRule::V1(
                MinorUnitValidationRuleV1::PositiveOnly,
            )],
            transformation_rules: vec![],
        })
    }
}
