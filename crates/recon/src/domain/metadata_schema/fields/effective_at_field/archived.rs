use common_types::metadata_schema::{
    datatypes::datetime::DateTimeConfig,
    fields::effective_at_field::archived::EffectiveAtFieldConfigV1, ReconMetadataFieldConfig,
};
use error_stack::{Report, ResultExt};
use time::PrimitiveDateTime;

use crate::domain::metadata_schema::{
    config_validator::validate_non_empty_string,
    data_validator::{fetch_main_field_value, FieldValidator, ValueValidator},
    types::{ConfigValidationError, FieldValidationOutcome},
    ConfigValidator, MappedRow,
};

impl FieldValidator for EffectiveAtFieldConfigV1 {
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

impl ConfigValidator for EffectiveAtFieldConfigV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        validate_non_empty_string(&self.identifier).change_context(
            ConfigValidationError::EmptyFieldIdentifier {
                field_name: ReconMetadataFieldConfig::from(self).fetch_field_name(),
            },
        )
    }
}
