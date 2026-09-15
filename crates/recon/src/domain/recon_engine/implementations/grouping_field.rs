use common_types::{
    composite_field::build_composite_value,
    recon_rule::{
        entry_field::{EntryField, ExtractFromEntryField},
        grouping_field::{
            CompositeGroupingField, GroupingField, GroupingFieldV1, SingleGroupingField,
        },
    },
};

use crate::domain::recon_engine::{ProcessingError, ProcessingResult};

pub trait GroupingFieldExt {
    fn extract_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String>;
}

impl GroupingFieldExt for GroupingField {
    fn extract_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String> {
        match self {
            Self::V1(GroupingFieldV1::Single(SingleGroupingField { field })) => data
                .extract_field(field)
                .ok_or_else(|| ProcessingError::MissingGroupingField(field.clone()).into()),
            Self::V1(GroupingFieldV1::Composite(composite_grouping_field)) => {
                composite_grouping_field.get_composite_value(data)
            }
        }
    }
}

trait CompositeGroupingFieldExt {
    fn get_sorted_fields(&self) -> Vec<EntryField>;
    fn get_composite_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String>;
}

impl CompositeGroupingFieldExt for CompositeGroupingField {
    fn get_sorted_fields(&self) -> Vec<EntryField> {
        let mut fields = self.fields.clone();
        fields.sort();
        fields
    }

    fn get_composite_value(&self, data: &impl ExtractFromEntryField) -> ProcessingResult<String> {
        let sorted_fields = self.get_sorted_fields();
        build_composite_value(&sorted_fields, &self.delimiter.to_string(), data, |f| f)
            .map_err(|f| ProcessingError::MissingGroupingField(f).into())
    }
}
