use std::collections::HashMap;

use common_types::metadata_schema::{
    FieldTypeConfig, FieldTypeConfigV1, MetadataSchemaData, ReconFieldsConfig, ReconFieldsConfigV1,
    ReconMetadataFieldConfig, ReconMetadataFieldConfigV1, ReconMetadataSchema,
    ReconMetadataSchemaV1,
};
use common_utils::logging;
use error_stack::Report;
use uuid::Uuid;

use crate::domain::{
    metadata_schema::types::{self, FieldError, RuleError, ValidatedFieldValue, ValueError},
    transformation::types::{LocatedValue, SourceLocation},
};

use super::types::{
    FieldValidationOutcome, MappedRow, RowRejection, RowValidationOutcome, ValidatedEntryData,
};

pub trait RowValidator {
    fn validate_row(&self, entry: &MappedRow) -> RowValidationOutcome;
}

pub trait FieldValidator {
    type DataType;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType>;
}

pub trait ValueValidator {
    type DataType;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>>;
}

pub trait RuleValidator {
    type DataType;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>>;
}

pub trait ValueTransformer {
    type DataType;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>>;
}

pub trait PostParseTransformer {
    type DataType;
    fn apply_post_parse_transformation(
        &self,
        value: &mut Self::DataType,
    ) -> Result<(), Report<RuleError>>;
}

impl RowValidator for MetadataSchemaData {
    fn validate_row(&self, row: &MappedRow) -> RowValidationOutcome {
        match self {
            Self::Recon(schema) => schema.validate_row(row),
        }
    }
}

impl RowValidator for ReconMetadataSchema {
    fn validate_row(&self, row: &MappedRow) -> RowValidationOutcome {
        match self {
            Self::V1(schema) => schema.validate_row(row),
        }
    }
}

impl RowValidator for ReconMetadataSchemaV1 {
    fn validate_row(&self, row: &MappedRow) -> RowValidationOutcome {
        self.fields.validate_row(row)
    }
}

impl RowValidator for ReconFieldsConfig {
    fn validate_row(&self, row: &MappedRow) -> RowValidationOutcome {
        match self {
            Self::V1(fields) => fields.validate_row(row),
        }
    }
}

fn reject_row(
    entry: &MappedRow,
    field_errors: Vec<LocatedValue<FieldError>>,
) -> RowValidationOutcome {
    RowValidationOutcome::Rejected(RowRejection {
        source_record: entry.source.clone(),
        field_errors,
    })
}

fn unmapped_field_location(identifier: &str) -> SourceLocation {
    SourceLocation::Field {
        field_name: identifier.to_string(),
    }
}

fn build_validated_metadata_from_fields(
    validated_fields: &HashMap<String, LocatedValue<Option<ValidatedFieldValue>>>,
) -> Result<serde_json::Value, Vec<LocatedValue<FieldError>>> {
    let mut metadata = serde_json::Map::new();
    let mut field_errors = Vec::new();

    for (field_name, located) in validated_fields {
        match &located.value {
            Some(value) => match value.to_json() {
                Ok(parsed_value) => {
                    metadata.insert(field_name.clone(), parsed_value);
                }
                Err(validation_error_type) => {
                    field_errors.push(LocatedValue {
                        value: validation_error_type.to_field_error(field_name.clone()),
                        location: located.location.clone(),
                    });
                }
            },
            None => {
                metadata.insert(field_name.clone(), serde_json::Value::Null);
            }
        };
    }
    if !field_errors.is_empty() {
        return Err(field_errors);
    }
    Ok(serde_json::Value::Object(metadata))
}

impl RowValidator for ReconFieldsConfigV1 {
    fn validate_row(&self, entry: &MappedRow) -> RowValidationOutcome {
        let currency = match self.currency.validate_field(entry) {
            FieldValidationOutcome::Valid(value) => value,
            FieldValidationOutcome::Invalid(error) => return reject_row(entry, vec![error]),
        };
        let amount = match self.amount.validate_field(entry) {
            FieldValidationOutcome::Valid(value) => value,
            FieldValidationOutcome::Invalid(error) => return reject_row(entry, vec![error]),
        };
        let balance_direction = match self.balance_direction.validate_field(entry) {
            FieldValidationOutcome::Valid(value) => value,
            FieldValidationOutcome::Invalid(error) => return reject_row(entry, vec![error]),
        };
        let effective_at = match self.effective_at.validate_field(entry) {
            FieldValidationOutcome::Valid(value) => value,
            FieldValidationOutcome::Invalid(error) => return reject_row(entry, vec![error]),
        };
        let order_id = match self.order_id.validate_field(entry) {
            FieldValidationOutcome::Valid(value) => value,
            FieldValidationOutcome::Invalid(error) => return reject_row(entry, vec![error]),
        };

        let mut all_fields: HashMap<String, LocatedValue<Option<ValidatedFieldValue>>> =
            HashMap::with_capacity(self.metadata_fields.len());
        let mut exposed_fields: HashMap<String, LocatedValue<Option<ValidatedFieldValue>>> =
            HashMap::new();
        let mut searchable_fields: HashMap<String, LocatedValue<Option<ValidatedFieldValue>>> =
            HashMap::new();
        let mut field_errors: Vec<LocatedValue<FieldError>> = Vec::new();

        for field in &self.metadata_fields {
            match field.validate_field(entry) {
                FieldValidationOutcome::Valid(validated_value) => {
                    let field_name = field
                        .fetch_field_name()
                        .to_field_path_without_metadata_prefix();
                    if let Some(expose_config) = field.is_exposed() {
                        exposed_fields.insert(field_name.clone(), validated_value.clone());
                        if expose_config.is_searchable {
                            searchable_fields.insert(field_name.clone(), validated_value.clone());
                        }
                    }
                    all_fields.insert(field_name, validated_value);
                }
                FieldValidationOutcome::Invalid(field_validation_error) => {
                    logging::error!(
                        "Field validation error for {:?}, and identifier {}: {field_validation_error:?}",
                        entry.source,
                        field.fetch_identifier()
                    );
                    field_errors.push(field_validation_error);
                }
            }
        }

        if !field_errors.is_empty() {
            return reject_row(entry, field_errors);
        }

        let metadata = match build_validated_metadata_from_fields(&all_fields) {
            Ok(metadata) => metadata,
            Err(json_field_errors) => return reject_row(entry, json_field_errors),
        };
        let exposed_metadata = match build_validated_metadata_from_fields(&exposed_fields) {
            Ok(metadata) => metadata,
            Err(json_field_errors) => return reject_row(entry, json_field_errors),
        };
        let searchable_metadata = match build_validated_metadata_from_fields(&searchable_fields) {
            Ok(metadata) => metadata,
            Err(json_field_errors) => return reject_row(entry, json_field_errors),
        };

        let amount = match amount.value.to_minor_unit(currency) {
            Ok(amount) => amount,
            Err(error) => {
                return reject_row(
                    entry,
                    vec![LocatedValue {
                        value: ValueError::JsonParseError {
                            actual_value: error,
                        }
                        .to_field_error("amount".to_string()),
                        location: amount.location,
                    }],
                )
            }
        };

        RowValidationOutcome::Validated(ValidatedEntryData::new(
            Uuid::new_v4(),
            entry.source.clone(),
            balance_direction,
            amount,
            currency,
            effective_at,
            order_id,
            metadata,
            exposed_metadata,
            searchable_metadata,
        ))
    }
}

impl FieldValidator for ReconMetadataFieldConfig {
    type DataType = LocatedValue<Option<ValidatedFieldValue>>;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        match self {
            Self::V1(config_v1) => config_v1.validate_field(entry),
        }
    }
}

impl FieldValidator for ReconMetadataFieldConfigV1 {
    type DataType = LocatedValue<Option<ValidatedFieldValue>>;
    fn validate_field(&self, entry: &MappedRow) -> FieldValidationOutcome<Self::DataType> {
        let field_identifier = self.identifier.clone();
        match entry.mapped_fields.get(&field_identifier) {
            Some(located) => match self.field_config.validate_value(located.value.clone()) {
                Ok(value) => {
                    if self.required && value.is_none() {
                        FieldValidationOutcome::invalid(
                            FieldError::RequiredFieldMissing(field_identifier),
                            &located.location,
                        )
                    } else {
                        FieldValidationOutcome::Valid(LocatedValue {
                            value,
                            location: located.location.clone(),
                        })
                    }
                }
                Err(validation_error) => FieldValidationOutcome::invalid(
                    validation_error
                        .current_context()
                        .to_field_error(field_identifier),
                    &located.location,
                ),
            },
            None => FieldValidationOutcome::invalid(
                FieldError::FieldNotMapped(field_identifier.clone()),
                &unmapped_field_location(&field_identifier),
            ),
            // This case means the field was there in schema but wasn't present in the entry, not even as null
        }
    }
}

impl ValueValidator for FieldTypeConfig {
    type DataType = ValidatedFieldValue;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(field_config_v1) => field_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for FieldTypeConfigV1 {
    type DataType = ValidatedFieldValue;

    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::String(string_config) => Ok(string_config
                .validate_value(value)?
                .map(ValidatedFieldValue::String)),
            Self::Number(number_config) => Ok(number_config
                .validate_value(value)?
                .map(ValidatedFieldValue::Number)),
            Self::Currency(currency_config) => Ok(currency_config
                .validate_value(value)?
                .map(ValidatedFieldValue::Currency)),
            Self::BalanceDirection(balance_config) => Ok(balance_config
                .validate_value(value)?
                .map(ValidatedFieldValue::BalanceDirection)),
            Self::DateTime(datetime_config) => Ok(datetime_config
                .validate_value(value)?
                .map(ValidatedFieldValue::DateTime)),
            Self::MinorUnit(minor_unit_config) => Ok(minor_unit_config
                .validate_value(value)?
                .map(types::Amount::MinorUnit)
                .map(ValidatedFieldValue::Amount)),
            Self::MajorUnit(major_unit_config) => Ok(major_unit_config
                .validate_value(value)?
                .map(types::Amount::FloatMajorUnit)
                .map(ValidatedFieldValue::Amount)),
            Self::Enum(enum_config) => Ok(enum_config
                .validate_value(value)?
                .map(ValidatedFieldValue::Enum)),
        }
    }
}

pub fn fetch_main_field_value(
    entry: &MappedRow,
    field: String,
) -> FieldValidationOutcome<LocatedValue<Option<String>>> {
    match entry.mapped_fields.get(&field) {
        Some(located) => FieldValidationOutcome::Valid(located.clone()),
        None => FieldValidationOutcome::invalid(
            FieldError::FieldNotMapped(field.clone()),
            &unmapped_field_location(&field),
        ),
    }
}
