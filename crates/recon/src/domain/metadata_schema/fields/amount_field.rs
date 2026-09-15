use common_types::metadata_schema::{
    fields::amount_field::{AmountFieldConfig, AmountFieldConfigV2, AmountUnitConfig},
    ReconMetadataFieldConfig,
};
use error_stack::{Report, ResultExt};

pub mod archived;

use crate::domain::{
    metadata_schema::{
        config_validator::{validate_non_empty_string, ConfigValidator},
        data_validator::{fetch_main_field_value, FieldValidator, ValueValidator},
        types::{Amount, ConfigValidationError, FieldValidationOutcome, MappedRow},
    },
    transformation::types::LocatedValue,
};

impl FieldValidator for AmountFieldConfig {
    type DataType = LocatedValue<Amount>;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        match self {
            Self::V1(amount_field_config_v1) => amount_field_config_v1
                .validate_field(entry)
                .map(|located| located.map(Amount::MinorUnit)),
            Self::V2(amount_field_config_v2) => amount_field_config_v2.validate_field(entry),
        }
    }
}

impl FieldValidator for AmountFieldConfigV2 {
    type DataType = LocatedValue<Amount>;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        fetch_main_field_value(entry, self.identifier.clone()).and_then(|located| {
            match &self.unit_config {
                AmountUnitConfig::MinorUnit(minor_config) => {
                    FieldValidationOutcome::from_required_value_located(
                        minor_config.validate_value(located.value),
                        &self.identifier,
                        &located.location,
                    )
                    .map(|amount| amount.map(Amount::MinorUnit))
                }
                AmountUnitConfig::MajorUnit(major_config) => {
                    FieldValidationOutcome::from_required_value_located(
                        major_config.validate_value(located.value),
                        &self.identifier,
                        &located.location,
                    )
                    .map(|amount| amount.map(Amount::FloatMajorUnit))
                }
            }
        })
    }
}

impl ConfigValidator for AmountFieldConfig {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
            Self::V2(config_v2) => config_v2.validate_config(),
        }
    }
}

impl ConfigValidator for AmountFieldConfigV2 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        validate_non_empty_string(&self.identifier).change_context(
            ConfigValidationError::EmptyFieldIdentifier {
                field_name: ReconMetadataFieldConfig::from(self).fetch_field_name(),
            },
        )?;

        match &self.unit_config {
            AmountUnitConfig::MinorUnit(minor_config) => {
                minor_config.validate_config().map_err(|config_error| {
                    ConfigValidationError::FieldError {
                        error: config_error.current_context().clone(),
                        identifier: self.identifier.clone(),
                        field_name: common_types::recon_rule::entry_field::EntryField::Amount,
                    }
                })?;
            }
            AmountUnitConfig::MajorUnit(major_config) => {
                major_config.validate_config().map_err(|config_error| {
                    ConfigValidationError::FieldError {
                        error: config_error.current_context().clone(),
                        identifier: self.identifier.clone(),
                        field_name: common_types::recon_rule::entry_field::EntryField::Amount,
                    }
                })?;
            }
        }

        Ok(())
    }
}
