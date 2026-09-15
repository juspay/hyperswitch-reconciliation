use common_types::{
    composite_field::CompositeDelimiter,
    metadata_schema::{
        CompositeUniqueField, UniqueConstraint, UniqueConstraintType, UniqueConstraintTypeV1,
        UniqueConstraintV1,
    },
    recon_rule::entry_field::{EntryField, ExtractFromEntryField},
};

pub trait UniqueConstraintExt {
    fn get_unique_fields(&self) -> Vec<EntryField>;

    fn get_unique_field_delimiter(&self) -> CompositeDelimiter;

    fn extract_unique_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> Vec<(EntryField, Option<String>)>;
}

impl UniqueConstraintExt for UniqueConstraint {
    fn get_unique_fields(&self) -> Vec<EntryField> {
        match self {
            Self::V1(constraint_v1) => constraint_v1.get_unique_fields(),
        }
    }

    fn get_unique_field_delimiter(&self) -> CompositeDelimiter {
        match self {
            Self::V1(constraint_v1) => constraint_v1.get_unique_field_delimiter(),
        }
    }

    fn extract_unique_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> Vec<(EntryField, Option<String>)> {
        match self {
            Self::V1(constraint_v1) => constraint_v1.extract_unique_value(data),
        }
    }
}

impl UniqueConstraintExt for UniqueConstraintV1 {
    fn get_unique_fields(&self) -> Vec<EntryField> {
        self.constraint_type.get_unique_fields()
    }

    fn get_unique_field_delimiter(&self) -> CompositeDelimiter {
        self.constraint_type.get_unique_field_delimiter()
    }

    fn extract_unique_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> Vec<(EntryField, Option<String>)> {
        self.constraint_type.extract_unique_value(data)
    }
}

impl UniqueConstraintExt for UniqueConstraintType {
    fn get_unique_fields(&self) -> Vec<EntryField> {
        match self {
            Self::V1(constraint_type_v1) => constraint_type_v1.get_unique_fields(),
        }
    }

    fn get_unique_field_delimiter(&self) -> CompositeDelimiter {
        match self {
            Self::V1(constraint_type_v1) => constraint_type_v1.get_unique_field_delimiter(),
        }
    }

    fn extract_unique_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> Vec<(EntryField, Option<String>)> {
        match self {
            Self::V1(constraint_type_v1) => constraint_type_v1.extract_unique_value(data),
        }
    }
}

impl UniqueConstraintExt for UniqueConstraintTypeV1 {
    fn get_unique_fields(&self) -> Vec<EntryField> {
        match self {
            Self::SingleField { field_name } => vec![field_name.clone()],
            Self::CompositeField(composite) => composite.get_sorted_fields(),
        }
    }

    fn get_unique_field_delimiter(&self) -> CompositeDelimiter {
        match self {
            Self::SingleField { .. } => CompositeDelimiter::None,
            Self::CompositeField(composite) => composite.delimiter,
        }
    }

    fn extract_unique_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> Vec<(EntryField, Option<String>)> {
        match self {
            Self::SingleField { field_name } => {
                vec![(field_name.clone(), data.extract_field(field_name))]
            }
            Self::CompositeField(composite) => composite
                .get_sorted_fields()
                .into_iter()
                .map(|field| {
                    let value = data.extract_field(&field);
                    (field, value)
                })
                .collect(),
        }
    }
}

trait CompositeUniqueFieldExt {
    fn get_sorted_fields(&self) -> Vec<EntryField>;
}

impl CompositeUniqueFieldExt for CompositeUniqueField {
    fn get_sorted_fields(&self) -> Vec<EntryField> {
        let mut fields = self.field_names.clone();
        fields.sort();
        fields
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::unimplemented)]
mod tests {
    use std::collections::HashMap;

    use common_enums::{BalanceDirection, Currency};
    use common_types::{recon_rule::entry_field::ExtractAccountId, units::MinorUnit};
    use common_utils::crypto::EncryptableValue;
    use time::PrimitiveDateTime;

    use super::*;

    struct FakeRow {
        fields: HashMap<EntryField, String>,
    }

    impl FakeRow {
        fn new(pairs: &[(EntryField, &str)]) -> Self {
            Self {
                fields: pairs
                    .iter()
                    .map(|(f, v)| (f.clone(), (*v).to_string()))
                    .collect(),
            }
        }
    }

    impl ExtractAccountId for FakeRow {
        fn extract_account_id(&self) -> String {
            String::new()
        }
    }

    impl ExtractFromEntryField for FakeRow {
        fn extract_field(&self, field: &EntryField) -> Option<String> {
            self.fields.get(field).cloned()
        }
        fn currency(&self) -> Currency {
            Currency::USD
        }
        fn entry_type(&self) -> BalanceDirection {
            BalanceDirection::Credit
        }
        fn amount(&self) -> MinorUnit {
            MinorUnit::new(0)
        }
        fn metadata(&self) -> &EncryptableValue {
            unimplemented!("not used in unique constraint tests")
        }
        fn effective_at(&self) -> PrimitiveDateTime {
            unimplemented!("not used in unique constraint tests")
        }
        fn order_id(&self) -> String {
            String::new()
        }
    }

    fn metadata(name: &str) -> EntryField {
        EntryField::Metadata(name.to_string())
    }

    #[test]
    fn single_field_key_and_value_are_unchanged() {
        let constraint = UniqueConstraintTypeV1::SingleField {
            field_name: metadata("customer_id"),
        };
        let row = FakeRow::new(&[(metadata("customer_id"), "cust_1")]);

        assert_eq!(
            constraint.get_unique_fields(),
            vec![metadata("customer_id")]
        );
        assert_eq!(
            constraint.extract_unique_value(&row),
            vec![(metadata("customer_id"), Some("cust_1".to_string()))]
        );
    }

    #[test]
    fn composite_key_and_values_are_in_sorted_order() {
        let constraint = UniqueConstraintTypeV1::CompositeField(CompositeUniqueField {
            field_names: vec![metadata("order_ref"), metadata("customer_id")],
            delimiter: CompositeDelimiter::Pipe,
        });
        let row = FakeRow::new(&[
            (metadata("customer_id"), "cust_1"),
            (metadata("order_ref"), "ord_9"),
        ]);

        assert_eq!(
            constraint.get_unique_fields(),
            vec![metadata("customer_id"), metadata("order_ref")]
        );
        assert_eq!(
            constraint.get_unique_field_delimiter(),
            CompositeDelimiter::Pipe
        );
        assert_eq!(
            constraint.extract_unique_value(&row),
            vec![
                (metadata("customer_id"), Some("cust_1".to_string())),
                (metadata("order_ref"), Some("ord_9".to_string())),
            ]
        );
    }

    #[test]
    fn composite_missing_field_comes_back_as_none() {
        let constraint = UniqueConstraintTypeV1::CompositeField(CompositeUniqueField {
            field_names: vec![metadata("customer_id"), metadata("order_ref")],
            delimiter: CompositeDelimiter::Pipe,
        });
        let row = FakeRow::new(&[(metadata("customer_id"), "cust_1")]);

        assert_eq!(
            constraint.extract_unique_value(&row),
            vec![
                (metadata("customer_id"), Some("cust_1".to_string())),
                (metadata("order_ref"), None),
            ]
        );
    }
}
