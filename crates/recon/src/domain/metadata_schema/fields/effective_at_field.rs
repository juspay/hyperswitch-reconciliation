use common_types::metadata_schema::{
    datatypes::datetime::DateTimeConfig,
    fields::effective_at_field::{EffectiveAtFieldConfig, EffectiveAtFieldConfigV2},
    ReconMetadataFieldConfig,
};
use error_stack::{Report, ResultExt};
use time::PrimitiveDateTime;

pub mod archived;

use crate::domain::metadata_schema::{
    config_validator::{validate_non_empty_string, ConfigValidator},
    data_validator::{fetch_main_field_value, FieldValidator, ValueValidator},
    types::{ConfigValidationError, FieldValidationOutcome, MappedRow},
};

impl FieldValidator for EffectiveAtFieldConfig {
    type DataType = PrimitiveDateTime;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        match self {
            Self::V1(effective_at_field_config_v1) => {
                effective_at_field_config_v1.validate_field(entry)
            }
            Self::V2(effective_at_field_config_v2) => {
                effective_at_field_config_v2.validate_field(entry)
            }
        }
    }
}

impl FieldValidator for EffectiveAtFieldConfigV2 {
    type DataType = PrimitiveDateTime;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        fetch_main_field_value(entry, self.identifier.clone()).and_then(|located| {
            FieldValidationOutcome::from_required_value(
                DateTimeConfig::from(self).validate_value(located.value),
                &self.identifier,
                &located.location,
            )
        })
    }
}

impl ConfigValidator for EffectiveAtFieldConfig {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
            Self::V2(config_v2) => config_v2.validate_config(),
        }
    }
}

impl ConfigValidator for EffectiveAtFieldConfigV2 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        validate_non_empty_string(&self.identifier).change_context(
            ConfigValidationError::EmptyFieldIdentifier {
                field_name: ReconMetadataFieldConfig::from(self).fetch_field_name(),
            },
        )?;

        self.date_time_format
            .build_format_description()
            .change_context(ConfigValidationError::InvalidDateTimeConfig)
            .map(|_| ())
    }
}
