use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod archived;

use crate::{
    metadata_schema::{
        datatypes::datetime::{
            DateTimeConfig, DateTimeConfigV2, DateTimeFormat, DateTimePostParseTransformationRule,
            DateTimeTransformationRule,
        },
        fields::effective_at_field::archived::EffectiveAtFieldConfigV1,
        FieldTypeConfig, FieldTypeConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    },
    recon_rule::entry_field::EntryField,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "effective_at_field_config_version", rename_all = "snake_case")]
pub enum EffectiveAtFieldConfig {
    V1(EffectiveAtFieldConfigV1),
    V2(EffectiveAtFieldConfigV2),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EffectiveAtFieldConfigV2 {
    pub identifier: String,
    pub date_time_format: DateTimeFormat,
    #[serde(default)]
    pub transformation_rules: Vec<DateTimeTransformationRule>,
    #[serde(default)]
    pub post_parse_transformations: Vec<DateTimePostParseTransformationRule>,
}

impl From<&EffectiveAtFieldConfig> for ReconMetadataFieldConfig {
    fn from(config: &EffectiveAtFieldConfig) -> Self {
        match config {
            EffectiveAtFieldConfig::V1(config_v1) => config_v1.into(),
            EffectiveAtFieldConfig::V2(config_v2) => config_v2.into(),
        }
    }
}

impl From<&EffectiveAtFieldConfigV2> for ReconMetadataFieldConfig {
    fn from(config_v2: &EffectiveAtFieldConfigV2) -> Self {
        Self::V1(config_v2.into())
    }
}

impl From<&EffectiveAtFieldConfigV2> for ReconMetadataFieldConfigV1 {
    fn from(config_v2: &EffectiveAtFieldConfigV2) -> Self {
        Self {
            field_name: EntryField::EffectiveAt,
            identifier: config_v2.identifier.clone(),
            field_config: FieldTypeConfig::V1(FieldTypeConfigV1::DateTime(config_v2.into())),
            required: true,
            description: "Main effective_at field".to_string(),
            is_exposed: None,
        }
    }
}

impl From<&EffectiveAtFieldConfig> for DateTimeConfig {
    fn from(value: &EffectiveAtFieldConfig) -> Self {
        match value {
            EffectiveAtFieldConfig::V1(config_v1) => config_v1.into(),
            EffectiveAtFieldConfig::V2(config_v2) => config_v2.into(),
        }
    }
}

impl From<&EffectiveAtFieldConfigV2> for DateTimeConfig {
    fn from(value: &EffectiveAtFieldConfigV2) -> Self {
        Self::V2(DateTimeConfigV2 {
            format: value.date_time_format.clone(),
            transformation_rules: value.transformation_rules.clone(),
            post_parse_transformations: value.post_parse_transformations.clone(),
        })
    }
}
