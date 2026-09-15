use std::collections::HashMap;

use common_enums::{BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::{
    composite_field::CompositeDelimiter,
    metadata_schema::datatypes::{
        datetime::DateTimeFormat,
        majorunit::{AmountDelimiter, ReplaceMode},
    },
    recon_rule::entry_field::EntryField,
    units::{FloatMajorUnit, MinorUnit},
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

/// Request to create a new metadata schema
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateMetadataSchemaRequest {
    #[schema(example = "acc_123456")]
    pub account_id: String,
    pub schema_data: MetadataSchemaData,
}

/// Request to update an existing metadata schema (creates new version)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetadataSchemaUpdateRequest {
    pub schema_data: MetadataSchemaData,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "schema_type", rename_all = "snake_case")]
pub enum MetadataSchemaData {
    BasicSchemaData {
        fields: FieldsConfig,
        unique_constraint: UniqueConstraint,
        processing_mode: StagingEntryProcessingMode,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FieldsConfig {
    // Required main fields
    pub currency: CurrencySchemaConfig,
    pub amount: AmountSchemaConfig,
    pub effective_at: EffectiveAtSchemaConfig,
    pub balance_direction: BalanceDirectionSchemaConfig,
    pub order_id: OrderIdSchemaConfig,

    // Optional metadata fields
    pub metadata_fields: Vec<MetadataFieldSchemaConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CurrencySchemaConfig {
    #[schema(example = "currency_code")]
    pub identifier: String,
    #[serde(default)]
    pub transformation_rules: Vec<CurrencyTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "unit_type", rename_all = "snake_case")]
pub enum AmountUnitConfigApi {
    MinorUnit {
        validation_rules: Vec<MinorUnitValidationRule>,
        #[serde(default)]
        transformation_rules: Vec<MinorUnitTransformationRule>,
    },
    MajorUnit {
        delimiter: AmountDelimiter,
        validation_rules: Vec<MajorUnitValidationRule>,
        #[serde(default)]
        transformation_rules: Vec<MajorUnitTransformationRule>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AmountSchemaConfig {
    #[schema(example = "transaction_amount")]
    pub identifier: String,
    #[serde(flatten)]
    pub unit_config: AmountUnitConfigApi,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EffectiveAtSchemaConfig {
    #[schema(example = "effective_timestamp")]
    pub identifier: String,
    #[schema(example = json!({
        "date_format": {
            "order": "DAY_MONTH_YEAR",
            "delimiter": "hyphen"
        },
        "time_format": null
    }))]
    pub date_time_format: DateTimeFormat,
    #[serde(default)]
    pub transformation_rules: Vec<DateTimeTransformationRule>,
    #[serde(default)]
    pub post_parse_transformations: Vec<DateTimePostParseTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BalanceDirectionSchemaConfig {
    #[schema(example = "direction")]
    pub identifier: String,
    #[schema(example = "[\"credit\", \"cr\"]")]
    pub credit_values: Vec<String>,
    #[schema(example = "[\"debit\", \"dr\"]")]
    pub debit_values: Vec<String>,
    #[serde(default)]
    pub transformation_rules: Vec<BalanceDirectionTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OrderIdSchemaConfig {
    #[schema(example = "order_id")]
    pub identifier: String,
    pub transformation_rules: Vec<StringTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MetadataFieldSchemaConfig {
    #[schema(example = "customer_name")]
    pub identifier: String,
    #[schema(example = "metadata.customer_name")]
    pub field_name: EntryField,
    #[serde(flatten)]
    pub field_config: FieldType,
    #[schema(example = true)]
    pub required: bool,
    #[schema(example = "Customer's full name")]
    pub description: String,
    #[serde(default)]
    pub is_exposed: Option<MetadataExposeConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct MetadataExposeConfig {
    #[schema(example = true)]
    pub is_searchable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "field_type", rename_all = "snake_case")]
pub enum FieldType {
    String {
        validation_rules: Vec<StringValidationRule>,
        transformation_rules: Vec<StringTransformationRule>,
    },
    Number {
        validation_rules: Vec<NumberValidationRule>,
        #[serde(default)]
        transformation_rules: Vec<NumberTransformationRule>,
    },
    Currency {
        #[serde(default)]
        transformation_rules: Vec<CurrencyTransformationRule>,
    },
    MinorUnit {
        validation_rules: Vec<MinorUnitValidationRule>,
        #[serde(default)]
        transformation_rules: Vec<MinorUnitTransformationRule>,
    },
    MajorUnit {
        delimiter: AmountDelimiter,
        validation_rules: Vec<MajorUnitValidationRule>,
        #[serde(default)]
        transformation_rules: Vec<MajorUnitTransformationRule>,
    },
    DateTime {
        date_time_format: DateTimeFormat,
        #[serde(default)]
        transformation_rules: Vec<DateTimeTransformationRule>,
        #[serde(default)]
        post_parse_transformations: Vec<DateTimePostParseTransformationRule>,
    },
    BalanceDirection {
        #[schema(example = "[\"credit\", \"cr\"]")]
        credit_values: Vec<String>,
        #[schema(example = "[\"debit\", \"dr\"]")]
        debit_values: Vec<String>,
        #[serde(default)]
        transformation_rules: Vec<BalanceDirectionTransformationRule>,
    },
    Enum {
        mappings: HashMap<String, String>,
        #[serde(default)]
        transformation_rules: Vec<EnumTransformationRule>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "validation_rule_type", rename_all = "snake_case")]
pub enum StringValidationRule {
    MaxLength {
        #[schema(example = 100)]
        value: usize,
    },
    MinLength {
        #[schema(example = 1)]
        value: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum StringTransformationRule {
    DefaultValue {
        #[schema(example = "fallback")]
        value: String,
    },
    ToUpperCase,
    ToLowerCase,
    StripPrefix {
        #[schema(example = "Mr. ")]
        prefix: String,
    },
    StripSuffix {
        #[schema(example = " Jr.")]
        suffix: String,
    },
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
    Regex {
        #[schema(example = "Order_ID:\\s*(\\d+)")]
        pattern: String,
        #[schema(example = 1)]
        group: Option<usize>,
    },
    ReplaceChar {
        #[schema(value_type = String)]
        from: char,
        #[schema(value_type = Option<String>)]
        to: Option<char>,
        mode: ReplaceMode,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum CurrencyTransformationRule {
    DefaultValue {
        #[schema(value_type = Currency)]
        currency: Currency,
    },
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum BalanceDirectionTransformationRule {
    DefaultValue {
        #[schema(value_type = BalanceDirection)]
        direction: BalanceDirection,
    },
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
    StartsWith {
        #[schema(example = "CR")]
        prefix: String,
        #[schema(example = "credit")]
        then: String,
        #[schema(example = "debit")]
        otherwise: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum NumberTransformationRule {
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum MinorUnitTransformationRule {
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
    Absolute,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum MajorUnitTransformationRule {
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
    Negate,
    Absolute,
    ReplaceChar {
        #[schema(value_type = String)]
        from: char,
        #[schema(value_type = Option<String>)]
        to: Option<char>,
        mode: ReplaceMode,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum DateTimeTransformationRule {
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
    Regex {
        #[schema(example = "Totals for Payment on (\\d{2}/\\d{2}/\\d{4})")]
        pattern: String,
        #[schema(example = 1)]
        group: Option<usize>,
    },
}

/// Post-parse rules for datetime fields. Run on the parsed `PrimitiveDateTime` after parsing.
/// Used to derive grouping keys (hour/day/month/year buckets) and shifted timestamps for joins.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "post_parse_rule_type", rename_all = "snake_case")]
pub enum DateTimePostParseTransformationRule {
    /// Truncate the parsed datetime to the start of the specified precision.
    Truncate { precision: TruncationPrecision },
    /// Add a duration to the parsed datetime.
    AddDuration { duration: DateTimeDuration },
    /// Subtract a duration from the parsed datetime.
    SubtractDuration { duration: DateTimeDuration },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TruncationPrecision {
    StartOfHour,
    StartOfDay,
    StartOfMonth,
    StartOfYear,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct DateTimeDuration {
    #[schema(example = 1)]
    pub value: u32,
    pub unit: DurationUnit,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DurationUnit {
    Minutes,
    Hours,
    Days,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum EnumTransformationRule {
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "validation_rule_type", rename_all = "snake_case")]
pub enum NumberValidationRule {
    MinValue {
        #[schema(example = 0.0)]
        value: f64,
    },
    MaxValue {
        #[schema(example = 1000000.0)]
        value: f64,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "validation_rule_type", rename_all = "snake_case")]
pub enum MinorUnitValidationRule {
    PositiveOnly,
    MinValue {
        #[schema(example = 100)]
        value: MinorUnit,
    },
    MaxValue {
        #[schema(example = 1000000)]
        value: MinorUnit,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "validation_rule_type", rename_all = "snake_case")]
pub enum MajorUnitValidationRule {
    PositiveOnly,
    MinValue {
        #[schema(example = 100.0)]
        value: FloatMajorUnit,
    },
    MaxValue {
        #[schema(example = 1000000.0)]
        value: FloatMajorUnit,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UniqueConstraint {
    pub constraint_type: UniqueConstraintType,

    #[schema(example = "Customer name must be unique per account")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "unique_constraint_type", rename_all = "snake_case")]
pub enum UniqueConstraintType {
    SingleField {
        #[schema(example = "metadata.customer_name")]
        field_name: EntryField,
    },
    CompositeField {
        #[schema(example = json!(["metadata.customer_name", "metadata.order_ref"]))]
        field_names: Vec<EntryField>,
        delimiter: CompositeDelimiter,
    },
}

/// Metadata schema response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetadataSchemaResponse {
    pub id: String,
    pub schema_id: String,
    pub profile_id: String,
    pub account_id: String,
    pub schema_data: MetadataSchemaDataResponse,
    pub version: i32,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}

/// List of metadata schemas response
pub type ListMetadataSchemasResponse = Vec<MetadataSchemaResponse>;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "schema_type", rename_all = "snake_case")]
pub enum MetadataSchemaDataResponse {
    BasicSchemaData {
        fields: FieldsConfigResponse,
        unique_constraint: UniqueConstraintResponse,
        processing_mode: StagingEntryProcessingMode,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FieldsConfigResponse {
    // Required main fields
    pub currency: CurrencySchemaConfigResponse,
    pub amount: AmountSchemaConfigResponse,
    pub effective_at: EffectiveAtSchemaConfigResponse,
    pub balance_direction: BalanceDirectionSchemaConfigResponse,
    pub order_id: OrderIdSchemaConfigResponse,

    // Optional metadata fields
    pub metadata_fields: Vec<MetadataFieldSchemaConfigResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CurrencySchemaConfigResponse {
    #[schema(example = "currency_code")]
    pub identifier: String,
    pub transformation_rules: Vec<CurrencyTransformationRuleResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AmountSchemaConfigResponse {
    #[schema(example = "transaction_amount")]
    pub identifier: String,
    #[serde(flatten)]
    pub unit_config: AmountUnitConfigApi,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EffectiveAtSchemaConfigResponse {
    #[schema(example = "effective_timestamp")]
    pub identifier: String,
    pub date_time_format: DateTimeFormat,
    pub transformation_rules: Vec<DateTimeTransformationRule>,
    pub post_parse_transformations: Vec<DateTimePostParseTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BalanceDirectionSchemaConfigResponse {
    #[schema(example = "direction")]
    pub identifier: String,
    #[schema(example = "[\"credit\", \"cr\"]")]
    pub credit_values: Vec<String>,
    #[schema(example = "[\"debit\", \"dr\"]")]
    pub debit_values: Vec<String>,
    pub transformation_rules: Vec<BalanceDirectionTransformationRuleResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OrderIdSchemaConfigResponse {
    #[schema(example = "order_id")]
    pub identifier: String,
    pub transformation_rules: Vec<StringTransformationRuleResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MetadataFieldSchemaConfigResponse {
    #[schema(example = "customer_name")]
    pub identifier: String,
    #[schema(example = "metadata.customer_name")]
    pub field_name: EntryField,
    #[serde(flatten)]
    pub field_config: FieldTypeResponse,
    #[schema(example = true)]
    pub required: bool,
    #[schema(example = "Customer's full name")]
    pub description: String,
    #[serde(default)]
    pub is_exposed: Option<MetadataExposeConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "field_type", rename_all = "snake_case")]
pub enum FieldTypeResponse {
    String {
        validation_rules: Vec<StringValidationRuleResponse>,
        transformation_rules: Vec<StringTransformationRuleResponse>,
    },
    Number {
        validation_rules: Vec<NumberValidationRuleResponse>,
        transformation_rules: Vec<NumberTransformationRule>,
    },
    Currency {
        transformation_rules: Vec<CurrencyTransformationRuleResponse>,
    },
    MinorUnit {
        validation_rules: Vec<MinorUnitValidationRuleResponse>,
        transformation_rules: Vec<MinorUnitTransformationRule>,
    },
    MajorUnit {
        delimiter: AmountDelimiter,
        validation_rules: Vec<MajorUnitValidationRule>,
        transformation_rules: Vec<MajorUnitTransformationRule>,
    },
    DateTime {
        date_time_format: Option<DateTimeFormat>,
        transformation_rules: Vec<DateTimeTransformationRule>,
        post_parse_transformations: Vec<DateTimePostParseTransformationRule>,
    },
    BalanceDirection {
        #[schema(example = "[\"credit\", \"cr\"]")]
        credit_values: Vec<String>,
        #[schema(example = "[\"debit\", \"dr\"]")]
        debit_values: Vec<String>,
        transformation_rules: Vec<BalanceDirectionTransformationRuleResponse>,
    },
    Enum {
        mappings: HashMap<String, String>,
        transformation_rules: Vec<EnumTransformationRule>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "validation_rule_type", rename_all = "snake_case")]
pub enum StringValidationRuleResponse {
    MaxLength {
        #[schema(example = 100)]
        value: usize,
    },
    MinLength {
        #[schema(example = 1)]
        value: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum StringTransformationRuleResponse {
    DefaultValue {
        #[schema(example = "fallback")]
        value: String,
    },
    ToUpperCase,
    ToLowerCase,
    StripPrefix {
        #[schema(example = "Mr. ")]
        prefix: String,
    },
    StripSuffix {
        #[schema(example = " Jr.")]
        suffix: String,
    },
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
    Regex {
        #[schema(example = "Order_ID:\\s*(\\d+)")]
        pattern: String,
        #[schema(example = 1)]
        group: Option<usize>,
    },
    ReplaceChar {
        #[schema(value_type = String)]
        from: char,
        #[schema(value_type = Option<String>)]
        to: Option<char>,
        mode: ReplaceMode,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum CurrencyTransformationRuleResponse {
    DefaultValue {
        #[schema(value_type = Currency)]
        currency: Currency,
    },
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_rule_type", rename_all = "snake_case")]
pub enum BalanceDirectionTransformationRuleResponse {
    DefaultValue {
        #[schema(value_type = BalanceDirection)]
        direction: BalanceDirection,
    },
    Trim,
    JsonExtract {
        #[schema(example = "/pspReference")]
        pointer: String,
    },
    StartsWith {
        #[schema(example = "CR")]
        prefix: String,
        #[schema(example = "credit")]
        then: String,
        #[schema(example = "debit")]
        otherwise: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "validation_rule_type", rename_all = "snake_case")]
pub enum NumberValidationRuleResponse {
    MinValue {
        #[schema(example = 0.0)]
        value: f64,
    },
    MaxValue {
        #[schema(example = 1000000.0)]
        value: f64,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "validation_rule_type", rename_all = "snake_case")]
pub enum MinorUnitValidationRuleResponse {
    PositiveOnly,
    MinValue {
        #[schema(example = 100)]
        value: MinorUnit,
    },
    MaxValue {
        #[schema(example = 1000000)]
        value: MinorUnit,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UniqueConstraintResponse {
    pub constraint_type: UniqueConstraintTypeResponse,

    #[schema(example = "Customer name must be unique per account")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "unique_constraint_type", rename_all = "snake_case")]
pub enum UniqueConstraintTypeResponse {
    SingleField {
        #[schema(example = "metadata.customer_name")]
        field_name: EntryField,
    },
    CompositeField {
        #[schema(example = json!(["metadata.customer_name", "metadata.order_ref"]))]
        field_names: Vec<EntryField>,
        delimiter: CompositeDelimiter,
    },
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetadataSchemaResponseV2 {
    pub id: String,
    pub schema_id: String,
    pub profile_id: String,
    pub account_id: String,
    pub schema_data: MetadataSchemaDataResponseV2,
    pub version: i32,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "schema_type", rename_all = "snake_case")]
pub enum MetadataSchemaDataResponseV2 {
    BasicSchemaData {
        fields: FieldsConfigResponseV2,
        unique_constraint: UniqueConstraintResponse,
        processing_mode: StagingEntryProcessingMode,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FieldsConfigResponseV2 {
    // Required main fields
    pub main_fields: Vec<MainFieldsResponse>,

    // Optional metadata fields
    pub metadata_fields: Vec<MetadataFieldSchemaConfigResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "field_name", rename_all = "snake_case")]
pub enum MainFieldsResponse {
    Currency(CurrencySchemaConfigResponse),
    Amount(AmountSchemaConfigResponse),
    EffectiveAt(EffectiveAtSchemaConfigResponse),
    BalanceDirection(BalanceDirectionSchemaConfigResponse),
    OrderId(OrderIdSchemaConfigResponse),
}
