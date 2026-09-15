use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    metadata_schema::{
        datatypes::currency::{CurrencyConfig, CurrencyConfigV1, CurrencyTransformationRule},
        FieldTypeConfig, FieldTypeConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    },
    recon_rule::entry_field::EntryField,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "currency_field_config_version", rename_all = "snake_case")]
pub enum CurrencyFieldConfig {
    V1(CurrencyFieldConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CurrencyFieldConfigV1 {
    pub identifier: String,
    #[serde(default)]
    pub transformation_rules: Vec<CurrencyTransformationRule>,
}

impl From<&CurrencyFieldConfig> for ReconMetadataFieldConfig {
    fn from(config: &CurrencyFieldConfig) -> Self {
        match config {
            CurrencyFieldConfig::V1(config_v1) => config_v1.into(),
        }
    }
}

impl From<&CurrencyFieldConfigV1> for ReconMetadataFieldConfig {
    fn from(config_v1: &CurrencyFieldConfigV1) -> Self {
        Self::V1(config_v1.into())
    }
}

impl From<&CurrencyFieldConfigV1> for ReconMetadataFieldConfigV1 {
    fn from(config_v1: &CurrencyFieldConfigV1) -> Self {
        Self {
            field_name: EntryField::Currency,
            identifier: config_v1.identifier.clone(),
            field_config: FieldTypeConfig::V1(FieldTypeConfigV1::Currency(config_v1.into())),
            required: true,
            description: "Main currency field".to_string(),
            is_exposed: None,
        }
    }
}

impl From<&CurrencyFieldConfig> for CurrencyConfig {
    fn from(value: &CurrencyFieldConfig) -> Self {
        match value {
            CurrencyFieldConfig::V1(config_v1) => config_v1.into(),
        }
    }
}

impl From<&CurrencyFieldConfigV1> for CurrencyConfig {
    fn from(config_v1: &CurrencyFieldConfigV1) -> Self {
        Self::V1(CurrencyConfigV1 {
            transformation_rules: config_v1.transformation_rules.clone(),
        })
    }
}
