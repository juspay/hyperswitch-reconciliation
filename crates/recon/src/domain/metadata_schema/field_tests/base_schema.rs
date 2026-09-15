//! Base schema builder for tests
//!
//! Provides a builder pattern to construct metadata schemas with minimal boilerplate.
//! Tests can override only the parts they need.

use common_enums::StagingEntryProcessingMode;
use common_types::metadata_schema::{
    datatypes::{
        datetime::{DateFormatConfig, DateOrder, DateTimeFormat, Delimiter},
        minorunit::{MinorUnitConfig, MinorUnitConfigV1},
        string::{StringConfig, StringConfigV1},
    },
    fields::{
        amount_field::{AmountFieldConfig, AmountFieldConfigV2, AmountUnitConfig},
        balance_direction_field::{BalanceDirectionFieldConfig, BalanceDirectionFieldConfigV1},
        currency_field::{CurrencyFieldConfig, CurrencyFieldConfigV1},
        effective_at_field::{EffectiveAtFieldConfig, EffectiveAtFieldConfigV2},
        order_id_field::{OrderIdFieldConfig, OrderIdFieldConfigV1},
    },
    FieldTypeConfig, FieldTypeConfigV1, MetadataExposeConfig, MetadataSchemaData,
    ReconFieldsConfig, ReconFieldsConfigV1, ReconMetadataFieldConfig, ReconMetadataFieldConfigV1,
    ReconMetadataSchema, ReconMetadataSchemaV1, UniqueConstraint, UniqueConstraintType,
    UniqueConstraintTypeV1, UniqueConstraintV1,
};

pub struct BaseSchemaBuilder {
    // Main fields - store full configs
    amount: Option<AmountFieldConfig>,
    currency: Option<CurrencyFieldConfig>,
    balance_direction: Option<BalanceDirectionFieldConfig>,
    effective_at: Option<EffectiveAtFieldConfig>,
    order_id: Option<OrderIdFieldConfig>,

    // Metadata fields (start empty)
    metadata_fields: Vec<ReconMetadataFieldConfig>,

    // Optional overrides
    unique_constraint: Option<UniqueConstraint>,
    processing_mode: Option<StagingEntryProcessingMode>,
}

impl BaseSchemaBuilder {
    pub fn new() -> Self {
        Self {
            amount: None,
            currency: None,
            balance_direction: None,
            effective_at: None,
            order_id: None,
            metadata_fields: vec![],
            unique_constraint: None,
            processing_mode: None,
        }
    }

    /// Set all default main fields at once with sensible defaults
    pub fn with_default_main_fields(mut self) -> Self {
        self.amount = Some(AmountFieldConfig::V2(AmountFieldConfigV2 {
            identifier: "amount".to_string(),
            unit_config: AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
                validation_rules: vec![],
                transformation_rules: vec![],
            })),
        }));
        self.currency = Some(CurrencyFieldConfig::V1(CurrencyFieldConfigV1 {
            identifier: "currency".to_string(),
            transformation_rules: vec![],
        }));
        self.balance_direction = Some(BalanceDirectionFieldConfig::V1(
            BalanceDirectionFieldConfigV1 {
                identifier: "balance_direction".to_string(),
                credit_values: vec!["credit".to_string()],
                debit_values: vec!["debit".to_string()],
                transformation_rules: vec![],
            },
        ));
        self.effective_at = Some(EffectiveAtFieldConfig::V2(EffectiveAtFieldConfigV2 {
            identifier: "effective_at".to_string(),
            date_time_format: DateTimeFormat {
                date_format: DateFormatConfig {
                    order: DateOrder::DayMonthYear,
                    delimiter: Delimiter::Hyphen,
                },
                time_format: None,
            },
            transformation_rules: vec![],
            post_parse_transformations: vec![],
        }));
        self.order_id = Some(OrderIdFieldConfig::V1(OrderIdFieldConfigV1 {
            identifier: "order_id".to_string(),
            transformation_rules: vec![],
        }));
        self
    }

    /// Override amount field
    pub fn with_amount(mut self, config: AmountFieldConfig) -> Self {
        self.amount = Some(config);
        self
    }

    /// Override currency field
    pub fn with_currency(mut self, config: CurrencyFieldConfig) -> Self {
        self.currency = Some(config);
        self
    }

    /// Override balance_direction field
    pub fn with_balance_direction(mut self, config: BalanceDirectionFieldConfig) -> Self {
        self.balance_direction = Some(config);
        self
    }

    /// Override effective_at field
    pub fn with_effective_at(mut self, config: EffectiveAtFieldConfig) -> Self {
        self.effective_at = Some(config);
        self
    }

    /// Override order_id field
    pub fn with_order_id(mut self, config: OrderIdFieldConfig) -> Self {
        self.order_id = Some(config);
        self
    }

    /// Add a single metadata field
    pub fn add_metadata_field(
        mut self,
        field_name: &str,
        identifier: &str,
        field_config: FieldTypeConfig,
        required: bool,
    ) -> Self {
        self.metadata_fields
            .push(ReconMetadataFieldConfig::V1(ReconMetadataFieldConfigV1 {
                field_name: format!("metadata.{}", field_name).parse().unwrap(),
                identifier: identifier.to_string(),
                description: format!("{} field", field_name),
                required,
                field_config,
                is_exposed: None,
            }));
        self
    }

    /// Add a single metadata field with an explicit exposed configuration.
    pub fn add_exposed_metadata_field(
        mut self,
        field_name: &str,
        identifier: &str,
        field_config: FieldTypeConfig,
        required: bool,
        is_exposed: Option<MetadataExposeConfig>,
    ) -> Self {
        self.metadata_fields
            .push(ReconMetadataFieldConfig::V1(ReconMetadataFieldConfigV1 {
                field_name: format!("metadata.{}", field_name).parse().unwrap(),
                identifier: identifier.to_string(),
                description: format!("{} field", field_name),
                required,
                field_config,
                is_exposed,
            }));
        self
    }

    /// Set processing mode
    pub fn with_processing_mode(mut self, mode: StagingEntryProcessingMode) -> Self {
        self.processing_mode = Some(mode);
        self
    }

    /// Set unique constraint
    pub fn with_unique_constraint(mut self, constraint: UniqueConstraint) -> Self {
        self.unique_constraint = Some(constraint);
        self
    }

    /// Build the final MetadataSchemaData
    pub fn build(self) -> MetadataSchemaData {
        // Extract fields to avoid borrow after move
        let default_field_name = self
            .metadata_fields
            .first()
            .map(|f| match f {
                ReconMetadataFieldConfig::V1(config) => config.field_name.clone(),
            })
            .unwrap_or_else(|| "metadata.dummy".parse().unwrap());

        MetadataSchemaData::Recon(ReconMetadataSchema::V1(ReconMetadataSchemaV1 {
            fields: ReconFieldsConfig::V1(ReconFieldsConfigV1 {
                amount: self.amount.expect(
                    "amount field not set. Call with_default_main_fields() or with_amount()",
                ),
                currency: self.currency.expect(
                    "currency field not set. Call with_default_main_fields() or with_currency()",
                ),
                balance_direction: self.balance_direction.expect(
                    "balance_direction field not set. Call with_default_main_fields() or with_balance_direction()",
                ),
                effective_at: self.effective_at.expect(
                    "effective_at field not set. Call with_default_main_fields() or with_effective_at()",
                ),
                order_id: self.order_id.expect(
                    "order_id field not set. Call with_default_main_fields() or with_order_id()",
                ),
                metadata_fields: self.metadata_fields,
            }),
            unique_constraint: self.unique_constraint.unwrap_or_else(|| {
                UniqueConstraint::V1(UniqueConstraintV1 {
                    constraint_type: UniqueConstraintType::V1(
                        UniqueConstraintTypeV1::SingleField {
                            field_name: default_field_name,
                        },
                    ),
                    description: "Default unique constraint".to_string(),
                })
            }),
            processing_mode: self
                .processing_mode
                .unwrap_or(StagingEntryProcessingMode::Transaction),
        }))
    }
}

impl Default for BaseSchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod exposure_tests {
    use common_types::metadata_schema::ExposedKeysProvider;

    use super::*;

    fn string_field() -> FieldTypeConfig {
        FieldTypeConfig::V1(FieldTypeConfigV1::String(StringConfig::V1(
            StringConfigV1 {
                validation_rules: vec![],
                transformation_rules: vec![],
            },
        )))
    }

    #[test]
    fn exposed_keys_collects_exposed_and_searchable_subsets() {
        // field_name and identifier are intentionally DIFFERENT here. The validated
        // metadata is keyed by field_name, so exposed_keys() must return field names
        // (e.g. "psp_reference"), not identifiers (e.g. "psp_ref"). This guards against
        // the projection-by-wrong-key bug.
        let schema = BaseSchemaBuilder::new()
            .with_default_main_fields()
            .add_exposed_metadata_field(
                "psp_reference",
                "psp_ref",
                string_field(),
                true,
                Some(MetadataExposeConfig {
                    is_searchable: true,
                }),
            )
            .add_exposed_metadata_field(
                "settlement_fee",
                "fee",
                string_field(),
                false,
                Some(MetadataExposeConfig {
                    is_searchable: false,
                }),
            )
            .add_metadata_field("internal_note", "internal_note", string_field(), false)
            .build();

        let keys = schema.exposed_keys();

        let mut exposed = keys.exposed.clone();
        exposed.sort();
        assert_eq!(
            exposed,
            vec!["psp_reference".to_string(), "settlement_fee".to_string()]
        );
        assert_eq!(keys.searchable, vec!["psp_reference".to_string()]);
    }

    #[test]
    fn exposed_keys_empty_when_no_fields_exposed() {
        let schema = BaseSchemaBuilder::new()
            .with_default_main_fields()
            .add_metadata_field("plain", "plain", string_field(), false)
            .build();

        let keys = schema.exposed_keys();
        assert!(keys.exposed.is_empty());
        assert!(keys.searchable.is_empty());
    }
}
