use common_types::{
    metadata_schema::{
        datatypes::minorunit::MinorUnitConfig, fields::amount_field::archived::AmountFieldConfigV1,
        ReconMetadataFieldConfig,
    },
    units::MinorUnit,
};
use error_stack::{Report, ResultExt};

use crate::domain::{
    metadata_schema::{
        config_validator::validate_non_empty_string,
        data_validator::{fetch_main_field_value, FieldValidator, ValueValidator},
        types::{ConfigValidationError, FieldValidationOutcome},
        ConfigValidator, MappedRow,
    },
    transformation::types::LocatedValue,
};

impl FieldValidator for AmountFieldConfigV1 {
    type DataType = LocatedValue<MinorUnit>;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        fetch_main_field_value(entry, self.identifier.clone()).and_then(|located| {
            FieldValidationOutcome::from_required_value_located(
                MinorUnitConfig::from(self).validate_value(located.value),
                &self.identifier,
                &located.location,
            )
        })
    }
}

impl ConfigValidator for AmountFieldConfigV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        validate_non_empty_string(&self.identifier).change_context(
            ConfigValidationError::EmptyFieldIdentifier {
                field_name: ReconMetadataFieldConfig::from(self).fetch_field_name(),
            },
        )?;
        Ok(())
    }
}
