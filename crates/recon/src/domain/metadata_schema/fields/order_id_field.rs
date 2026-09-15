use common_types::metadata_schema::{
    datatypes::string::StringConfig,
    fields::order_id_field::{OrderIdFieldConfig, OrderIdFieldConfigV1},
    ReconMetadataFieldConfig,
};
use error_stack::{Report, ResultExt};

use crate::domain::metadata_schema::{
    config_validator::{validate_non_empty_string, ConfigValidator},
    data_validator::{fetch_main_field_value, FieldValidator, ValueValidator},
    types::{ConfigValidationError, FieldValidationOutcome, MappedRow},
};

impl FieldValidator for OrderIdFieldConfig {
    type DataType = String;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        match self {
            Self::V1(order_id_field_config_v1) => order_id_field_config_v1.validate_field(entry),
        }
    }
}

impl FieldValidator for OrderIdFieldConfigV1 {
    type DataType = String;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        fetch_main_field_value(entry, self.identifier.clone()).and_then(|located| {
            FieldValidationOutcome::from_required_value(
                StringConfig::from(self).validate_value(located.value),
                &self.identifier,
                &located.location,
            )
        })
    }
}

impl ConfigValidator for OrderIdFieldConfig {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for OrderIdFieldConfigV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        validate_non_empty_string(&self.identifier).change_context(
            ConfigValidationError::EmptyFieldIdentifier {
                field_name: ReconMetadataFieldConfig::from(self).fetch_field_name(),
            },
        )
    }
}
