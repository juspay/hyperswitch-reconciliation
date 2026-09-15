use core::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::{EntryField, ExtractFromEntryField};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CompositeDelimiter {
    Pipe,
    None,
}

impl fmt::Display for CompositeDelimiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pipe => write!(f, "|"),
            Self::None => write!(f, ""),
        }
    }
}

pub fn build_composite<I, S>(parts: I, separator: &str) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = String::new();
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            out.push_str(separator);
        }
        out.push_str(part.as_ref());
    }
    out
}

pub fn build_composite_key<T, F>(items: &[T], separator: &str, name_of: F) -> String
where
    F: Fn(&T) -> &EntryField,
{
    build_composite(items.iter().map(|t| name_of(t).to_string()), separator)
}

pub fn build_composite_value<T, F>(
    items: &[T],
    separator: &str,
    data: &impl ExtractFromEntryField,
    name_of: F,
) -> Result<String, EntryField>
where
    F: Fn(&T) -> &EntryField,
{
    let parts = items
        .iter()
        .map(|t| {
            let f = name_of(t);
            data.extract_field(f).ok_or_else(|| f.clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(build_composite(parts, separator))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::unimplemented)]
mod tests {
    use std::collections::HashMap;

    use common_enums::{BalanceDirection, Currency};
    use common_utils::crypto::EncryptableValue;
    use time::PrimitiveDateTime;

    use super::*;
    use crate::{
        recon_rule::{entry_field::ExtractAccountId, search_identifier::SearchIdentifierKey},
        units::MinorUnit,
    };

    fn metadata(name: &str) -> EntryField {
        EntryField::Metadata(name.to_string())
    }

    /// Minimal `ExtractFromEntryField` impl backed by a `HashMap` so tests can
    /// assert composite construction without dragging in `RawEntryData`.
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
            unimplemented!("not used in composite_key tests")
        }
        fn effective_at(&self) -> PrimitiveDateTime {
            unimplemented!("not used in composite_key tests")
        }
        fn order_id(&self) -> String {
            String::new()
        }
    }

    /// Sort by `source_field` so callers don't have to care about input order —
    /// matches the canonicalization the runtime applies in the Composite path.
    fn search_keys<I>(pairs: I) -> Vec<SearchIdentifierKey>
    where
        I: IntoIterator<Item = (EntryField, EntryField)>,
    {
        let mut v: Vec<SearchIdentifierKey> = pairs
            .into_iter()
            .map(|(s, t)| SearchIdentifierKey {
                source_field: s,
                target_field: t,
            })
            .collect();
        v.sort_by_key(|key| key.source_field.to_string());
        v
    }

    fn fields<I>(items: I) -> Vec<EntryField>
    where
        I: IntoIterator<Item = EntryField>,
    {
        let mut v: Vec<EntryField> = items.into_iter().collect();
        v.sort_by_key(|a| a.to_string());
        v
    }

    const PIPE: &str = "|";

    #[test]
    fn single_pair_matches_legacy_format() {
        // For a single-key V1 SearchIdentifier, the composite must be
        // byte-identical to the legacy single-key format so existing
        // search_expectation rows still match.
        let keys = search_keys([(EntryField::OrderId, metadata("transaction_id"))]);
        let row = FakeRow::new(&[(EntryField::OrderId, "abc")]);
        let value = build_composite_value(&keys, PIPE, &row, |k| &k.source_field).unwrap();
        assert_eq!(value, "abc");

        let target_key = build_composite_key(&keys, PIPE, |k| &k.target_field);
        assert_eq!(target_key, "metadata.transaction_id");
    }

    #[test]
    fn source_and_target_values_align_under_asymmetric_field_names() {
        let keys = search_keys([
            (metadata("b_src"), metadata("b_tgt")),
            (metadata("a_src"), metadata("a_tgt")),
        ]);
        let source_row = FakeRow::new(&[(metadata("a_src"), "Y"), (metadata("b_src"), "X")]);
        let target_row = FakeRow::new(&[(metadata("a_tgt"), "Y"), (metadata("b_tgt"), "X")]);

        let source_value =
            build_composite_value(&keys, PIPE, &source_row, |k| &k.source_field).unwrap();
        let target_value =
            build_composite_value(&keys, PIPE, &target_row, |k| &k.target_field).unwrap();

        assert_eq!(source_value, target_value);
    }

    #[test]
    fn custom_delimiter_is_used() {
        let keys = search_keys([
            (metadata("a"), metadata("a")),
            (metadata("b"), metadata("b")),
        ]);
        let row = FakeRow::new(&[(metadata("a"), "x"), (metadata("b"), "z")]);
        let value = build_composite_value(&keys, ",", &row, |k| &k.source_field).unwrap();
        assert_eq!(value, "x,z");
    }

    #[test]
    fn missing_field_returns_offending_entry_field() {
        let set = fields([metadata("present"), metadata("missing")]);
        let row = FakeRow::new(&[(metadata("present"), "v")]);
        let err = build_composite_value(&set, PIPE, &row, |f| f).unwrap_err();
        assert_eq!(err, metadata("missing"));
    }

    #[test]
    fn grouping_field_set_iterates_in_canonical_order() {
        let set = fields([metadata("b"), metadata("a")]);
        let row = FakeRow::new(&[(metadata("a"), "v_a"), (metadata("b"), "v_b")]);
        // Sorted by `EntryField::to_string()` — "metadata.a" before "metadata.b".
        let key = build_composite_key(&set, PIPE, |f| f);
        assert_eq!(key, "metadata.a|metadata.b");
        let value = build_composite_value(&set, PIPE, &row, |f| f).unwrap();
        assert_eq!(value, "v_a|v_b");
    }
}
