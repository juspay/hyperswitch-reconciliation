use std::collections::HashSet;

use common_utils::id_type::MerchantId;

use crate::{
    errors::staging_entry::{StagingEntryError, StagingEntryResult},
    routes::AppState,
    types::transformers::ForeignFrom,
};
use api_models::{
    staging_entry::{
        ApiListStagingEntriesFilters, ApiStagingEntryCoarseStatus, ApiStagingEntryListFiltersV2,
        ApiStagingEntryStatus, NeedsManualReviewData, PendingData as PendingDataApi,
        StagingEntryData as StagingEntryDataApi,
    },
    user::User,
};
use common_enums::DieselStagingEntryStatus;
use common_types::staging_entry::{
    status::{DomainStagingEntryStatus, StagingEntryManualReviewData},
    EditedData, ListStagingEntriesFilters, ManualReviewData, PendingData, PendingDataV1,
    ProcessingErrorType, ProcessingErrorTypeV1, StagingEntryData as DieselStagingEntryData,
    StagingEntryDataV2, StagingEntryListFiltersV2, VoidData,
};
use error_stack::ResultExt;
use external_services::user::interfaces::UserInterface;
use recon_domain_models::staging_entry::DomainStagingEntry;
use strum::IntoEnumIterator;

/// Trait for converting Diesel transaction data to API transaction data
pub trait StagingEntryDataExt {
    /// Convert a DieselStagingEntry model to a StagingEntryDataApi model
    fn to_api_response(self) -> StagingEntryDataApi;
}

/// Implementation of StagingEntryDataExt for DieselStagingEntry
impl StagingEntryDataExt for DieselStagingEntryData {
    fn to_api_response(self) -> StagingEntryDataApi {
        match self {
            Self::V2(v2_data) => v2_data.to_api_response(),
        }
    }
}

/// Implementation of StagingEntryDataExt for StagingEntryDataV2 directly
impl StagingEntryDataExt for StagingEntryDataV2 {
    fn to_api_response(self) -> StagingEntryDataApi {
        match self {
            Self::Pending(pending_data) => match pending_data {
                PendingData::V1(v1_data) => match v1_data {
                    PendingDataV1::New => StagingEntryDataApi::Pending(PendingDataApi::New),
                    PendingDataV1::Edited(edited_data) => {
                        StagingEntryDataApi::Pending(PendingDataApi::Edited {
                            reason: match edited_data {
                                EditedData::V1(v1_data) => v1_data.reason,
                            },
                        })
                    }
                },
            },
            Self::NeedsManualReview(manual_review) => match manual_review {
                ManualReviewData::V1(v1_data) => v1_data.error_type.to_api_response(),
            },
            Self::Processed => StagingEntryDataApi::Processed,
            Self::Void(void_data) => StagingEntryDataApi::Void {
                reason: match void_data {
                    VoidData::V1(v1_data) => v1_data.reason,
                },
            },
            Self::Archived => StagingEntryDataApi::Archived,
        }
    }
}

/// Implementation of StagingEntryDataExt for ProcessingErrorType
impl StagingEntryDataExt for ProcessingErrorType {
    fn to_api_response(self) -> StagingEntryDataApi {
        match self {
            Self::V1(error_v1) => match error_v1 {
                ProcessingErrorTypeV1::NoRulesFound => {
                    StagingEntryDataApi::NeedsManualReview(NeedsManualReviewData::NoRulesFound)
                }
                ProcessingErrorTypeV1::StagingEntryCurrencyMismatch {
                    staging_currency,
                    account_currency,
                } => StagingEntryDataApi::NeedsManualReview(
                    NeedsManualReviewData::StagingEntryCurrencyMismatch {
                        staging_currency,
                        account_currency,
                    },
                ),
                ProcessingErrorTypeV1::MissingSearchIdentifierValue { missing_field } => {
                    StagingEntryDataApi::NeedsManualReview(
                        NeedsManualReviewData::MissingSearchIdentifierValue { missing_field },
                    )
                }
                ProcessingErrorTypeV1::DuplicateEntry => {
                    StagingEntryDataApi::NeedsManualReview(NeedsManualReviewData::DuplicateEntry)
                }
                ProcessingErrorTypeV1::NoExpectationEntryFound => {
                    StagingEntryDataApi::NeedsManualReview(
                        NeedsManualReviewData::NoExpectationEntryFound,
                    )
                }
                ProcessingErrorTypeV1::MultipleExpectedEntriesFound { transaction_ids } => {
                    StagingEntryDataApi::NeedsManualReview(
                        NeedsManualReviewData::MultipleExpectedEntriesFound { transaction_ids },
                    )
                }
                ProcessingErrorTypeV1::MissingMatchField { missing_field } => {
                    StagingEntryDataApi::NeedsManualReview(
                        NeedsManualReviewData::MissingMatchField { missing_field },
                    )
                }
                ProcessingErrorTypeV1::MissingUniqueField { missing_field } => {
                    StagingEntryDataApi::NeedsManualReview(
                        NeedsManualReviewData::MissingUniqueField { missing_field },
                    )
                }
                ProcessingErrorTypeV1::MissingGroupingField { missing_field } => {
                    StagingEntryDataApi::NeedsManualReview(
                        NeedsManualReviewData::MissingGroupingField { missing_field },
                    )
                }
                ProcessingErrorTypeV1::InternalError => {
                    StagingEntryDataApi::NeedsManualReview(NeedsManualReviewData::InternalError)
                }
            },
        }
    }
}

pub trait DomainStagingEntryStatusExt {
    fn to_api_response(self) -> ApiStagingEntryStatus;
}

impl DomainStagingEntryStatusExt for DomainStagingEntryStatus {
    fn to_api_response(self) -> ApiStagingEntryStatus {
        match self {
            Self::Pending => ApiStagingEntryStatus::Pending,
            Self::NeedsManualReview(manual_review_data) => {
                ApiStagingEntryStatus::NeedsManualReview(Some(manual_review_data))
            }
            Self::Processed => ApiStagingEntryStatus::Processed,
            Self::Void => ApiStagingEntryStatus::Void,
            Self::Archived => ApiStagingEntryStatus::Archived,
        }
    }
}

pub trait ApiStagingEntryStatusExt {
    fn to_domain_statuses(self) -> Vec<DomainStagingEntryStatus>;
}

impl ApiStagingEntryStatusExt for ApiStagingEntryStatus {
    fn to_domain_statuses(self) -> Vec<DomainStagingEntryStatus> {
        match self {
            Self::Pending => vec![DomainStagingEntryStatus::Pending],
            Self::NeedsManualReview(None) => StagingEntryManualReviewData::iter()
                .map(DomainStagingEntryStatus::NeedsManualReview)
                .collect(),
            Self::NeedsManualReview(Some(sub_status)) => {
                vec![DomainStagingEntryStatus::NeedsManualReview(sub_status)]
            }
            Self::Processed => vec![DomainStagingEntryStatus::Processed],
            Self::Void => vec![DomainStagingEntryStatus::Void],
            Self::Archived => vec![DomainStagingEntryStatus::Archived],
        }
    }
}

fn to_diesel_statuses(
    statuses: Option<Vec<ApiStagingEntryStatus>>,
) -> Option<Vec<DieselStagingEntryStatus>> {
    statuses.map(|statuses| {
        statuses
            .into_iter()
            .flat_map(ApiStagingEntryStatus::to_domain_statuses)
            .map(DieselStagingEntryStatus::from)
            .collect()
    })
}

fn coarse_to_diesel_statuses(
    statuses: Option<Vec<ApiStagingEntryCoarseStatus>>,
) -> Option<Vec<DieselStagingEntryStatus>> {
    statuses.map(|statuses| {
        statuses
            .into_iter()
            .map(ApiStagingEntryCoarseStatus::to_detailed)
            .flat_map(ApiStagingEntryStatus::to_domain_statuses)
            .map(DieselStagingEntryStatus::from)
            .collect()
    })
}

impl ForeignFrom<ApiListStagingEntriesFilters> for ListStagingEntriesFilters {
    fn foreign_from(value: ApiListStagingEntriesFilters) -> Self {
        Self {
            status: coarse_to_diesel_statuses(value.status),
            entry_type: value.entry_type,
            account_id: value.account_id,
            transformation_history_id: value.transformation_history_id,
            currency: value.currency,
            time_range: value.time_range,
            staging_entry_id: value.staging_entry_id,
        }
    }
}

impl ForeignFrom<ApiStagingEntryListFiltersV2> for StagingEntryListFiltersV2 {
    fn foreign_from(value: ApiStagingEntryListFiltersV2) -> Self {
        let coarse = coarse_to_diesel_statuses(value.status);
        let detailed = to_diesel_statuses(value.detailed_status);

        Self {
            status: match (coarse, detailed) {
                (Some(coarse), Some(detailed)) => {
                    let set: HashSet<DieselStagingEntryStatus> =
                        coarse.into_iter().chain(detailed).collect();
                    Some(set.into_iter().collect())
                }
                (Some(statuses), None) | (None, Some(statuses)) => Some(statuses),
                (None, None) => None,
            },
            entry_type: value.entry_type,
            account_ids: value.account_ids,
            transformation_config_ids: value.transformation_config_ids,
            transformation_history_id: value.transformation_history_id,
            transformation_history_ids: value.transformation_history_ids,
            currency: value.currency,
            time_range: value.time_range,
            staging_entry_id: value.staging_entry_id,
            order_id: value.order_id,
        }
    }
}

/// Extension trait for staging entry data correction validation
pub trait StagingEntryDataValidationExt {
    /// Validates that correction is allowed for this staging entry data
    fn validate_editing_allowed(&self) -> StagingEntryResult<()>;
}

impl StagingEntryDataValidationExt for DieselStagingEntryData {
    fn validate_editing_allowed(&self) -> StagingEntryResult<()> {
        match self {
            Self::V2(v2_data) => v2_data.validate_editing_allowed(),
        }
    }
}

impl StagingEntryDataValidationExt for StagingEntryDataV2 {
    fn validate_editing_allowed(&self) -> StagingEntryResult<()> {
        match self {
            Self::NeedsManualReview(review_data) => match review_data {
                ManualReviewData::V1(v1_data) => match &v1_data.error_type {
                    ProcessingErrorType::V1(error) => match error {
                        ProcessingErrorTypeV1::NoRulesFound
                        | ProcessingErrorTypeV1::StagingEntryCurrencyMismatch { .. }
                        | ProcessingErrorTypeV1::MissingSearchIdentifierValue { .. }
                        | ProcessingErrorTypeV1::DuplicateEntry
                        | ProcessingErrorTypeV1::NoExpectationEntryFound
                        | ProcessingErrorTypeV1::MultipleExpectedEntriesFound { .. }
                        | ProcessingErrorTypeV1::MissingMatchField { .. }
                        | ProcessingErrorTypeV1::MissingUniqueField { .. }
                        | ProcessingErrorTypeV1::MissingGroupingField { .. }
                        | ProcessingErrorTypeV1::InternalError => Ok(()),
                    },
                },
            },
            Self::Pending(_) | Self::Processed | Self::Void(_) | Self::Archived => {
                let status: DomainStagingEntryStatus = self.into();
                Err(StagingEntryError::InvalidStatus {
                    details: format!(
                        "Correction only allowed for NeedsManualReview status, got: {status:?}"
                    ),
                }
                .into())
            }
        }
    }
}

/// Trait for staging entry operations
pub trait StagingEntryExt {
    /// Get user details for the modified_by field by fetching from user_client
    async fn get_modified_by(
        &self,
        state: &AppState,
        merchant_id: MerchantId,
        profile_id: String,
    ) -> StagingEntryResult<Option<User>>;
}

impl StagingEntryExt for DomainStagingEntry {
    async fn get_modified_by(
        &self,
        state: &AppState,
        merchant_id: MerchantId,
        profile_id: String,
    ) -> StagingEntryResult<Option<User>> {
        let Some(user_id) = self.modified_by.clone() else {
            return Ok(None);
        };
        let user_response = state
            .user_client
            .get_user_by_id(merchant_id, profile_id, user_id)
            .await
            .change_context(StagingEntryError::InternalServerError)?;
        Ok(Some(User::foreign_from(user_response)))
    }
}
