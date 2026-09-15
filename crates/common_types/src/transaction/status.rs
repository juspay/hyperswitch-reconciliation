use common_enums::DieselTransactionStatus;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", content = "sub_status", rename_all = "snake_case")]
pub enum DomainTransactionStatus {
    Expected,
    Posted(TransactionPostedData),
    OverAmount(TransactionAmountMismatchData),
    UnderAmount(TransactionAmountMismatchData),
    DataMismatch,
    CurrencyMismatch,
    SplitMismatch,
    Archived,
    Void,
    PartiallyReconciled,
    Matched(TransactionMatchedData),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionMatchedData {
    Auto,
    Manual,
    Force,
    WithTolerance,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionPostedData {
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionAmountMismatchData {
    Expected,
    Mismatch,
}

impl From<DomainTransactionStatus> for DieselTransactionStatus {
    fn from(value: DomainTransactionStatus) -> Self {
        match value {
            DomainTransactionStatus::Expected => Self::Expected,
            DomainTransactionStatus::Posted(posted_data) => match posted_data {
                TransactionPostedData::Manual => Self::PostedManual,
            },
            DomainTransactionStatus::OverAmount(mismatch_data) => match mismatch_data {
                TransactionAmountMismatchData::Expected => Self::OverAmountExpected,
                TransactionAmountMismatchData::Mismatch => Self::OverAmountMismatch,
            },
            DomainTransactionStatus::UnderAmount(mismatch_data) => match mismatch_data {
                TransactionAmountMismatchData::Expected => Self::UnderAmountExpected,
                TransactionAmountMismatchData::Mismatch => Self::UnderAmountMismatch,
            },
            DomainTransactionStatus::DataMismatch => Self::DataMismatch,
            DomainTransactionStatus::CurrencyMismatch => Self::CurrencyMismatch,
            DomainTransactionStatus::SplitMismatch => Self::SplitMismatch,
            DomainTransactionStatus::Archived => Self::Archived,
            DomainTransactionStatus::Void => Self::Void,
            DomainTransactionStatus::PartiallyReconciled => Self::PartiallyReconciled,
            DomainTransactionStatus::Matched(matched_data) => match matched_data {
                TransactionMatchedData::Auto => Self::MatchedAuto,
                TransactionMatchedData::Manual => Self::MatchedManual,
                TransactionMatchedData::Force => Self::MatchedForce,
                TransactionMatchedData::WithTolerance => Self::MatchedWithTolerance,
            },
        }
    }
}

impl From<DieselTransactionStatus> for DomainTransactionStatus {
    fn from(value: DieselTransactionStatus) -> Self {
        match value {
            DieselTransactionStatus::Expected => Self::Expected,
            DieselTransactionStatus::PostedManual => Self::Posted(TransactionPostedData::Manual),
            DieselTransactionStatus::OverAmountExpected => {
                Self::OverAmount(TransactionAmountMismatchData::Expected)
            }
            DieselTransactionStatus::OverAmountMismatch => {
                Self::OverAmount(TransactionAmountMismatchData::Mismatch)
            }
            DieselTransactionStatus::UnderAmountExpected => {
                Self::UnderAmount(TransactionAmountMismatchData::Expected)
            }
            DieselTransactionStatus::UnderAmountMismatch => {
                Self::UnderAmount(TransactionAmountMismatchData::Mismatch)
            }
            DieselTransactionStatus::DataMismatch => Self::DataMismatch,
            DieselTransactionStatus::CurrencyMismatch => Self::CurrencyMismatch,
            DieselTransactionStatus::SplitMismatch => Self::SplitMismatch,
            DieselTransactionStatus::Archived => Self::Archived,
            DieselTransactionStatus::Void => Self::Void,
            DieselTransactionStatus::PartiallyReconciled => Self::PartiallyReconciled,
            DieselTransactionStatus::MatchedAuto => Self::Matched(TransactionMatchedData::Auto),
            DieselTransactionStatus::MatchedManual => Self::Matched(TransactionMatchedData::Manual),
            DieselTransactionStatus::MatchedForce => Self::Matched(TransactionMatchedData::Force),
            DieselTransactionStatus::MatchedWithTolerance => {
                Self::Matched(TransactionMatchedData::WithTolerance)
            }
        }
    }
}
