use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    metadata_schema::{
        datatypes::datetime::{archived::DateTimeConfigV1, DateTimeConfig},
        FieldTypeConfig, FieldTypeConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    },
    recon_rule::entry_field::EntryField,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EffectiveAtFieldConfigV1 {
    pub identifier: String,
}

impl From<&EffectiveAtFieldConfigV1> for ReconMetadataFieldConfig {
    fn from(config_v1: &EffectiveAtFieldConfigV1) -> Self {
        Self::V1(config_v1.into())
    }
}

impl From<&EffectiveAtFieldConfigV1> for ReconMetadataFieldConfigV1 {
    fn from(config_v1: &EffectiveAtFieldConfigV1) -> Self {
        Self {
            field_name: EntryField::EffectiveAt,
            identifier: config_v1.identifier.clone(),
            field_config: FieldTypeConfig::V1(FieldTypeConfigV1::DateTime(config_v1.into())),
            required: true,
            description: "Main effective_at field".to_string(),
            is_exposed: None,
        }
    }
}

impl From<&EffectiveAtFieldConfigV1> for DateTimeConfig {
    fn from(_: &EffectiveAtFieldConfigV1) -> Self {
        Self::V1(DateTimeConfigV1 {})
    }
}
