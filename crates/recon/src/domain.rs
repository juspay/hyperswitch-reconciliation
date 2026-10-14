pub mod audit_trail;
pub mod entry;
pub mod exception_management;
pub mod ingestion;
pub mod metadata_schema;
pub mod parsing_engine;
pub mod recon_engine;
pub mod reporting;
pub mod staging_entry;
pub mod transaction;
pub mod transformation;

use common_types::{
    composite_field::{build_composite_key, build_composite_value, CompositeDelimiter},
    recon_rule::entry_field::{EntryField, ExtractFromEntryField},
};

trait CompositeSearchIdentifierExt {
    type Key;

    /// Return keys in the same canonical order for both sides of the identifier.
    fn get_sorted_keys(&self) -> Vec<Self::Key>;

    fn get_delimiter(&self) -> CompositeDelimiter;

    fn get_composite_key<F>(&self, name_of: F) -> String
    where
        F: Fn(&Self::Key) -> &EntryField,
    {
        let sorted_keys = self.get_sorted_keys();
        build_composite_key(&sorted_keys, &self.get_delimiter().to_string(), name_of)
    }

    fn get_composite_value<F, E>(
        &self,
        data: &impl ExtractFromEntryField,
        name_of: F,
        on_missing_field: impl FnOnce(EntryField) -> E,
    ) -> error_stack::Result<String, E>
    where
        F: Fn(&Self::Key) -> &EntryField,
        E: error_stack::Context,
    {
        let sorted_keys = self.get_sorted_keys();
        build_composite_value(
            &sorted_keys,
            &self.get_delimiter().to_string(),
            data,
            name_of,
        )
        .map_err(|field| error_stack::Report::new(on_missing_field(field)))
    }
}
