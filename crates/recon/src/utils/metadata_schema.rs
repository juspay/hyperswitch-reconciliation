use api_models::metadata_schema::{
    AmountSchemaConfig as AmountSchemaConfigApi, AmountSchemaConfigResponse, AmountUnitConfigApi,
    BalanceDirectionSchemaConfig as BalanceDirectionSchemaConfigApi,
    BalanceDirectionSchemaConfigResponse,
    BalanceDirectionTransformationRule as BalanceDirectionTransformationRuleApi,
    BalanceDirectionTransformationRuleResponse, CurrencySchemaConfig as CurrencySchemaConfigApi,
    CurrencySchemaConfigResponse, CurrencyTransformationRule as CurrencyTransformationRuleApi,
    CurrencyTransformationRuleResponse, DateTimeDuration as DateTimeDurationApi,
    DateTimePostParseTransformationRule as DateTimePostParseTransformationRuleApi,
    DateTimeTransformationRule as DateTimeTransformationRuleApi, DurationUnit as DurationUnitApi,
    EffectiveAtSchemaConfig as EffectiveAtSchemaConfigApi, EffectiveAtSchemaConfigResponse,
    EnumTransformationRule as EnumTransformationRuleApi, FieldType as FieldTypeApi,
    FieldTypeResponse, FieldsConfig as FieldsConfigApi, FieldsConfigResponse,
    FieldsConfigResponseV2, MainFieldsResponse,
    MajorUnitTransformationRule as MajorUnitTransformationRuleApi,
    MajorUnitValidationRule as MajorUnitValidationRuleApi,
    MetadataExposeConfig as MetadataExposeConfigApi,
    MetadataFieldSchemaConfig as MetadataFieldSchemaConfigApi, MetadataFieldSchemaConfigResponse,
    MetadataSchemaData as MetadataSchemaDataApi, MetadataSchemaDataResponse,
    MetadataSchemaDataResponseV2, MetadataSchemaResponseV2,
    MinorUnitTransformationRule as MinorUnitTransformationRuleApi,
    MinorUnitValidationRule as MinorUnitValidationRuleApi, MinorUnitValidationRuleResponse,
    NumberTransformationRule as NumberTransformationRuleApi,
    NumberValidationRule as NumberValidationRuleApi, NumberValidationRuleResponse,
    OrderIdSchemaConfig as OrderIdSchemaConfigApi, OrderIdSchemaConfigResponse,
    StringTransformationRule as StringTransformationRuleApi, StringTransformationRuleResponse,
    StringValidationRule as StringValidationRuleApi, StringValidationRuleResponse,
    TruncationPrecision as TruncationPrecisionApi, UniqueConstraint as UniqueConstraintApi,
    UniqueConstraintResponse, UniqueConstraintType as UniqueConstraintTypeApi,
    UniqueConstraintTypeResponse,
};
use common_types::metadata_schema::{
    datatypes::{
        balance_direction::{
            BalanceDirectionConfig, BalanceDirectionConfigV1, BalanceDirectionTransformationRule,
            BalanceDirectionTransformationRuleV1,
        },
        currency::{
            CurrencyConfig, CurrencyConfigV1, CurrencyTransformationRule,
            CurrencyTransformationRuleV1,
        },
        datetime::{
            DateFormatConfig, DateOrder, DateTimeConfig, DateTimeConfigV2, DateTimeDuration,
            DateTimeFormat, DateTimePostParseTransformationRule,
            DateTimePostParseTransformationRuleV1, DateTimeTransformationRule,
            DateTimeTransformationRuleV1, Delimiter, DurationUnit, TruncationPrecision,
        },
        enum_type::{EnumConfig, EnumConfigV1, EnumTransformationRule, EnumTransformationRuleV1},
        majorunit::{
            MajorUnitConfig, MajorUnitConfigV1, MajorUnitTransformationRule,
            MajorUnitTransformationRuleV1, MajorUnitValidationRule, MajorUnitValidationRuleV1,
        },
        minorunit::{
            MinorUnitConfig, MinorUnitConfigV1, MinorUnitTransformationRule,
            MinorUnitTransformationRuleV1, MinorUnitValidationRule, MinorUnitValidationRuleV1,
        },
        number::{
            NumberConfig, NumberConfigV1, NumberTransformationRule, NumberTransformationRuleV1,
            NumberValidationRule, NumberValidationRuleV1,
        },
        string::{
            StringConfig, StringConfigV1, StringTransformationRule, StringTransformationRuleV1,
            StringValidationRule, StringValidationRuleV1,
        },
    },
    fields::{
        amount_field::{AmountFieldConfig, AmountFieldConfigV2, AmountUnitConfig},
        balance_direction_field::{BalanceDirectionFieldConfig, BalanceDirectionFieldConfigV1},
        currency_field::{CurrencyFieldConfig, CurrencyFieldConfigV1},
        effective_at_field::{EffectiveAtFieldConfig, EffectiveAtFieldConfigV2},
        order_id_field::{OrderIdFieldConfig, OrderIdFieldConfigV1},
    },
    CompositeUniqueField, FieldTypeConfig, FieldTypeConfigV1, MetadataExposeConfig,
    MetadataSchemaData, ReconFieldsConfig, ReconFieldsConfigV1, ReconMetadataFieldConfig,
    ReconMetadataFieldConfigV1, ReconMetadataSchema, ReconMetadataSchemaV1, UniqueConstraint,
    UniqueConstraintType, UniqueConstraintTypeV1, UniqueConstraintV1,
};
use diesel_models::metadata_schema::MetadataSchema;

pub trait MetadataSchemaDataApiExt<T> {
    fn to_api_response(self) -> T;
}

pub trait MetadataSchemaDataDieselExt<T> {
    fn to_diesel_schema(self) -> T;
}

// -------------------------
// Common Types → API
// -------------------------

impl MetadataSchemaDataApiExt<MetadataSchemaDataResponse> for MetadataSchemaData {
    fn to_api_response(self) -> MetadataSchemaDataResponse {
        match self {
            Self::Recon(recon_schema) => match recon_schema {
                ReconMetadataSchema::V1(v1) => MetadataSchemaDataResponse::BasicSchemaData {
                    fields: v1.fields.to_api_response(),
                    unique_constraint: v1.unique_constraint.to_api_response(),
                    processing_mode: v1.processing_mode,
                },
            },
        }
    }
}

impl MetadataSchemaDataApiExt<FieldsConfigResponse> for ReconFieldsConfig {
    fn to_api_response(self) -> FieldsConfigResponse {
        match self {
            Self::V1(v1) => v1.to_api_response(),
        }
    }
}

impl MetadataSchemaDataApiExt<FieldsConfigResponse> for ReconFieldsConfigV1 {
    fn to_api_response(self) -> FieldsConfigResponse {
        FieldsConfigResponse {
            currency: self.currency.to_api_response(),
            amount: self.amount.to_api_response(),
            effective_at: self.effective_at.to_api_response(),
            balance_direction: self.balance_direction.to_api_response(),
            order_id: self.order_id.to_api_response(),
            metadata_fields: self
                .metadata_fields
                .into_iter()
                .map(|m| m.to_api_response())
                .collect(),
        }
    }
}

impl MetadataSchemaDataApiExt<CurrencySchemaConfigResponse> for CurrencyFieldConfig {
    fn to_api_response(self) -> CurrencySchemaConfigResponse {
        match self {
            Self::V1(v1) => CurrencySchemaConfigResponse {
                identifier: v1.identifier,
                transformation_rules: v1
                    .transformation_rules
                    .into_iter()
                    .map(|r| r.to_api_response())
                    .collect(),
            },
        }
    }
}

impl MetadataSchemaDataApiExt<CurrencyTransformationRuleResponse> for CurrencyTransformationRule {
    fn to_api_response(self) -> CurrencyTransformationRuleResponse {
        match self {
            Self::V1(CurrencyTransformationRuleV1::DefaultValue { currency }) => {
                CurrencyTransformationRuleResponse::DefaultValue { currency }
            }
            Self::V1(CurrencyTransformationRuleV1::Trim) => {
                CurrencyTransformationRuleResponse::Trim
            }
            Self::V1(CurrencyTransformationRuleV1::JsonExtract { pointer }) => {
                CurrencyTransformationRuleResponse::JsonExtract { pointer }
            }
        }
    }
}

impl MetadataSchemaDataDieselExt<CurrencyTransformationRule> for CurrencyTransformationRuleApi {
    fn to_diesel_schema(self) -> CurrencyTransformationRule {
        match self {
            Self::DefaultValue { currency } => {
                CurrencyTransformationRule::V1(CurrencyTransformationRuleV1::DefaultValue {
                    currency,
                })
            }
            Self::Trim => CurrencyTransformationRule::V1(CurrencyTransformationRuleV1::Trim),
            Self::JsonExtract { pointer } => {
                CurrencyTransformationRule::V1(CurrencyTransformationRuleV1::JsonExtract {
                    pointer,
                })
            }
        }
    }
}

impl MetadataSchemaDataApiExt<BalanceDirectionTransformationRuleResponse>
    for BalanceDirectionTransformationRule
{
    fn to_api_response(self) -> BalanceDirectionTransformationRuleResponse {
        match self {
            Self::V1(BalanceDirectionTransformationRuleV1::DefaultValue { direction }) => {
                BalanceDirectionTransformationRuleResponse::DefaultValue { direction }
            }
            Self::V1(BalanceDirectionTransformationRuleV1::Trim) => {
                BalanceDirectionTransformationRuleResponse::Trim
            }
            Self::V1(BalanceDirectionTransformationRuleV1::JsonExtract { pointer }) => {
                BalanceDirectionTransformationRuleResponse::JsonExtract { pointer }
            }
            Self::V1(BalanceDirectionTransformationRuleV1::StartsWith {
                prefix,
                then,
                otherwise,
            }) => BalanceDirectionTransformationRuleResponse::StartsWith {
                prefix,
                then,
                otherwise,
            },
        }
    }
}

impl MetadataSchemaDataDieselExt<BalanceDirectionTransformationRule>
    for BalanceDirectionTransformationRuleApi
{
    fn to_diesel_schema(self) -> BalanceDirectionTransformationRule {
        match self {
            Self::DefaultValue { direction } => BalanceDirectionTransformationRule::V1(
                BalanceDirectionTransformationRuleV1::DefaultValue { direction },
            ),
            Self::Trim => {
                BalanceDirectionTransformationRule::V1(BalanceDirectionTransformationRuleV1::Trim)
            }
            Self::JsonExtract { pointer } => BalanceDirectionTransformationRule::V1(
                BalanceDirectionTransformationRuleV1::JsonExtract { pointer },
            ),
            Self::StartsWith {
                prefix,
                then,
                otherwise,
            } => BalanceDirectionTransformationRule::V1(
                BalanceDirectionTransformationRuleV1::StartsWith {
                    prefix,
                    then,
                    otherwise,
                },
            ),
        }
    }
}

impl MetadataSchemaDataApiExt<NumberTransformationRuleApi> for NumberTransformationRule {
    fn to_api_response(self) -> NumberTransformationRuleApi {
        match self {
            Self::V1(NumberTransformationRuleV1::Trim) => NumberTransformationRuleApi::Trim,
            Self::V1(NumberTransformationRuleV1::JsonExtract { pointer }) => {
                NumberTransformationRuleApi::JsonExtract { pointer }
            }
        }
    }
}

impl MetadataSchemaDataDieselExt<NumberTransformationRule> for NumberTransformationRuleApi {
    fn to_diesel_schema(self) -> NumberTransformationRule {
        match self {
            Self::Trim => NumberTransformationRule::V1(NumberTransformationRuleV1::Trim),
            Self::JsonExtract { pointer } => {
                NumberTransformationRule::V1(NumberTransformationRuleV1::JsonExtract { pointer })
            }
        }
    }
}

impl MetadataSchemaDataApiExt<MinorUnitTransformationRuleApi> for MinorUnitTransformationRule {
    fn to_api_response(self) -> MinorUnitTransformationRuleApi {
        match self {
            Self::V1(MinorUnitTransformationRuleV1::Trim) => MinorUnitTransformationRuleApi::Trim,
            Self::V1(MinorUnitTransformationRuleV1::JsonExtract { pointer }) => {
                MinorUnitTransformationRuleApi::JsonExtract { pointer }
            }
            Self::V1(MinorUnitTransformationRuleV1::Absolute) => {
                MinorUnitTransformationRuleApi::Absolute
            }
        }
    }
}

impl MetadataSchemaDataDieselExt<MinorUnitTransformationRule> for MinorUnitTransformationRuleApi {
    fn to_diesel_schema(self) -> MinorUnitTransformationRule {
        match self {
            Self::Trim => MinorUnitTransformationRule::V1(MinorUnitTransformationRuleV1::Trim),
            Self::JsonExtract { pointer } => {
                MinorUnitTransformationRule::V1(MinorUnitTransformationRuleV1::JsonExtract {
                    pointer,
                })
            }
            Self::Absolute => {
                MinorUnitTransformationRule::V1(MinorUnitTransformationRuleV1::Absolute)
            }
        }
    }
}

impl MetadataSchemaDataApiExt<MajorUnitTransformationRuleApi> for MajorUnitTransformationRule {
    fn to_api_response(self) -> MajorUnitTransformationRuleApi {
        match self {
            Self::V1(MajorUnitTransformationRuleV1::Trim) => MajorUnitTransformationRuleApi::Trim,
            Self::V1(MajorUnitTransformationRuleV1::JsonExtract { pointer }) => {
                MajorUnitTransformationRuleApi::JsonExtract { pointer }
            }
            Self::V1(MajorUnitTransformationRuleV1::Negate) => {
                MajorUnitTransformationRuleApi::Negate
            }
            Self::V1(MajorUnitTransformationRuleV1::Absolute) => {
                MajorUnitTransformationRuleApi::Absolute
            }
            Self::V1(MajorUnitTransformationRuleV1::ReplaceChar { from, to, mode }) => {
                MajorUnitTransformationRuleApi::ReplaceChar { from, to, mode }
            }
        }
    }
}

impl MetadataSchemaDataDieselExt<MajorUnitTransformationRule> for MajorUnitTransformationRuleApi {
    fn to_diesel_schema(self) -> MajorUnitTransformationRule {
        match self {
            Self::Trim => MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::Trim),
            Self::JsonExtract { pointer } => {
                MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::JsonExtract {
                    pointer,
                })
            }
            Self::Negate => MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::Negate),
            Self::Absolute => {
                MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::Absolute)
            }
            Self::ReplaceChar { from, to, mode } => {
                MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::ReplaceChar {
                    from,
                    to,
                    mode,
                })
            }
        }
    }
}

impl MetadataSchemaDataApiExt<DateTimeTransformationRuleApi> for DateTimeTransformationRule {
    fn to_api_response(self) -> DateTimeTransformationRuleApi {
        match self {
            Self::V1(DateTimeTransformationRuleV1::Trim) => DateTimeTransformationRuleApi::Trim,
            Self::V1(DateTimeTransformationRuleV1::JsonExtract { pointer }) => {
                DateTimeTransformationRuleApi::JsonExtract { pointer }
            }
            Self::V1(DateTimeTransformationRuleV1::Regex { pattern, group }) => {
                DateTimeTransformationRuleApi::Regex { pattern, group }
            }
        }
    }
}

impl MetadataSchemaDataDieselExt<DateTimeTransformationRule> for DateTimeTransformationRuleApi {
    fn to_diesel_schema(self) -> DateTimeTransformationRule {
        match self {
            Self::Trim => DateTimeTransformationRule::V1(DateTimeTransformationRuleV1::Trim),
            Self::JsonExtract { pointer } => {
                DateTimeTransformationRule::V1(DateTimeTransformationRuleV1::JsonExtract {
                    pointer,
                })
            }
            Self::Regex { pattern, group } => {
                DateTimeTransformationRule::V1(DateTimeTransformationRuleV1::Regex {
                    pattern,
                    group,
                })
            }
        }
    }
}

impl MetadataSchemaDataApiExt<DateTimePostParseTransformationRuleApi>
    for DateTimePostParseTransformationRule
{
    fn to_api_response(self) -> DateTimePostParseTransformationRuleApi {
        match self {
            Self::V1(DateTimePostParseTransformationRuleV1::Truncate { precision }) => {
                DateTimePostParseTransformationRuleApi::Truncate {
                    precision: precision.to_api_response(),
                }
            }
            Self::V1(DateTimePostParseTransformationRuleV1::AddDuration { duration }) => {
                DateTimePostParseTransformationRuleApi::AddDuration {
                    duration: duration.to_api_response(),
                }
            }
            Self::V1(DateTimePostParseTransformationRuleV1::SubtractDuration { duration }) => {
                DateTimePostParseTransformationRuleApi::SubtractDuration {
                    duration: duration.to_api_response(),
                }
            }
        }
    }
}

impl MetadataSchemaDataDieselExt<DateTimePostParseTransformationRule>
    for DateTimePostParseTransformationRuleApi
{
    fn to_diesel_schema(self) -> DateTimePostParseTransformationRule {
        match self {
            Self::Truncate { precision } => DateTimePostParseTransformationRule::V1(
                DateTimePostParseTransformationRuleV1::Truncate {
                    precision: precision.to_diesel_schema(),
                },
            ),
            Self::AddDuration { duration } => DateTimePostParseTransformationRule::V1(
                DateTimePostParseTransformationRuleV1::AddDuration {
                    duration: duration.to_diesel_schema(),
                },
            ),
            Self::SubtractDuration { duration } => DateTimePostParseTransformationRule::V1(
                DateTimePostParseTransformationRuleV1::SubtractDuration {
                    duration: duration.to_diesel_schema(),
                },
            ),
        }
    }
}

impl MetadataSchemaDataApiExt<TruncationPrecisionApi> for TruncationPrecision {
    fn to_api_response(self) -> TruncationPrecisionApi {
        match self {
            Self::StartOfHour => TruncationPrecisionApi::StartOfHour,
            Self::StartOfDay => TruncationPrecisionApi::StartOfDay,
            Self::StartOfMonth => TruncationPrecisionApi::StartOfMonth,
            Self::StartOfYear => TruncationPrecisionApi::StartOfYear,
        }
    }
}

impl MetadataSchemaDataDieselExt<TruncationPrecision> for TruncationPrecisionApi {
    fn to_diesel_schema(self) -> TruncationPrecision {
        match self {
            Self::StartOfHour => TruncationPrecision::StartOfHour,
            Self::StartOfDay => TruncationPrecision::StartOfDay,
            Self::StartOfMonth => TruncationPrecision::StartOfMonth,
            Self::StartOfYear => TruncationPrecision::StartOfYear,
        }
    }
}

impl MetadataSchemaDataApiExt<DateTimeDurationApi> for DateTimeDuration {
    fn to_api_response(self) -> DateTimeDurationApi {
        DateTimeDurationApi {
            value: self.value,
            unit: self.unit.to_api_response(),
        }
    }
}

impl MetadataSchemaDataDieselExt<DateTimeDuration> for DateTimeDurationApi {
    fn to_diesel_schema(self) -> DateTimeDuration {
        DateTimeDuration {
            value: self.value,
            unit: self.unit.to_diesel_schema(),
        }
    }
}

impl MetadataSchemaDataApiExt<DurationUnitApi> for DurationUnit {
    fn to_api_response(self) -> DurationUnitApi {
        match self {
            Self::Minutes => DurationUnitApi::Minutes,
            Self::Hours => DurationUnitApi::Hours,
            Self::Days => DurationUnitApi::Days,
        }
    }
}

impl MetadataSchemaDataDieselExt<DurationUnit> for DurationUnitApi {
    fn to_diesel_schema(self) -> DurationUnit {
        match self {
            Self::Minutes => DurationUnit::Minutes,
            Self::Hours => DurationUnit::Hours,
            Self::Days => DurationUnit::Days,
        }
    }
}

impl MetadataSchemaDataApiExt<EnumTransformationRuleApi> for EnumTransformationRule {
    fn to_api_response(self) -> EnumTransformationRuleApi {
        match self {
            Self::V1(EnumTransformationRuleV1::Trim) => EnumTransformationRuleApi::Trim,
            Self::V1(EnumTransformationRuleV1::JsonExtract { pointer }) => {
                EnumTransformationRuleApi::JsonExtract { pointer }
            }
        }
    }
}

impl MetadataSchemaDataDieselExt<EnumTransformationRule> for EnumTransformationRuleApi {
    fn to_diesel_schema(self) -> EnumTransformationRule {
        match self {
            Self::Trim => EnumTransformationRule::V1(EnumTransformationRuleV1::Trim),
            Self::JsonExtract { pointer } => {
                EnumTransformationRule::V1(EnumTransformationRuleV1::JsonExtract { pointer })
            }
        }
    }
}

impl MetadataSchemaDataApiExt<AmountSchemaConfigResponse> for AmountFieldConfig {
    fn to_api_response(self) -> AmountSchemaConfigResponse {
        match self {
            Self::V1(v1) => AmountSchemaConfigResponse {
                identifier: v1.identifier,
                unit_config: AmountUnitConfigApi::MinorUnit {
                    validation_rules: vec![MinorUnitValidationRuleApi::PositiveOnly],
                    transformation_rules: vec![],
                },
            },
            Self::V2(v2) => AmountSchemaConfigResponse {
                identifier: v2.identifier,
                unit_config: v2.unit_config.to_api_response(),
            },
        }
    }
}

impl MetadataSchemaDataApiExt<AmountUnitConfigApi> for AmountUnitConfig {
    fn to_api_response(self) -> AmountUnitConfigApi {
        match self {
            Self::MinorUnit(MinorUnitConfig::V1(v1)) => AmountUnitConfigApi::MinorUnit {
                validation_rules: v1
                    .validation_rules
                    .into_iter()
                    .map(|r| match r {
                        MinorUnitValidationRule::V1(rule_v1) => match rule_v1 {
                            MinorUnitValidationRuleV1::PositiveOnly => {
                                MinorUnitValidationRuleApi::PositiveOnly
                            }
                            MinorUnitValidationRuleV1::MinValue { value } => {
                                MinorUnitValidationRuleApi::MinValue { value }
                            }
                            MinorUnitValidationRuleV1::MaxValue { value } => {
                                MinorUnitValidationRuleApi::MaxValue { value }
                            }
                        },
                    })
                    .collect(),
                transformation_rules: v1
                    .transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_api_response())
                    .collect(),
            },
            Self::MajorUnit(MajorUnitConfig::V1(v1)) => AmountUnitConfigApi::MajorUnit {
                delimiter: v1.delimiter,
                validation_rules: v1
                    .validation_rules
                    .into_iter()
                    .map(|r| match r {
                        MajorUnitValidationRule::V1(rule_v1) => match rule_v1 {
                            MajorUnitValidationRuleV1::PositiveOnly => {
                                MajorUnitValidationRuleApi::PositiveOnly
                            }
                            MajorUnitValidationRuleV1::MinValue { value } => {
                                MajorUnitValidationRuleApi::MinValue { value }
                            }
                            MajorUnitValidationRuleV1::MaxValue { value } => {
                                MajorUnitValidationRuleApi::MaxValue { value }
                            }
                        },
                    })
                    .collect(),
                transformation_rules: v1
                    .transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_api_response())
                    .collect(),
            },
        }
    }
}

impl MetadataSchemaDataApiExt<EffectiveAtSchemaConfigResponse> for EffectiveAtFieldConfig {
    fn to_api_response(self) -> EffectiveAtSchemaConfigResponse {
        match self {
            Self::V1(v1) => EffectiveAtSchemaConfigResponse {
                identifier: v1.identifier,
                transformation_rules: vec![],
                date_time_format: DateTimeFormat {
                    date_format: DateFormatConfig {
                        order: DateOrder::DayMonthYear,
                        delimiter: Delimiter::Colon,
                    },
                    time_format: None,
                },
                post_parse_transformations: vec![],
            },
            Self::V2(v2) => EffectiveAtSchemaConfigResponse {
                identifier: v2.identifier,
                date_time_format: v2.date_time_format,
                transformation_rules: v2
                    .transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_api_response())
                    .collect(),
                post_parse_transformations: v2
                    .post_parse_transformations
                    .into_iter()
                    .map(|rule| rule.to_api_response())
                    .collect(),
            },
        }
    }
}

impl MetadataSchemaDataApiExt<BalanceDirectionSchemaConfigResponse>
    for BalanceDirectionFieldConfig
{
    fn to_api_response(self) -> BalanceDirectionSchemaConfigResponse {
        match self {
            Self::V1(v1) => BalanceDirectionSchemaConfigResponse {
                identifier: v1.identifier,
                credit_values: v1.credit_values,
                debit_values: v1.debit_values,
                transformation_rules: v1
                    .transformation_rules
                    .into_iter()
                    .map(|r| r.to_api_response())
                    .collect(),
            },
        }
    }
}

impl MetadataSchemaDataApiExt<OrderIdSchemaConfigResponse> for OrderIdFieldConfig {
    fn to_api_response(self) -> OrderIdSchemaConfigResponse {
        match self {
            Self::V1(v1) => OrderIdSchemaConfigResponse {
                identifier: v1.identifier,
                transformation_rules: v1
                    .transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_api_response())
                    .collect(),
            },
        }
    }
}

impl MetadataSchemaDataApiExt<MetadataFieldSchemaConfigResponse> for ReconMetadataFieldConfig {
    fn to_api_response(self) -> MetadataFieldSchemaConfigResponse {
        match self {
            Self::V1(v1) => v1.to_api_response(),
        }
    }
}

impl MetadataSchemaDataApiExt<MetadataFieldSchemaConfigResponse> for ReconMetadataFieldConfigV1 {
    fn to_api_response(self) -> MetadataFieldSchemaConfigResponse {
        MetadataFieldSchemaConfigResponse {
            identifier: self.identifier,
            field_name: self.field_name,
            field_config: self.field_config.to_api_response(),
            description: self.description,
            required: self.required,
            is_exposed: self
                .is_exposed
                .map(|expose_config| MetadataExposeConfigApi {
                    is_searchable: expose_config.is_searchable,
                }),
        }
    }
}

impl MetadataSchemaDataApiExt<FieldTypeResponse> for FieldTypeConfig {
    fn to_api_response(self) -> FieldTypeResponse {
        match self {
            Self::V1(v1) => v1.to_api_response(),
        }
    }
}

impl MetadataSchemaDataApiExt<FieldTypeResponse> for FieldTypeConfigV1 {
    fn to_api_response(self) -> FieldTypeResponse {
        match self {
            Self::String(cfg) => match cfg {
                StringConfig::V1(v1) => FieldTypeResponse::String {
                    validation_rules: v1
                        .validation_rules
                        .into_iter()
                        .map(|r| match r {
                            StringValidationRule::V1(rule_v1) => match rule_v1 {
                                StringValidationRuleV1::MaxLength { value } => {
                                    StringValidationRuleResponse::MaxLength { value }
                                }
                                StringValidationRuleV1::MinLength { value } => {
                                    StringValidationRuleResponse::MinLength { value }
                                }
                            },
                        })
                        .collect(),
                    transformation_rules: v1
                        .transformation_rules
                        .into_iter()
                        .map(|rule| rule.to_api_response())
                        .collect(),
                },
            },
            Self::Number(cfg) => match cfg {
                NumberConfig::V1(v1) => FieldTypeResponse::Number {
                    validation_rules: v1
                        .validation_rules
                        .into_iter()
                        .map(|r| match r {
                            NumberValidationRule::V1(rule_v1) => match rule_v1 {
                                NumberValidationRuleV1::MinValue { value } => {
                                    NumberValidationRuleResponse::MinValue { value }
                                }
                                NumberValidationRuleV1::MaxValue { value } => {
                                    NumberValidationRuleResponse::MaxValue { value }
                                }
                            },
                        })
                        .collect(),
                    transformation_rules: v1
                        .transformation_rules
                        .into_iter()
                        .map(|rule| rule.to_api_response())
                        .collect(),
                },
            },
            Self::Currency(cfg) => match cfg {
                CurrencyConfig::V1(v1) => FieldTypeResponse::Currency {
                    transformation_rules: v1
                        .transformation_rules
                        .into_iter()
                        .map(|r| r.to_api_response())
                        .collect(),
                },
            },
            Self::MinorUnit(cfg) => match cfg {
                MinorUnitConfig::V1(v1) => FieldTypeResponse::MinorUnit {
                    validation_rules: v1
                        .validation_rules
                        .into_iter()
                        .map(|r| match r {
                            MinorUnitValidationRule::V1(rule_v1) => match rule_v1 {
                                MinorUnitValidationRuleV1::PositiveOnly => {
                                    MinorUnitValidationRuleResponse::PositiveOnly
                                }
                                MinorUnitValidationRuleV1::MinValue { value } => {
                                    MinorUnitValidationRuleResponse::MinValue { value }
                                }
                                MinorUnitValidationRuleV1::MaxValue { value } => {
                                    MinorUnitValidationRuleResponse::MaxValue { value }
                                }
                            },
                        })
                        .collect(),
                    transformation_rules: v1
                        .transformation_rules
                        .into_iter()
                        .map(|rule| rule.to_api_response())
                        .collect(),
                },
            },
            Self::MajorUnit(cfg) => match cfg {
                MajorUnitConfig::V1(v1) => FieldTypeResponse::MajorUnit {
                    validation_rules: v1
                        .validation_rules
                        .into_iter()
                        .map(|r| match r {
                            MajorUnitValidationRule::V1(rule_v1) => match rule_v1 {
                                MajorUnitValidationRuleV1::PositiveOnly => {
                                    MajorUnitValidationRuleApi::PositiveOnly
                                }
                                MajorUnitValidationRuleV1::MinValue { value } => {
                                    MajorUnitValidationRuleApi::MinValue { value }
                                }
                                MajorUnitValidationRuleV1::MaxValue { value } => {
                                    MajorUnitValidationRuleApi::MaxValue { value }
                                }
                            },
                        })
                        .collect(),
                    delimiter: v1.delimiter,
                    transformation_rules: v1
                        .transformation_rules
                        .into_iter()
                        .map(|rule| rule.to_api_response())
                        .collect(),
                },
            },
            Self::DateTime(cfg) => match cfg {
                DateTimeConfig::V1(_) => FieldTypeResponse::DateTime {
                    date_time_format: None,
                    transformation_rules: vec![],
                    post_parse_transformations: vec![],
                },
                DateTimeConfig::V2(cfg) => FieldTypeResponse::DateTime {
                    date_time_format: Some(cfg.format),
                    transformation_rules: cfg
                        .transformation_rules
                        .into_iter()
                        .map(|rule| rule.to_api_response())
                        .collect(),
                    post_parse_transformations: cfg
                        .post_parse_transformations
                        .into_iter()
                        .map(|rule| rule.to_api_response())
                        .collect(),
                },
            },
            Self::BalanceDirection(cfg) => match cfg {
                BalanceDirectionConfig::V1(v1) => FieldTypeResponse::BalanceDirection {
                    credit_values: v1.credit_values,
                    debit_values: v1.debit_values,
                    transformation_rules: v1
                        .transformation_rules
                        .into_iter()
                        .map(|r| r.to_api_response())
                        .collect(),
                },
            },
            Self::Enum(EnumConfig::V1(EnumConfigV1 {
                mappings,
                transformation_rules,
            })) => FieldTypeResponse::Enum {
                mappings,
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_api_response())
                    .collect(),
            },
        }
    }
}

impl MetadataSchemaDataApiExt<UniqueConstraintResponse> for UniqueConstraint {
    fn to_api_response(self) -> UniqueConstraintResponse {
        match self {
            Self::V1(constraint_v1) => constraint_v1.to_api_response(),
        }
    }
}

impl MetadataSchemaDataApiExt<UniqueConstraintResponse> for UniqueConstraintV1 {
    fn to_api_response(self) -> UniqueConstraintResponse {
        UniqueConstraintResponse {
            constraint_type: self.constraint_type.to_api_response(),
            description: self.description,
        }
    }
}

impl MetadataSchemaDataApiExt<UniqueConstraintTypeResponse> for UniqueConstraintType {
    fn to_api_response(self) -> UniqueConstraintTypeResponse {
        match self {
            Self::V1(v1) => match v1 {
                UniqueConstraintTypeV1::SingleField { field_name } => {
                    UniqueConstraintTypeResponse::SingleField { field_name }
                }
                UniqueConstraintTypeV1::CompositeField(CompositeUniqueField {
                    field_names,
                    delimiter,
                }) => UniqueConstraintTypeResponse::CompositeField {
                    field_names,
                    delimiter,
                },
            },
        }
    }
}

impl MetadataSchemaDataApiExt<MetadataSchemaResponseV2> for MetadataSchema {
    fn to_api_response(self) -> MetadataSchemaResponseV2 {
        MetadataSchemaResponseV2 {
            id: self.id,
            schema_id: self.schema_id,
            profile_id: self.profile_id,
            account_id: self.account_id,
            schema_data: self.schema_data.to_api_response(),
            version: self.version,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}

impl MetadataSchemaDataApiExt<MetadataSchemaDataResponseV2> for MetadataSchemaData {
    fn to_api_response(self) -> MetadataSchemaDataResponseV2 {
        match self {
            Self::Recon(recon_schema) => match recon_schema {
                ReconMetadataSchema::V1(v1) => MetadataSchemaDataResponseV2::BasicSchemaData {
                    fields: v1.fields.to_api_response(),
                    unique_constraint: v1.unique_constraint.to_api_response(),
                    processing_mode: v1.processing_mode,
                },
            },
        }
    }
}

impl MetadataSchemaDataApiExt<FieldsConfigResponseV2> for ReconFieldsConfig {
    fn to_api_response(self) -> FieldsConfigResponseV2 {
        match self {
            Self::V1(v1) => v1.to_api_response(),
        }
    }
}

impl MetadataSchemaDataApiExt<FieldsConfigResponseV2> for ReconFieldsConfigV1 {
    fn to_api_response(self) -> FieldsConfigResponseV2 {
        FieldsConfigResponseV2 {
            main_fields: vec![
                MainFieldsResponse::Currency(self.currency.to_api_response()),
                MainFieldsResponse::Amount(self.amount.to_api_response()),
                MainFieldsResponse::EffectiveAt(self.effective_at.to_api_response()),
                MainFieldsResponse::BalanceDirection(self.balance_direction.to_api_response()),
                MainFieldsResponse::OrderId(self.order_id.to_api_response()),
            ],
            metadata_fields: self
                .metadata_fields
                .into_iter()
                .map(|m| m.to_api_response())
                .collect(),
        }
    }
}

impl MetadataSchemaDataApiExt<StringTransformationRuleResponse> for StringTransformationRule {
    fn to_api_response(self) -> StringTransformationRuleResponse {
        match self {
            Self::V1(rule_v1) => match rule_v1 {
                StringTransformationRuleV1::DefaultValue { value } => {
                    StringTransformationRuleResponse::DefaultValue { value }
                }
                StringTransformationRuleV1::ToUpperCase => {
                    StringTransformationRuleResponse::ToUpperCase
                }
                StringTransformationRuleV1::ToLowerCase => {
                    StringTransformationRuleResponse::ToLowerCase
                }
                StringTransformationRuleV1::StripPrefix { prefix } => {
                    StringTransformationRuleResponse::StripPrefix { prefix }
                }
                StringTransformationRuleV1::StripSuffix { suffix } => {
                    StringTransformationRuleResponse::StripSuffix { suffix }
                }
                StringTransformationRuleV1::Trim => StringTransformationRuleResponse::Trim,
                StringTransformationRuleV1::JsonExtract { pointer } => {
                    StringTransformationRuleResponse::JsonExtract { pointer }
                }
                StringTransformationRuleV1::Regex { pattern, group } => {
                    StringTransformationRuleResponse::Regex { pattern, group }
                }
                StringTransformationRuleV1::ReplaceChar { from, to, mode } => {
                    StringTransformationRuleResponse::ReplaceChar { from, to, mode }
                }
            },
        }
    }
}

// -------------------------
// API → Common Types
// -------------------------

impl MetadataSchemaDataDieselExt<MetadataSchemaData> for MetadataSchemaDataApi {
    fn to_diesel_schema(self) -> MetadataSchemaData {
        match self {
            Self::BasicSchemaData {
                fields,
                unique_constraint,
                processing_mode,
            } => MetadataSchemaData::Recon(ReconMetadataSchema::V1(ReconMetadataSchemaV1 {
                fields: fields.to_diesel_schema(),
                unique_constraint: unique_constraint.to_diesel_schema(),
                processing_mode,
            })),
        }
    }
}

impl MetadataSchemaDataDieselExt<ReconFieldsConfig> for FieldsConfigApi {
    fn to_diesel_schema(self) -> ReconFieldsConfig {
        ReconFieldsConfig::V1(ReconFieldsConfigV1 {
            currency: self.currency.to_diesel_schema(),
            amount: self.amount.to_diesel_schema(),
            effective_at: self.effective_at.to_diesel_schema(),
            balance_direction: self.balance_direction.to_diesel_schema(),
            order_id: self.order_id.to_diesel_schema(),
            metadata_fields: self
                .metadata_fields
                .into_iter()
                .map(|m| m.to_diesel_schema())
                .collect(),
        })
    }
}

impl MetadataSchemaDataDieselExt<CurrencyFieldConfig> for CurrencySchemaConfigApi {
    fn to_diesel_schema(self) -> CurrencyFieldConfig {
        CurrencyFieldConfig::V1(CurrencyFieldConfigV1 {
            identifier: self.identifier,
            transformation_rules: self
                .transformation_rules
                .into_iter()
                .map(|r| r.to_diesel_schema())
                .collect(),
        })
    }
}

impl MetadataSchemaDataDieselExt<AmountFieldConfig> for AmountSchemaConfigApi {
    fn to_diesel_schema(self) -> AmountFieldConfig {
        AmountFieldConfig::V2(AmountFieldConfigV2 {
            identifier: self.identifier,
            unit_config: self.unit_config.to_diesel_schema(),
        })
    }
}

impl MetadataSchemaDataDieselExt<AmountUnitConfig> for AmountUnitConfigApi {
    fn to_diesel_schema(self) -> AmountUnitConfig {
        match self {
            Self::MinorUnit {
                validation_rules,
                transformation_rules,
            } => AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
                validation_rules: validation_rules
                    .into_iter()
                    .map(|r| {
                        MinorUnitValidationRule::V1(match r {
                            MinorUnitValidationRuleApi::PositiveOnly => {
                                MinorUnitValidationRuleV1::PositiveOnly
                            }
                            MinorUnitValidationRuleApi::MinValue { value } => {
                                MinorUnitValidationRuleV1::MinValue { value }
                            }
                            MinorUnitValidationRuleApi::MaxValue { value } => {
                                MinorUnitValidationRuleV1::MaxValue { value }
                            }
                        })
                    })
                    .collect(),
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
            })),
            Self::MajorUnit {
                delimiter,
                validation_rules,
                transformation_rules,
            } => AmountUnitConfig::MajorUnit(MajorUnitConfig::V1(MajorUnitConfigV1 {
                delimiter,
                validation_rules: validation_rules
                    .into_iter()
                    .map(|r| {
                        MajorUnitValidationRule::V1(match r {
                            MajorUnitValidationRuleApi::PositiveOnly => {
                                MajorUnitValidationRuleV1::PositiveOnly
                            }
                            MajorUnitValidationRuleApi::MinValue { value } => {
                                MajorUnitValidationRuleV1::MinValue { value }
                            }
                            MajorUnitValidationRuleApi::MaxValue { value } => {
                                MajorUnitValidationRuleV1::MaxValue { value }
                            }
                        })
                    })
                    .collect(),
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
            })),
        }
    }
}

impl MetadataSchemaDataDieselExt<EffectiveAtFieldConfig> for EffectiveAtSchemaConfigApi {
    fn to_diesel_schema(self) -> EffectiveAtFieldConfig {
        EffectiveAtFieldConfig::V2(EffectiveAtFieldConfigV2 {
            identifier: self.identifier,
            date_time_format: self.date_time_format,
            transformation_rules: self
                .transformation_rules
                .into_iter()
                .map(|rule| rule.to_diesel_schema())
                .collect(),
            post_parse_transformations: self
                .post_parse_transformations
                .into_iter()
                .map(|rule| rule.to_diesel_schema())
                .collect(),
        })
    }
}

impl MetadataSchemaDataDieselExt<BalanceDirectionFieldConfig> for BalanceDirectionSchemaConfigApi {
    fn to_diesel_schema(self) -> BalanceDirectionFieldConfig {
        BalanceDirectionFieldConfig::V1(BalanceDirectionFieldConfigV1 {
            identifier: self.identifier,
            credit_values: self.credit_values,
            debit_values: self.debit_values,
            transformation_rules: self
                .transformation_rules
                .into_iter()
                .map(|r| r.to_diesel_schema())
                .collect(),
        })
    }
}

impl MetadataSchemaDataDieselExt<OrderIdFieldConfig> for OrderIdSchemaConfigApi {
    fn to_diesel_schema(self) -> OrderIdFieldConfig {
        OrderIdFieldConfig::V1(OrderIdFieldConfigV1 {
            identifier: self.identifier,
            transformation_rules: self
                .transformation_rules
                .into_iter()
                .map(|r| {
                    StringTransformationRule::V1(match r {
                        StringTransformationRuleApi::DefaultValue { value } => {
                            StringTransformationRuleV1::DefaultValue { value }
                        }
                        StringTransformationRuleApi::ToUpperCase => {
                            StringTransformationRuleV1::ToUpperCase
                        }
                        StringTransformationRuleApi::ToLowerCase => {
                            StringTransformationRuleV1::ToLowerCase
                        }
                        StringTransformationRuleApi::StripPrefix { prefix } => {
                            StringTransformationRuleV1::StripPrefix { prefix }
                        }
                        StringTransformationRuleApi::StripSuffix { suffix } => {
                            StringTransformationRuleV1::StripSuffix { suffix }
                        }
                        StringTransformationRuleApi::Trim => StringTransformationRuleV1::Trim,
                        StringTransformationRuleApi::JsonExtract { pointer } => {
                            StringTransformationRuleV1::JsonExtract { pointer }
                        }
                        StringTransformationRuleApi::Regex { pattern, group } => {
                            StringTransformationRuleV1::Regex { pattern, group }
                        }
                        StringTransformationRuleApi::ReplaceChar { from, to, mode } => {
                            StringTransformationRuleV1::ReplaceChar { from, to, mode }
                        }
                    })
                })
                .collect(),
        })
    }
}

impl MetadataSchemaDataDieselExt<ReconMetadataFieldConfig> for MetadataFieldSchemaConfigApi {
    fn to_diesel_schema(self) -> ReconMetadataFieldConfig {
        ReconMetadataFieldConfig::V1(ReconMetadataFieldConfigV1 {
            identifier: self.identifier,
            field_name: self.field_name,
            field_config: self.field_config.to_diesel_schema(),
            description: self.description,
            required: self.required,
            is_exposed: self.is_exposed.map(|expose_config| MetadataExposeConfig {
                is_searchable: expose_config.is_searchable,
            }),
        })
    }
}

impl MetadataSchemaDataDieselExt<FieldTypeConfig> for FieldTypeApi {
    fn to_diesel_schema(self) -> FieldTypeConfig {
        FieldTypeConfig::V1(match self {
            Self::String {
                validation_rules,
                transformation_rules,
            } => FieldTypeConfigV1::String(StringConfig::V1(StringConfigV1 {
                validation_rules: validation_rules
                    .into_iter()
                    .map(|r| {
                        StringValidationRule::V1(match r {
                            StringValidationRuleApi::MaxLength { value } => {
                                StringValidationRuleV1::MaxLength { value }
                            }
                            StringValidationRuleApi::MinLength { value } => {
                                StringValidationRuleV1::MinLength { value }
                            }
                        })
                    })
                    .collect(),
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|r| {
                        StringTransformationRule::V1(match r {
                            StringTransformationRuleApi::DefaultValue { value } => {
                                StringTransformationRuleV1::DefaultValue { value }
                            }
                            StringTransformationRuleApi::ToUpperCase => {
                                StringTransformationRuleV1::ToUpperCase
                            }
                            StringTransformationRuleApi::ToLowerCase => {
                                StringTransformationRuleV1::ToLowerCase
                            }
                            StringTransformationRuleApi::StripPrefix { prefix } => {
                                StringTransformationRuleV1::StripPrefix { prefix }
                            }
                            StringTransformationRuleApi::StripSuffix { suffix } => {
                                StringTransformationRuleV1::StripSuffix { suffix }
                            }
                            StringTransformationRuleApi::Trim => StringTransformationRuleV1::Trim,
                            StringTransformationRuleApi::JsonExtract { pointer } => {
                                StringTransformationRuleV1::JsonExtract { pointer }
                            }
                            StringTransformationRuleApi::Regex { pattern, group } => {
                                StringTransformationRuleV1::Regex { pattern, group }
                            }
                            StringTransformationRuleApi::ReplaceChar { from, to, mode } => {
                                StringTransformationRuleV1::ReplaceChar { from, to, mode }
                            }
                        })
                    })
                    .collect(),
            })),
            Self::Number {
                validation_rules,
                transformation_rules,
            } => FieldTypeConfigV1::Number(NumberConfig::V1(NumberConfigV1 {
                validation_rules: validation_rules
                    .into_iter()
                    .map(|r| {
                        NumberValidationRule::V1(match r {
                            NumberValidationRuleApi::MinValue { value } => {
                                NumberValidationRuleV1::MinValue { value }
                            }
                            NumberValidationRuleApi::MaxValue { value } => {
                                NumberValidationRuleV1::MaxValue { value }
                            }
                        })
                    })
                    .collect(),
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
            })),
            Self::Currency {
                transformation_rules,
            } => FieldTypeConfigV1::Currency(CurrencyConfig::V1(CurrencyConfigV1 {
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|r| r.to_diesel_schema())
                    .collect(),
            })),
            Self::MinorUnit {
                validation_rules,
                transformation_rules,
            } => FieldTypeConfigV1::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
                validation_rules: validation_rules
                    .into_iter()
                    .map(|r| {
                        MinorUnitValidationRule::V1(match r {
                            MinorUnitValidationRuleApi::PositiveOnly => {
                                MinorUnitValidationRuleV1::PositiveOnly
                            }
                            MinorUnitValidationRuleApi::MinValue { value } => {
                                MinorUnitValidationRuleV1::MinValue { value }
                            }
                            MinorUnitValidationRuleApi::MaxValue { value } => {
                                MinorUnitValidationRuleV1::MaxValue { value }
                            }
                        })
                    })
                    .collect(),
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
            })),
            Self::MajorUnit {
                validation_rules,
                delimiter,
                transformation_rules,
            } => FieldTypeConfigV1::MajorUnit(MajorUnitConfig::V1(MajorUnitConfigV1 {
                validation_rules: validation_rules
                    .into_iter()
                    .map(|r| {
                        MajorUnitValidationRule::V1(match r {
                            MajorUnitValidationRuleApi::PositiveOnly => {
                                MajorUnitValidationRuleV1::PositiveOnly
                            }
                            MajorUnitValidationRuleApi::MinValue { value } => {
                                MajorUnitValidationRuleV1::MinValue { value }
                            }
                            MajorUnitValidationRuleApi::MaxValue { value } => {
                                MajorUnitValidationRuleV1::MaxValue { value }
                            }
                        })
                    })
                    .collect(),
                delimiter,
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
            })),
            Self::DateTime {
                date_time_format,
                transformation_rules,
                post_parse_transformations,
            } => FieldTypeConfigV1::DateTime(DateTimeConfig::V2(DateTimeConfigV2 {
                format: date_time_format,
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
                post_parse_transformations: post_parse_transformations
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
            })),
            Self::BalanceDirection {
                credit_values,
                debit_values,
                transformation_rules,
            } => FieldTypeConfigV1::BalanceDirection(BalanceDirectionConfig::V1(
                BalanceDirectionConfigV1 {
                    credit_values,
                    debit_values,
                    transformation_rules: transformation_rules
                        .into_iter()
                        .map(|r| r.to_diesel_schema())
                        .collect(),
                },
            )),
            Self::Enum {
                mappings,
                transformation_rules,
            } => FieldTypeConfigV1::Enum(EnumConfig::V1(EnumConfigV1 {
                mappings,
                transformation_rules: transformation_rules
                    .into_iter()
                    .map(|rule| rule.to_diesel_schema())
                    .collect(),
            })),
        })
    }
}

impl MetadataSchemaDataDieselExt<UniqueConstraint> for UniqueConstraintApi {
    fn to_diesel_schema(self) -> UniqueConstraint {
        UniqueConstraint::V1(self.to_diesel_schema())
    }
}

impl MetadataSchemaDataDieselExt<UniqueConstraintV1> for UniqueConstraintApi {
    fn to_diesel_schema(self) -> UniqueConstraintV1 {
        UniqueConstraintV1 {
            constraint_type: self.constraint_type.to_diesel_schema(),
            description: self.description,
        }
    }
}

impl MetadataSchemaDataDieselExt<UniqueConstraintType> for UniqueConstraintTypeApi {
    fn to_diesel_schema(self) -> UniqueConstraintType {
        UniqueConstraintType::V1(match self {
            Self::SingleField { field_name } => UniqueConstraintTypeV1::SingleField { field_name },
            Self::CompositeField {
                field_names,
                delimiter,
            } => UniqueConstraintTypeV1::CompositeField(CompositeUniqueField {
                field_names,
                delimiter,
            }),
        })
    }
}

pub fn project_metadata(metadata: &serde_json::Value, keys: &[String]) -> serde_json::Value {
    let projected = keys
        .iter()
        .filter_map(|key| metadata.get(key).map(|value| (key.clone(), value.clone())))
        .collect::<serde_json::Map<String, serde_json::Value>>();
    serde_json::Value::Object(projected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_metadata_keeps_only_requested_present_keys() {
        let metadata = serde_json::json!({
            "psp_ref": "ABC123",
            "fee": 100,
            "internal_note": "secret",
        });
        let keys = vec![
            "psp_ref".to_string(),
            "fee".to_string(),
            "absent".to_string(),
        ];

        let projected = project_metadata(&metadata, &keys);

        assert_eq!(
            projected,
            serde_json::json!({ "psp_ref": "ABC123", "fee": 100 })
        );
    }

    #[test]
    fn project_metadata_empty_keys_yields_empty_object() {
        let metadata = serde_json::json!({ "a": 1 });
        assert_eq!(project_metadata(&metadata, &[]), serde_json::json!({}));
    }

    #[test]
    fn project_metadata_non_object_yields_empty_object() {
        let metadata = serde_json::json!("not an object");
        let keys = vec!["a".to_string()];
        assert_eq!(project_metadata(&metadata, &keys), serde_json::json!({}));
    }

    #[test]
    fn project_metadata_preserves_null_values() {
        let metadata = serde_json::json!({ "a": null, "b": 2 });
        let keys = vec!["a".to_string(), "b".to_string()];

        let projected = project_metadata(&metadata, &keys);

        assert_eq!(projected, serde_json::json!({ "a": null, "b": 2 }));
    }
}
