use std::collections::HashMap;

use super::types::{MappedRow, ParsedRow};
use crate::domain::transformation::types::{LocatedValue, SourceLocation};
use common_types::metadata_schema::{
    fields::{
        amount_field::{archived::AmountFieldConfigV1, AmountFieldConfig, AmountFieldConfigV2},
        balance_direction_field::{BalanceDirectionFieldConfig, BalanceDirectionFieldConfigV1},
        currency_field::{CurrencyFieldConfig, CurrencyFieldConfigV1},
        effective_at_field::{
            archived::EffectiveAtFieldConfigV1, EffectiveAtFieldConfig, EffectiveAtFieldConfigV2,
        },
        order_id_field::{OrderIdFieldConfig, OrderIdFieldConfigV1},
    },
    MetadataSchemaData, ReconFieldsConfig, ReconFieldsConfigV1, ReconMetadataFieldConfig,
    ReconMetadataFieldConfigV1, ReconMetadataSchema, ReconMetadataSchemaV1,
};

/// Maps exactly one parsed row. Collection traversal belongs to the transformation engine.
pub trait RowMapper {
    fn map_row(&self, row: ParsedRow) -> MappedRow;
}

impl RowMapper for MetadataSchemaData {
    fn map_row(&self, row: ParsedRow) -> MappedRow {
        match self {
            Self::Recon(schema) => schema.map_row(row),
        }
    }
}

impl RowMapper for ReconMetadataSchema {
    fn map_row(&self, row: ParsedRow) -> MappedRow {
        match self {
            Self::V1(schema) => schema.map_row(row),
        }
    }
}

impl RowMapper for ReconMetadataSchemaV1 {
    fn map_row(&self, row: ParsedRow) -> MappedRow {
        MappedRow {
            source: row.source,
            mapped_fields: self.fields.map_fields(&row.fields),
        }
    }
}

/// Extension trait for mapping functionality on ReconFieldsConfig
pub trait FieldsConfigMapper {
    fn map_fields(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> HashMap<String, LocatedValue<Option<String>>>;
}

impl FieldsConfigMapper for ReconFieldsConfig {
    fn map_fields(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> HashMap<String, LocatedValue<Option<String>>> {
        match self {
            Self::V1(fields_v1) => fields_v1.map_fields(raw_fields),
        }
    }
}

impl FieldsConfigMapper for ReconFieldsConfigV1 {
    fn map_fields(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> HashMap<String, LocatedValue<Option<String>>> {
        let mut mapped_fields = HashMap::new();

        // Map main fields
        let (key, value) = self.currency.map_field(raw_fields);
        mapped_fields.insert(key, value);

        let (key, value) = self.amount.map_field(raw_fields);
        mapped_fields.insert(key, value);

        let (key, value) = self.effective_at.map_field(raw_fields);
        mapped_fields.insert(key, value);

        let (key, value) = self.balance_direction.map_field(raw_fields);
        mapped_fields.insert(key, value);

        let (key, value) = self.order_id.map_field(raw_fields);
        mapped_fields.insert(key, value);

        // Map metadata fields
        for metadata_field in &self.metadata_fields {
            let (key, value) = metadata_field.map_field(raw_fields);
            mapped_fields.insert(key, value);
        }

        mapped_fields
    }
}

fn map_located_field(
    raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    identifier: &str,
) -> LocatedValue<Option<String>> {
    raw_fields
        .get(identifier)
        .cloned()
        .unwrap_or_else(|| LocatedValue {
            value: None,
            location: SourceLocation::Field {
                field_name: identifier.to_string(),
            },
        })
}

/// Extension trait that adds mapping functionality to individual FieldSchema
pub trait MetadataSchemaMapper {
    /// Maps a field from raw fields using this field schema
    /// Returns the identifier and value for this specific field
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>);
}

impl MetadataSchemaMapper for CurrencyFieldConfig {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        match self {
            Self::V1(config_v1) => config_v1.map_field(raw_fields),
        }
    }
}

impl MetadataSchemaMapper for CurrencyFieldConfigV1 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

impl MetadataSchemaMapper for AmountFieldConfig {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        match self {
            Self::V1(config_v1) => config_v1.map_field(raw_fields),
            Self::V2(config_v2) => config_v2.map_field(raw_fields),
        }
    }
}

impl MetadataSchemaMapper for AmountFieldConfigV1 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

impl MetadataSchemaMapper for AmountFieldConfigV2 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

impl MetadataSchemaMapper for EffectiveAtFieldConfig {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        match self {
            Self::V1(config_v1) => config_v1.map_field(raw_fields),
            Self::V2(config_v2) => config_v2.map_field(raw_fields),
        }
    }
}

impl MetadataSchemaMapper for EffectiveAtFieldConfigV1 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

impl MetadataSchemaMapper for EffectiveAtFieldConfigV2 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

impl MetadataSchemaMapper for BalanceDirectionFieldConfig {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        match self {
            Self::V1(config_v1) => config_v1.map_field(raw_fields),
        }
    }
}

impl MetadataSchemaMapper for BalanceDirectionFieldConfigV1 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

impl MetadataSchemaMapper for OrderIdFieldConfig {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        match self {
            Self::V1(config_v1) => config_v1.map_field(raw_fields),
        }
    }
}

impl MetadataSchemaMapper for OrderIdFieldConfigV1 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

impl MetadataSchemaMapper for ReconMetadataFieldConfig {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        match self {
            Self::V1(config_v1) => config_v1.map_field(raw_fields),
        }
    }
}

impl MetadataSchemaMapper for ReconMetadataFieldConfigV1 {
    fn map_field(
        &self,
        raw_fields: &HashMap<String, LocatedValue<Option<String>>>,
    ) -> (String, LocatedValue<Option<String>>) {
        let value = map_located_field(raw_fields, &self.identifier);
        (self.identifier.clone(), value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_preserves_parser_location() {
        let location = SourceLocation::FileCell {
            line_number: 4,
            column: 2,
        };
        let raw_fields = HashMap::from([(
            "amount".to_string(),
            LocatedValue {
                value: Some("100".to_string()),
                location: location.clone(),
            },
        )]);

        let mapped = map_located_field(&raw_fields, "amount");

        assert_eq!(mapped.value, Some("100".to_string()));
        assert_eq!(mapped.location, location);
    }

    #[test]
    fn missing_field_uses_field_location() {
        let mapped = map_located_field(&HashMap::new(), "missing_column");

        assert_eq!(mapped.value, None);
        assert_eq!(
            mapped.location,
            SourceLocation::Field {
                field_name: "missing_column".to_string(),
            }
        );
    }
}
