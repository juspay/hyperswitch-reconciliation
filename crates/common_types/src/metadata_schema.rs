use crate::{
    composite_field::CompositeDelimiter,
    date_time::TimeRange,
    metadata_schema::{
        datatypes::{
            balance_direction, currency, datetime, enum_type, majorunit, minorunit, number, string,
        },
        fields::{
            amount_field, balance_direction_field, currency_field, effective_at_field,
            order_id_field,
        },
    },
    recon_rule::entry_field::EntryField,
};
use common_enums::StagingEntryProcessingMode;
use diesel::{AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub mod fields {
    pub mod amount_field;
    pub mod balance_direction_field;
    pub mod currency_field;
    pub mod effective_at_field;
    pub mod order_id_field;
}
pub mod datatypes {
    pub mod balance_direction;
    pub mod currency;
    pub mod datetime;
    pub mod enum_type;
    pub mod majorunit;
    pub mod minorunit;
    pub mod number;
    pub mod string;
}

// ======== Traits for Metadata Schema ========

/// Trait for types that can provide a processing mode for staging entry creation
pub trait ProcessingModeProvider {
    /// Get the processing mode defined in the schema
    fn get_processing_mode(&self) -> StagingEntryProcessingMode;
}

#[derive(Clone, Debug, Default)]
pub struct ExposedKeys {
    pub exposed: Vec<String>,
    pub searchable: Vec<String>,
}

pub static NO_EXPOSED_KEYS: ExposedKeys = ExposedKeys {
    exposed: Vec::new(),
    searchable: Vec::new(),
};

pub trait ExposedKeysProvider {
    fn exposed_keys(&self) -> ExposedKeys;
}

pub trait UniqueConstraintProvider {
    fn get_unique_constraint(&self) -> &UniqueConstraint;
}

// ======== Metadata Schema Definitions ========

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
#[serde(tag = "schema_type", rename_all = "snake_case")]
pub enum MetadataSchemaData {
    Recon(ReconMetadataSchema),
}

impl ProcessingModeProvider for MetadataSchemaData {
    fn get_processing_mode(&self) -> StagingEntryProcessingMode {
        match self {
            Self::Recon(recon_schema) => recon_schema.get_processing_mode(),
        }
    }
}

impl ExposedKeysProvider for MetadataSchemaData {
    fn exposed_keys(&self) -> ExposedKeys {
        match self {
            Self::Recon(recon_schema) => recon_schema.exposed_keys(),
        }
    }
}

impl UniqueConstraintProvider for MetadataSchemaData {
    fn get_unique_constraint(&self) -> &UniqueConstraint {
        match self {
            Self::Recon(recon_schema) => recon_schema.get_unique_constraint(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum ReconMetadataSchema {
    V1(ReconMetadataSchemaV1),
}

impl ProcessingModeProvider for ReconMetadataSchema {
    fn get_processing_mode(&self) -> StagingEntryProcessingMode {
        match self {
            Self::V1(schema_v1) => schema_v1.get_processing_mode(),
        }
    }
}

impl ExposedKeysProvider for ReconMetadataSchema {
    fn exposed_keys(&self) -> ExposedKeys {
        match self {
            Self::V1(schema_v1) => schema_v1.exposed_keys(),
        }
    }
}

impl UniqueConstraintProvider for ReconMetadataSchema {
    fn get_unique_constraint(&self) -> &UniqueConstraint {
        match self {
            Self::V1(schema_v1) => schema_v1.get_unique_constraint(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconMetadataSchemaV1 {
    pub fields: ReconFieldsConfig,
    pub unique_constraint: UniqueConstraint,
    pub processing_mode: StagingEntryProcessingMode,
}

impl ProcessingModeProvider for ReconMetadataSchemaV1 {
    fn get_processing_mode(&self) -> StagingEntryProcessingMode {
        self.processing_mode
    }
}

impl ExposedKeysProvider for ReconMetadataSchemaV1 {
    fn exposed_keys(&self) -> ExposedKeys {
        match &self.fields {
            ReconFieldsConfig::V1(fields_v1) => fields_v1.exposed_keys(),
        }
    }
}

impl UniqueConstraintProvider for ReconMetadataSchemaV1 {
    fn get_unique_constraint(&self) -> &UniqueConstraint {
        &self.unique_constraint
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "fields_config_version", rename_all = "snake_case")]
pub enum ReconFieldsConfig {
    V1(ReconFieldsConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconFieldsConfigV1 {
    // Required main fields
    pub currency: currency_field::CurrencyFieldConfig,
    pub amount: amount_field::AmountFieldConfig,
    pub effective_at: effective_at_field::EffectiveAtFieldConfig,
    pub balance_direction: balance_direction_field::BalanceDirectionFieldConfig,
    pub order_id: order_id_field::OrderIdFieldConfig,

    // Optional metadata fields
    pub metadata_fields: Vec<ReconMetadataFieldConfig>,
}

impl ExposedKeysProvider for ReconFieldsConfigV1 {
    fn exposed_keys(&self) -> ExposedKeys {
        let mut keys = ExposedKeys::default();
        for field in &self.metadata_fields {
            if let Some(expose_config) = field.is_exposed() {
                let field_name = field
                    .fetch_field_name()
                    .to_field_path_without_metadata_prefix();
                keys.exposed.push(field_name.clone());
                if expose_config.is_searchable {
                    keys.searchable.push(field_name);
                }
            }
        }
        keys
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "recon_metadata_field_config_version", rename_all = "snake_case")]
pub enum ReconMetadataFieldConfig {
    V1(ReconMetadataFieldConfigV1),
}

impl ReconMetadataFieldConfig {
    pub fn fetch_identifier(&self) -> String {
        match self {
            Self::V1(config_v1) => config_v1.identifier.clone(),
        }
    }

    pub fn fetch_field_name(&self) -> EntryField {
        match self {
            Self::V1(config_v1) => config_v1.field_name.clone(),
        }
    }

    pub fn is_exposed(&self) -> Option<MetadataExposeConfig> {
        match self {
            Self::V1(field_v1) => field_v1.is_exposed,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconMetadataFieldConfigV1 {
    pub field_name: EntryField,
    pub identifier: String,
    pub field_config: FieldTypeConfig,
    pub required: bool,
    pub description: String,
    #[serde(default)]
    pub is_exposed: Option<MetadataExposeConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct MetadataExposeConfig {
    pub is_searchable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "field_type_config_version", rename_all = "snake_case")]
pub enum FieldTypeConfig {
    V1(FieldTypeConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "field_type", rename_all = "snake_case")]
pub enum FieldTypeConfigV1 {
    String(string::StringConfig),
    Number(number::NumberConfig),
    Currency(currency::CurrencyConfig),
    BalanceDirection(balance_direction::BalanceDirectionConfig),
    DateTime(datetime::DateTimeConfig),
    MinorUnit(minorunit::MinorUnitConfig),
    MajorUnit(majorunit::MajorUnitConfig),
    Enum(enum_type::EnumConfig),
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum RuleType {
    MaxLength,
    MinLength,
    MaxValue,
    MinValue,
    PositiveOnly,
}

// ======== Unique Constraints ========

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "unique_constraint_version", rename_all = "snake_case")]
pub enum UniqueConstraint {
    V1(UniqueConstraintV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UniqueConstraintV1 {
    pub constraint_type: UniqueConstraintType,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "unique_constraint_type_version", rename_all = "snake_case")]
pub enum UniqueConstraintType {
    V1(UniqueConstraintTypeV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "unique_constraint_type", rename_all = "snake_case")]
pub enum UniqueConstraintTypeV1 {
    SingleField { field_name: EntryField },
    CompositeField(CompositeUniqueField),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CompositeUniqueField {
    pub field_names: Vec<EntryField>,
    pub delimiter: CompositeDelimiter,
}

// ====== Types =======

common_utils::impl_to_sql_from_sql_json!(MetadataSchemaData);

/// Filters for listing metadata schemas
#[derive(Debug, Clone, Default, Deserialize, IntoParams, ToSchema)]
pub struct ListMetadataSchemasFilters {
    /// Filter by schema_id
    #[param(example = "meta_sch_main_123")]
    pub schema_id: Option<String>,

    /// Filter by account_id
    #[param(example = "acc_123")]
    pub account_id: Option<String>,

    /// Filter by time range (created_at)
    #[serde(flatten, default)]
    #[param(value_type = TimeRange)]
    pub time_range: Option<TimeRange>,
}

impl ListMetadataSchemasFilters {
    /// Validates the time range filter
    ///
    /// Rules:
    /// - If end_time is provided, start_time must also be provided
    /// - If both are provided, start_time must be <= end_time
    /// - Only start_time without end_time is valid
    /// - Neither start_time nor end_time is valid (no time filtering)
    pub fn validate(&self) -> Result<(), String> {
        if let Some(time_range) = &self.time_range {
            if let Some(end_time) = time_range.end_time {
                if time_range.start_time > end_time {
                    return Err("start_time cannot be greater than end_time".to_string());
                }
            }
        }
        Ok(())
    }
}
