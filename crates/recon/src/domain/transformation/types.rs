use common_enums::{BalanceDirection, StagingEntryProcessingMode};
use common_types::{
    metadata_schema::MetadataSchemaData, transformation::TransformationConfigDataV2,
    units::MinorUnit,
};
use common_utils::id_type::MerchantId;
use diesel_models::staging_entry::StagingEntryNew;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::metadata_schema::{
    skip_config::SkipReason,
    types::{ParsingFailure, ValidatedEntryData},
};

#[derive(Debug, Clone)]
pub struct TransformationStagingEntryData {
    pub uuid: Uuid,
    pub source_record: SourceRecord,
    pub metadata: Option<serde_json::Value>,
    pub exposed_metadata: serde_json::Value,
    pub searchable_metadata: serde_json::Value,
    pub row_data: StagingEntryRowData,
}

#[derive(Debug, Clone)]
pub struct StagingEntryRowData {
    pub balance_direction: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub effective_at: time::PrimitiveDateTime,
    pub processing_mode: StagingEntryProcessingMode,
    pub order_id: String,
}

impl TransformationStagingEntryData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_record: SourceRecord,
        balance_direction: BalanceDirection,
        amount: MinorUnit,
        currency: common_enums::Currency,
        effective_at: time::PrimitiveDateTime,
        processing_mode: StagingEntryProcessingMode,
        order_id: String,
        metadata: Option<serde_json::Value>,
        exposed_metadata: serde_json::Value,
        searchable_metadata: serde_json::Value,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            source_record,
            row_data: StagingEntryRowData {
                balance_direction,
                amount,
                currency,
                effective_at,
                processing_mode,
                order_id,
            },
            metadata,
            exposed_metadata,
            searchable_metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransformationExecutionResult {
    pub staging_entries_new: Vec<StagingEntryNew>,
    pub rows: Vec<TransformationRowResult>,
}

impl FromIterator<(TransformationRowResult, Option<StagingEntryNew>)>
    for TransformationExecutionResult
{
    fn from_iter<I: IntoIterator<Item = (TransformationRowResult, Option<StagingEntryNew>)>>(
        iter: I,
    ) -> Self {
        let iter = iter.into_iter();
        let (row_count, _) = iter.size_hint();

        iter.fold(
            Self {
                staging_entries_new: Vec::with_capacity(row_count),
                rows: Vec::with_capacity(row_count),
            },
            |mut execution_result, (row, staging_entry)| {
                execution_result.rows.push(row);
                execution_result.staging_entries_new.extend(staging_entry);
                execution_result
            },
        )
    }
}

pub struct StagingEntryContext<'a> {
    pub account_id: &'a str,
    pub merchant_id: &'a MerchantId,
    pub profile_id: &'a str,
    pub transformation_id: &'a str,
    pub transformation_history_id: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransformationRowResult {
    pub source: SourceRecord,
    pub outcome: RowResultOutcome,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RowResultOutcome {
    Transformed {
        staging_entry_id: String,
    },
    Skipped {
        reason: SkipReason,
    },
    Failed {
        stage: RowFailureStage,
        errors: Vec<TransformationDiagnostic>,
    },
}

#[derive(Debug, Clone)]
pub enum StagedRow {
    Completed {
        result: TransformationRowResult,
    },
    AwaitingEncryption {
        entry_data: TransformationStagingEntryData,
    },
}

#[derive(Debug, Clone)]
pub enum TransformationOutput {
    Processed { rows: Vec<TransformationRowOutput> },
    Rejected { failure: ParsingFailure },
}

#[derive(Debug, Clone)]
pub struct TransformationRowOutput {
    pub source: SourceRecord,
    pub outcome: RowOutcome,
}

#[derive(Debug, Clone)]
pub enum RowOutcome {
    Success(ValidatedEntryData),
    Skipped {
        reason: SkipReason,
    },
    Failed {
        stage: RowFailureStage,
        errors: Vec<TransformationDiagnostic>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RowFailureStage {
    Parsing,
    Validation,
    Encryption,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum SourceRecord {
    FileRow { line_number: usize },
    ApiRecord { index: Option<usize> },
}

impl From<SourceRecord> for SourceLocation {
    fn from(source_record: SourceRecord) -> Self {
        match source_record {
            SourceRecord::FileRow { line_number } => Self::FileRow { line_number },
            SourceRecord::ApiRecord { index } => Self::ApiRecord { index },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "location_type", rename_all = "snake_case")]
pub enum SourceLocation {
    FileRow {
        line_number: usize,
    },
    FileCell {
        line_number: usize,
        column: usize,
    },
    FileRange {
        line_number: usize,
        start_column: usize,
        end_column: usize,
    },
    ApiRecord {
        index: Option<usize>,
    },
    Field {
        field_name: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct TransformationDiagnostic {
    pub location: SourceLocation,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct LocatedValue<T> {
    pub value: T,
    pub location: SourceLocation,
}

impl<T> LocatedValue<T> {
    pub fn map<U>(self, transform: impl FnOnce(T) -> U) -> LocatedValue<U> {
        LocatedValue {
            value: transform(self.value),
            location: self.location,
        }
    }
}

pub struct TransformationInput<'a> {
    pub file: &'a [u8],
    pub config: &'a TransformationConfigDataV2,
    pub schema: &'a MetadataSchemaData,
}
