use api_models::entry::{
    AmountMismatchData as AmountMismatchDataApi,
    BalanceDirectionMismatchData as BalanceDirectionMismatchDataApi,
    CurrencyMismatchData as CurrencyMismatchDataApi, EntryData as EntryDataApi,
    FieldMismatchDetail as FieldMismatchDetailApi, MetadataMismatchData as MetadataMismatchDataApi,
    MismatchedEntryData,
};
use common_types::{
    amount::{AmountResponse, AmountType},
    entry::{
        BalanceDirectionMismatchData, BalanceDirectionMismatchDataV1, CurrencyMismatchData,
        CurrencyMismatchDataV1, EntryAmountMismatchData, EntryAmountMismatchDataV1, EntryData,
        EntryDataV2, EntryMismatchedData, EntryMismatchedDataV1, FieldMismatchDetail,
        FieldMismatchDetailV1, MetadataMismatchData, MetadataMismatchDataV1,
    },
    units::MinorUnit,
};
use error_stack::ResultExt;

use crate::errors::entry::{EntryError, EntryResult};

pub trait EntryDataExt {
    type ResultType;
    /// Convert a DieselStagingEntry model to a StagingEntryDataApi model
    fn to_api_response(self) -> EntryResult<Self::ResultType>;
}

pub trait EntryDataApiExt {
    type ResultType;
    fn to_diesel(self) -> EntryResult<Self::ResultType>;
}

impl EntryDataExt for Option<EntryData> {
    type ResultType = Option<EntryDataApi>;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Some(entry_data) => Ok(Some(entry_data.to_api_response()?)),
            None => Ok(None),
        }
    }
}

impl EntryDataExt for EntryData {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::V2(v2_data) => v2_data.to_api_response(),
        }
    }
}

impl EntryDataExt for EntryDataV2 {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::Pending => Ok(EntryDataApi::Pending),
            Self::Expected => Ok(EntryDataApi::Expected),
            Self::Posted => Ok(EntryDataApi::Posted),
            Self::Matched => Ok(EntryDataApi::Matched),
            Self::Archived => Ok(EntryDataApi::Archived),
            Self::Void => Ok(EntryDataApi::Void),
            Self::Mismatched(mismatched_data) => mismatched_data.to_api_response(),
        }
    }
}

impl EntryDataExt for EntryMismatchedData {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

impl EntryDataExt for EntryMismatchedDataV1 {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::AmountMismatch(data) => data.to_api_response(),
            Self::BalanceDirectionMismatch(data) => data.to_api_response(),
            Self::CurrencyMismatch(data) => data.to_api_response(),
            Self::MetadataMismatch(data) => data.to_api_response(),
        }
    }
}

impl EntryDataExt for EntryAmountMismatchData {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

impl EntryDataExt for EntryAmountMismatchDataV1 {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        let data = EntryDataApi::Mismatched {
            mismatch_type: MismatchedEntryData::AmountMismatch(AmountMismatchDataApi {
                expected_amount: AmountResponse::from_minor_unit(
                    self.expected_amount,
                    self.currency,
                )
                .change_context(EntryError::InternalServerError)?,
                actual_amount: AmountResponse::from_minor_unit(self.actual_amount, self.currency)
                    .change_context(EntryError::InternalServerError)?,
            }),
        };

        Ok(data)
    }
}

impl EntryDataExt for BalanceDirectionMismatchData {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

impl EntryDataExt for BalanceDirectionMismatchDataV1 {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        let data = EntryDataApi::Mismatched {
            mismatch_type: MismatchedEntryData::BalanceDirectionMismatch(
                BalanceDirectionMismatchDataApi {
                    expected_direction: self.expected_direction,
                    actual_direction: self.actual_direction,
                },
            ),
        };

        Ok(data)
    }
}

impl EntryDataExt for CurrencyMismatchData {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

impl EntryDataExt for CurrencyMismatchDataV1 {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        let data = EntryDataApi::Mismatched {
            mismatch_type: MismatchedEntryData::CurrencyMismatch(CurrencyMismatchDataApi {
                expected_currency: self.expected_currency,
                actual_currency: self.actual_currency,
            }),
        };

        Ok(data)
    }
}

impl EntryDataExt for MetadataMismatchData {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

pub trait FieldMismatchDetailExt {
    fn to_api_response(self) -> FieldMismatchDetailApi;
}

impl FieldMismatchDetailExt for FieldMismatchDetail {
    fn to_api_response(self) -> FieldMismatchDetailApi {
        match self {
            Self::V1(v1_field) => FieldMismatchDetailApi {
                field_name: v1_field.field_name,
                label: v1_field.label,
                expected_value: v1_field.expected_value,
                actual_value: v1_field.actual_value,
            },
        }
    }
}

impl EntryDataExt for MetadataMismatchDataV1 {
    type ResultType = EntryDataApi;
    fn to_api_response(self) -> EntryResult<Self::ResultType> {
        let mismatched_fields = self
            .mismatched_fields
            .into_iter()
            .map(FieldMismatchDetail::to_api_response)
            .collect();

        let data = EntryDataApi::Mismatched {
            mismatch_type: MismatchedEntryData::MetadataMismatch(MetadataMismatchDataApi {
                mismatched_fields,
            }),
        };
        Ok(data)
    }
}

impl EntryDataApiExt for EntryDataApi {
    type ResultType = EntryData;
    fn to_diesel(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::Pending => Ok(EntryData::V2(EntryDataV2::Pending)),
            Self::Expected => Ok(EntryData::V2(EntryDataV2::Expected)),
            Self::Posted => Ok(EntryData::V2(EntryDataV2::Posted)),
            Self::Matched => Ok(EntryData::V2(EntryDataV2::Matched)),
            Self::Archived => Ok(EntryData::V2(EntryDataV2::Archived)),
            Self::Void => Ok(EntryData::V2(EntryDataV2::Void)),
            Self::Mismatched { mismatch_type } => {
                let mismatched_data = mismatch_type.to_diesel()?;
                Ok(EntryData::V2(EntryDataV2::Mismatched(mismatched_data)))
            }
        }
    }
}

impl EntryDataApiExt for MismatchedEntryData {
    type ResultType = EntryMismatchedData;
    fn to_diesel(self) -> EntryResult<Self::ResultType> {
        match self {
            Self::AmountMismatch(amount_mismatch_data) => amount_mismatch_data.to_diesel(),
            Self::BalanceDirectionMismatch(balance_direction_mismatch_data) => {
                balance_direction_mismatch_data.to_diesel()
            }
            Self::CurrencyMismatch(currency_mismatch_data) => currency_mismatch_data.to_diesel(),
            Self::MetadataMismatch(metadata_mismatch_data) => metadata_mismatch_data.to_diesel(),
        }
    }
}

impl EntryDataApiExt for AmountMismatchDataApi {
    type ResultType = EntryMismatchedData;
    fn to_diesel(self) -> EntryResult<Self::ResultType> {
        // Convert AmountResponse to MinorUnit using AmountType
        let expected_amount_type =
            AmountType::from_major_unit(self.expected_amount.value, self.expected_amount.currency)
                .change_context(EntryError::InternalServerError)?;
        let actual_amount_type =
            AmountType::from_major_unit(self.actual_amount.value, self.actual_amount.currency)
                .change_context(EntryError::InternalServerError)?;

        let expected_amount_minor_unit = expected_amount_type.value();
        let actual_amount_minor_unit = actual_amount_type.value();

        // Calculate variance as absolute difference
        let variance = MinorUnit::new(
            (expected_amount_minor_unit.get_amount_as_i64()
                - actual_amount_minor_unit.get_amount_as_i64())
            .abs(),
        );

        Ok(EntryMismatchedData::V1(
            EntryMismatchedDataV1::AmountMismatch(EntryAmountMismatchData::V1(
                EntryAmountMismatchDataV1 {
                    expected_amount: expected_amount_minor_unit,
                    actual_amount: actual_amount_minor_unit,
                    currency: self.expected_amount.currency,
                    variance,
                },
            )),
        ))
    }
}

impl EntryDataApiExt for BalanceDirectionMismatchDataApi {
    type ResultType = EntryMismatchedData;
    fn to_diesel(self) -> EntryResult<Self::ResultType> {
        Ok(EntryMismatchedData::V1(
            EntryMismatchedDataV1::BalanceDirectionMismatch(BalanceDirectionMismatchData::V1(
                BalanceDirectionMismatchDataV1 {
                    expected_direction: self.expected_direction,
                    actual_direction: self.actual_direction,
                },
            )),
        ))
    }
}

impl EntryDataApiExt for CurrencyMismatchDataApi {
    type ResultType = EntryMismatchedData;
    fn to_diesel(self) -> EntryResult<Self::ResultType> {
        Ok(EntryMismatchedData::V1(
            EntryMismatchedDataV1::CurrencyMismatch(CurrencyMismatchData::V1(
                CurrencyMismatchDataV1 {
                    expected_currency: self.expected_currency,
                    actual_currency: self.actual_currency,
                },
            )),
        ))
    }
}

impl EntryDataApiExt for MetadataMismatchDataApi {
    type ResultType = EntryMismatchedData;
    fn to_diesel(self) -> EntryResult<Self::ResultType> {
        let mismatched_fields = self
            .mismatched_fields
            .into_iter()
            .map(|field| field.to_diesel())
            .collect::<EntryResult<Vec<_>>>()?;

        Ok(EntryMismatchedData::V1(
            EntryMismatchedDataV1::MetadataMismatch(MetadataMismatchData::V1(
                MetadataMismatchDataV1 { mismatched_fields },
            )),
        ))
    }
}

impl EntryDataApiExt for FieldMismatchDetailApi {
    type ResultType = FieldMismatchDetail;
    fn to_diesel(self) -> EntryResult<Self::ResultType> {
        Ok(FieldMismatchDetail::V1(FieldMismatchDetailV1 {
            field_name: self.field_name,
            label: self.label,
            expected_value: self.expected_value,
            actual_value: self.actual_value,
        }))
    }
}
