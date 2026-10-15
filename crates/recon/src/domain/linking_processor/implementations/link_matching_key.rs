use common_types::{
    composite_field::CompositeDelimiter,
    field_path_key::FieldPathKey,
    link_config::link_matching_key::{
        CompositeLinkIdentifier, LinkIdentifierKey, LinkSearchIdentifier, LinkSearchIdentifierV1,
        SingleLinkIdentifier,
    },
    recon_rule::entry_field::{EntryField, ExtractFromEntryField},
};

use crate::domain::{
    linking_processor::{LinkProcessingResult, LinkingProcessingError},
    search_identifier::CompositeSearchIdentifierExt,
};

pub trait LinkSearchIdentifierExt {
    fn extract_transaction_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> LinkProcessingResult<String>;

    fn extract_confirmation_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> LinkProcessingResult<String>;

    fn get_transaction_field(&self) -> FieldPathKey;

    fn get_confirmation_field(&self) -> FieldPathKey;
}

fn extract_single(
    data: &impl ExtractFromEntryField,
    field: &EntryField,
) -> LinkProcessingResult<String> {
    data.extract_field(field)
        .ok_or_else(|| LinkingProcessingError::MissingSearchIdentifierValue(field.clone()).into())
}

impl LinkSearchIdentifierExt for LinkSearchIdentifier {
    fn extract_transaction_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> LinkProcessingResult<String> {
        match self {
            Self::V1(LinkSearchIdentifierV1::Single(SingleLinkIdentifier { key })) => {
                extract_single(data, &key.transaction_field)
            }
            Self::V1(LinkSearchIdentifierV1::Composite(composite_link_identifier)) => {
                composite_link_identifier.get_composite_value(
                    data,
                    |search_key| &search_key.transaction_field,
                    LinkingProcessingError::MissingSearchIdentifierValue,
                )
            }
        }
    }

    fn extract_confirmation_value(
        &self,
        data: &impl ExtractFromEntryField,
    ) -> LinkProcessingResult<String> {
        match self {
            Self::V1(LinkSearchIdentifierV1::Single(SingleLinkIdentifier { key })) => {
                extract_single(data, &key.confirmation_field)
            }
            Self::V1(LinkSearchIdentifierV1::Composite(composite_link_identifier)) => {
                composite_link_identifier.get_composite_value(
                    data,
                    |search_key| &search_key.confirmation_field,
                    LinkingProcessingError::MissingSearchIdentifierValue,
                )
            }
        }
    }

    fn get_transaction_field(&self) -> FieldPathKey {
        let key = match self {
            Self::V1(LinkSearchIdentifierV1::Single(SingleLinkIdentifier { key })) => {
                key.transaction_field.to_string()
            }
            Self::V1(LinkSearchIdentifierV1::Composite(composite_link_identifier)) => {
                composite_link_identifier
                    .get_composite_key(|search_key| &search_key.transaction_field)
            }
        };
        FieldPathKey::new(key)
    }

    fn get_confirmation_field(&self) -> FieldPathKey {
        let key = match self {
            Self::V1(LinkSearchIdentifierV1::Single(SingleLinkIdentifier { key })) => {
                key.confirmation_field.to_string()
            }
            Self::V1(LinkSearchIdentifierV1::Composite(composite_link_identifier)) => {
                composite_link_identifier
                    .get_composite_key(|search_key| &search_key.confirmation_field)
            }
        };
        FieldPathKey::new(key)
    }
}

impl CompositeSearchIdentifierExt for CompositeLinkIdentifier {
    type Key = LinkIdentifierKey;

    fn get_sorted_keys(&self) -> Vec<Self::Key> {
        let mut keys = self.keys.clone();
        keys.sort_by_key(|key| key.transaction_field.to_string());
        keys
    }

    fn get_delimiter(&self) -> CompositeDelimiter {
        self.delimiter
    }
}
