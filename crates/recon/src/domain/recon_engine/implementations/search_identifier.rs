use common_types::{
    composite_field::{build_composite_key, build_composite_value},
    field_path_key::FieldPathKey,
    recon_rule::{
        entry_field::{EntryField, ExtractFromEntryField},
        search_identifier::{
            CompositeSearchIdentifier, SearchIdentifier, SearchIdentifierKey, SearchIdentifierV1,
            SearchIdentifierV2, SingleSearchIdentifier,
        },
    },
};
use error_stack::report;

use crate::domain::recon_engine::{ProcessingError, ProcessingResult};

pub trait SearchIdentifierExt {
    fn extract_source_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String>;

    fn extract_target_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String>;

    fn get_target_field(&self) -> FieldPathKey;
}

fn extract_single(
    data: &impl ExtractFromEntryField,
    field: &EntryField,
) -> ProcessingResult<String> {
    data.extract_field(field)
        .ok_or_else(|| ProcessingError::MissingSearchIdentifierValue(field.clone()).into())
}

impl SearchIdentifierExt for SearchIdentifier {
    fn extract_source_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String> {
        match self {
            Self::V1(SearchIdentifierV1 { source_field, .. }) => extract_single(data, source_field),
            Self::V2(SearchIdentifierV2::Single(SingleSearchIdentifier { key })) => {
                extract_single(data, &key.source_field)
            }
            Self::V2(SearchIdentifierV2::Composite(composite_search_identifier)) => {
                composite_search_identifier
                    .get_composite_value(data, |search_key| &search_key.source_field)
            }
        }
    }

    fn extract_target_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String> {
        match self {
            Self::V1(SearchIdentifierV1 { target_field, .. }) => extract_single(data, target_field),
            Self::V2(SearchIdentifierV2::Single(SingleSearchIdentifier { key })) => {
                extract_single(data, &key.target_field)
            }
            Self::V2(SearchIdentifierV2::Composite(composite_search_identifier)) => {
                composite_search_identifier
                    .get_composite_value(data, |search_key| &search_key.target_field)
            }
        }
    }

    fn get_target_field(&self) -> FieldPathKey {
        let key = match self {
            Self::V1(SearchIdentifierV1 { target_field, .. }) => target_field.to_string(),
            Self::V2(SearchIdentifierV2::Single(SingleSearchIdentifier { key })) => {
                key.target_field.to_string()
            }
            Self::V2(SearchIdentifierV2::Composite(composite_search_identifier)) => {
                composite_search_identifier.get_composite_key(|search_key| &search_key.target_field)
            }
        };
        FieldPathKey::new(key)
    }
}

trait CompositeSearchIdentifierExt {
    fn get_sorted_keys(&self) -> Vec<SearchIdentifierKey>;

    fn get_composite_key<F>(&self, name_of: F) -> String
    where
        F: Fn(&SearchIdentifierKey) -> &EntryField;

    fn get_composite_value<F>(
        &self,
        data: &impl ExtractFromEntryField,
        name_of: F,
    ) -> ProcessingResult<String>
    where
        F: Fn(&SearchIdentifierKey) -> &EntryField;
}

impl CompositeSearchIdentifierExt for CompositeSearchIdentifier {
    fn get_sorted_keys(&self) -> Vec<SearchIdentifierKey> {
        let mut keys = self.keys.clone();
        keys.sort_by_key(|key| key.source_field.to_string());
        keys
    }

    fn get_composite_key<F>(&self, name_of: F) -> String
    where
        F: Fn(&SearchIdentifierKey) -> &EntryField,
    {
        let sorted_keys = self.get_sorted_keys();
        build_composite_key(&sorted_keys, &self.delimiter.to_string(), name_of)
    }

    fn get_composite_value<F>(
        &self,
        data: &impl ExtractFromEntryField,
        name_of: F,
    ) -> ProcessingResult<String>
    where
        F: Fn(&SearchIdentifierKey) -> &EntryField,
    {
        let sorted_keys = self.get_sorted_keys();
        build_composite_value(&sorted_keys, &self.delimiter.to_string(), data, name_of)
            .map_err(|f| report!(ProcessingError::MissingSearchIdentifierValue(f)))
    }
}
