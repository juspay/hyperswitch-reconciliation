//! Configuration validation for metadata schemas
//! This module provides validation functionality for metadata schema configurations
//! themselves (not the values against schemas). It ensures that schema definitions
//! are valid, consistent, and follow business rules.

use common_types::{
    composite_field::build_composite,
    metadata_schema::{
        CompositeUniqueField, FieldTypeConfig, FieldTypeConfigV1, MetadataSchemaData,
        ReconFieldsConfig, ReconFieldsConfigV1, ReconMetadataFieldConfig,
        ReconMetadataFieldConfigV1, ReconMetadataSchema, ReconMetadataSchemaV1, RuleType,
        UniqueConstraint, UniqueConstraintType, UniqueConstraintTypeV1, UniqueConstraintV1,
    },
    recon_rule::entry_field::EntryField,
};
use error_stack::{Report, ResultExt};

use crate::domain::metadata_schema::{
    implementations::unique_constraint::UniqueConstraintExt,
    types::{ConfigFieldValidationError, ConfigValidationError},
};
use std::collections::HashSet;

/// Trait for validating metadata schema configurations
pub trait ConfigValidator {
    type ErrorType;
    /// Validates the configuration and returns any errors found
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>>;
}

pub trait ToRuleType {
    fn to_rule_type(&self) -> RuleType;
}

impl ConfigValidator for MetadataSchemaData {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::Recon(recon_schema) => recon_schema.validate_config(),
        }
    }
}

impl ConfigValidator for ReconMetadataSchema {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(schema_v1) => schema_v1.validate_config(),
        }
    }
}

impl ConfigValidator for ReconMetadataSchemaV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        // Validate fields configuration
        self.fields.validate_config()?;

        // Validate unique constraints
        self.unique_constraint.validate_config()?;

        Ok(())
    }
}

impl ConfigValidator for ReconFieldsConfig {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(fields_v1) => fields_v1.validate_config(),
        }
    }
}

impl ConfigValidator for ReconFieldsConfigV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        self.currency.validate_config()?;
        self.amount.validate_config()?;
        self.effective_at.validate_config()?;
        self.balance_direction.validate_config()?;

        // Field names should be unique
        self.metadata_fields.iter().try_fold(
            HashSet::new(),
            |mut used_internal_fields,
             metadata_field|
             -> Result<HashSet<EntryField>, Report<Self::ErrorType>> {
                let field_name = metadata_field.fetch_field_name();

                if !used_internal_fields.insert(field_name.clone()) {
                    return Err(
                        (ConfigValidationError::DuplicateMetadataFieldName { field_name }).into(),
                    );
                }

                metadata_field.validate_config()?;

                Ok(used_internal_fields)
            },
        )?;

        Ok(())
    }
}

impl ConfigValidator for ReconMetadataFieldConfig {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for ReconMetadataFieldConfigV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        validate_non_empty_string(&self.identifier).change_context(
            ConfigValidationError::EmptyFieldIdentifier {
                field_name: self.field_name.clone(),
            },
        )?;

        validate_non_empty_string(&self.description).change_context(
            ConfigValidationError::EmptyFieldDescription {
                identifier: self.identifier.clone(),
            },
        )?;

        self.field_config.validate_config().map_err(|report_err| {
            let error = report_err.current_context().clone();
            report_err.change_context(ConfigValidationError::FieldError {
                error,
                identifier: self.identifier.clone(),
                field_name: self.field_name.clone(),
            })
        })?;

        Ok(())
    }
}

impl ConfigValidator for FieldTypeConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for FieldTypeConfigV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::String(string_config) => string_config.validate_config(),
            Self::Number(number_config) => number_config.validate_config(),
            Self::Currency(currency_config) => currency_config.validate_config(),
            Self::BalanceDirection(balance_direction_config) => {
                balance_direction_config.validate_config()
            }
            Self::DateTime(datetime_config) => datetime_config.validate_config(),
            Self::MinorUnit(minor_unit_config) => minor_unit_config.validate_config(),
            Self::MajorUnit(major_unit_config) => major_unit_config.validate_config(),
            Self::Enum(enum_config) => enum_config.validate_config(),
        }
    }
}

impl ConfigValidator for UniqueConstraint {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(constraint_v1) => constraint_v1.validate_config(),
        }
    }
}

impl ConfigValidator for UniqueConstraintV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        // Validate description
        validate_non_empty_string(&self.description).change_context(
            ConfigValidationError::EmptyFieldDescription {
                // We don't have identifier here, so use the constrained field(s) instead
                identifier: build_composite(
                    self.constraint_type
                        .get_unique_fields()
                        .iter()
                        .map(EntryField::to_string),
                    &self
                        .constraint_type
                        .get_unique_field_delimiter()
                        .to_string(),
                ),
            },
        )?;

        // Validate constraint type
        self.constraint_type.validate_config()?;
        Ok(())
    }
}

impl ConfigValidator for UniqueConstraintType {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(constraint_type_v1) => constraint_type_v1.validate_config(),
        }
    }
}

impl ConfigValidator for UniqueConstraintTypeV1 {
    type ErrorType = ConfigValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::SingleField { field_name: _ } => Ok(()),
            Self::CompositeField(CompositeUniqueField { field_names, .. }) => {
                validate_composite_unique_fields(field_names)
            }
        }
    }
}

fn validate_composite_unique_fields(
    field_names: &[EntryField],
) -> Result<(), Report<ConfigValidationError>> {
    if field_names.is_empty() {
        return Err(ConfigValidationError::EmptyCompositeUniqueConstraint.into());
    }

    let mut seen = HashSet::new();
    if let Some(duplicate) = field_names.iter().find(|field| !seen.insert(*field)) {
        return Err(ConfigValidationError::DuplicateMetadataFieldName {
            field_name: duplicate.clone(),
        }
        .into());
    }
    Ok(())
}

pub fn validate_non_empty_string(value: &str) -> Result<(), Report<ConfigValidationError>> {
    if value.trim().is_empty() {
        return Err(ConfigValidationError::EmptyString.into());
    }
    Ok(())
}

pub fn validate_number(value: f64) -> Result<(), Report<ConfigValidationError>> {
    if value.is_nan() || value.is_infinite() {
        return Err((ConfigValidationError::InvalidNumber {
            value: value.to_string(),
        })
        .into());
    }
    Ok(())
}
