use common_enums::BalanceDirection;
use common_types::metadata_schema::{
    datatypes::balance_direction::BalanceDirectionConfig,
    fields::balance_direction_field::{BalanceDirectionFieldConfig, BalanceDirectionFieldConfigV1},
    ReconMetadataFieldConfig,
};
use error_stack::{Report, ResultExt};

use crate::domain::metadata_schema::{
    config_validator::{validate_non_empty_string, ConfigValidator},
    data_validator::{fetch_main_field_value, FieldValidator, ValueValidator},
    types::{ConfigValidationError, FieldValidationOutcome, MappedRow},
};

impl FieldValidator for BalanceDirectionFieldConfig {
    type DataType = BalanceDirection;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        match self {
            Self::V1(balance_direction_field_config_v1) => {
                balance_direction_field_config_v1.validate_field(entry)
            }
        }
    }
}

impl FieldValidator for BalanceDirectionFieldConfigV1 {
    type DataType = BalanceDirection;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        fetch_main_field_value(entry, self.identifier.clone()).and_then(|located| {
            FieldValidationOutcome::from_required_value(
                BalanceDirectionConfig::from(self).validate_value(located.value),
                &self.identifier,
                &located.location,
            )
        })
    }
}

impl ConfigValidator for BalanceDirectionFieldConfig {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for BalanceDirectionFieldConfigV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        let field_name = ReconMetadataFieldConfig::from(self).fetch_field_name();
        validate_non_empty_string(&self.identifier).change_context(
            ConfigValidationError::EmptyFieldIdentifier {
                field_name: field_name.clone(),
            },
        )?;
        // Using Metadata BalanceDirection validation implementation to validate credit and debit values
        BalanceDirectionConfig::from(self)
            .validate_config()
            .map_err(|report_err| {
                let error = report_err.current_context().clone();
                report_err.change_context(ConfigValidationError::FieldError {
                    error,
                    identifier: self.identifier.clone(),
                    field_name,
                })
            })?;
        Ok(())
    }
}
