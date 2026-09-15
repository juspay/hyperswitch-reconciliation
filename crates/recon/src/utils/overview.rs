use std::collections::HashMap;

use api_models::overview::{
    AccountTransactionStatusAmount, StagingEntryDetailedStatusAmount, StagingEntryStatusAmount,
    TransactionStatusAmount,
};
use api_models::staging_entry::ApiStagingEntryCoarseStatus;
use api_models::transaction::ApiTransactionFilterStatus;
use common_enums::{Currency, DieselStagingEntryStatus, Granularity};
use common_types::{
    amount::AmountResponse,
    date_time::DefinedTimeRange,
    recon_rule::aging_config::{AgingConfig, AgingConfigV1},
    staging_entry::status::DomainStagingEntryStatus,
    transaction::status::DomainTransactionStatus,
    units::MinorUnit,
};
use common_utils::date_time;
use diesel_models::{
    staging_entry::StagingEntryOverviewRow,
    transaction::{RuleAccountTransactionOverviewRow, TransactionOverviewRow},
};
use error_stack::{report, Report, ResultExt};
use strum::IntoEnumIterator;
use time::{Date, Duration, PrimitiveDateTime, Time};

use crate::{
    errors::overview::{OverviewError, OverviewResult},
    types::transformers::ForeignTryFrom,
    utils::{
        staging_entry::DomainStagingEntryStatusExt, transaction::ApiTransactionFilterStatusExt,
    },
};

const MAX_BUCKETS: usize = 500;

pub trait GranularityExt {
    fn truncate_to_bucket(self, value: PrimitiveDateTime) -> OverviewResult<PrimitiveDateTime>;

    fn next_bucket_start(
        self,
        bucket_start: PrimitiveDateTime,
    ) -> OverviewResult<PrimitiveDateTime>;

    fn bucket_ranges(
        self,
        start_time: PrimitiveDateTime,
        end_time: PrimitiveDateTime,
        max_buckets: usize,
    ) -> OverviewResult<Vec<(PrimitiveDateTime, PrimitiveDateTime)>>;

    fn bucket_positions<I>(
        self,
        bucket_starts: I,
    ) -> OverviewResult<HashMap<PrimitiveDateTime, usize>>
    where
        I: IntoIterator<Item = PrimitiveDateTime>;
}

impl GranularityExt for Granularity {
    fn truncate_to_bucket(self, value: PrimitiveDateTime) -> OverviewResult<PrimitiveDateTime> {
        match self {
            Self::Hour => Ok(value.replace_time(
                Time::from_hms(value.hour(), 0, 0)
                    .map_err(|_| report!(OverviewError::InternalServerError))?,
            )),
            Self::Day => Ok(value.replace_time(Time::MIDNIGHT)),
            Self::Week => {
                let days_since_monday = i64::from(value.weekday().number_days_from_monday());
                let date = value
                    .date()
                    .checked_sub(Duration::days(days_since_monday))
                    .ok_or_else(|| report!(OverviewError::InternalServerError))?;
                Ok(PrimitiveDateTime::new(date, Time::MIDNIGHT))
            }
            Self::Month => {
                let date = Date::from_calendar_date(value.year(), value.month(), 1)
                    .map_err(|_| report!(OverviewError::InternalServerError))?;
                Ok(PrimitiveDateTime::new(date, Time::MIDNIGHT))
            }
        }
    }

    fn next_bucket_start(
        self,
        bucket_start: PrimitiveDateTime,
    ) -> OverviewResult<PrimitiveDateTime> {
        match self {
            Self::Hour => bucket_start
                .checked_add(Duration::HOUR)
                .ok_or_else(|| report!(OverviewError::InternalServerError)),
            Self::Day => bucket_start
                .checked_add(Duration::DAY)
                .ok_or_else(|| report!(OverviewError::InternalServerError)),
            Self::Week => bucket_start
                .checked_add(Duration::WEEK)
                .ok_or_else(|| report!(OverviewError::InternalServerError)),
            Self::Month => {
                let date = bucket_start.date();
                let (year, month) = if date.month() == time::Month::December {
                    (
                        date.year()
                            .checked_add(1)
                            .ok_or_else(|| report!(OverviewError::InternalServerError))?,
                        time::Month::January,
                    )
                } else {
                    (date.year(), date.month().next())
                };
                let next_date = Date::from_calendar_date(year, month, 1)
                    .map_err(|_| report!(OverviewError::InternalServerError))?;
                Ok(PrimitiveDateTime::new(next_date, Time::MIDNIGHT))
            }
        }
    }

    fn bucket_ranges(
        self,
        start_time: PrimitiveDateTime,
        end_time: PrimitiveDateTime,
        max_buckets: usize,
    ) -> OverviewResult<Vec<(PrimitiveDateTime, PrimitiveDateTime)>> {
        let mut ranges = Vec::new();
        let mut bucket_start = start_time;

        while bucket_start < end_time {
            if ranges.len() >= max_buckets {
                return Err(report!(OverviewError::TooManyBucketsError));
            }

            let aligned_bucket_start = self.truncate_to_bucket(bucket_start)?;
            let bucket_end = self.next_bucket_start(aligned_bucket_start)?.min(end_time);

            ranges.push((bucket_start, bucket_end));
            bucket_start = bucket_end;
        }

        Ok(ranges)
    }

    fn bucket_positions<I>(
        self,
        bucket_starts: I,
    ) -> OverviewResult<HashMap<PrimitiveDateTime, usize>>
    where
        I: IntoIterator<Item = PrimitiveDateTime>,
    {
        bucket_starts
            .into_iter()
            .enumerate()
            .map(|(position, bucket_start)| Ok((self.truncate_to_bucket(bucket_start)?, position)))
            .collect()
    }
}

pub struct OverviewAmount;

impl OverviewAmount {
    pub fn zero_filled_transaction_status_breakdown(
        status_breakdown: Vec<TransactionStatusAmount>,
    ) -> OverviewResult<Vec<TransactionStatusAmount>> {
        let currency = status_breakdown
            .first()
            .map(|status_amount| status_amount.credit_amount.currency)
            .unwrap_or_default();
        Self::zero_filled_transaction_status_breakdown_with_currency(status_breakdown, currency)
    }

    pub fn zero_filled_transaction_status_breakdown_with_currency(
        status_breakdown: Vec<TransactionStatusAmount>,
        currency: Currency,
    ) -> OverviewResult<Vec<TransactionStatusAmount>> {
        let mut status_map = status_breakdown
            .into_iter()
            .map(|status_amount| (status_amount.status, status_amount))
            .collect::<HashMap<_, _>>();

        ApiTransactionFilterStatus::iter()
            .filter(|status| *status != ApiTransactionFilterStatus::Archived)
            .map(|status| {
                status_map.remove(&status).map_or_else(
                    || {
                        Ok(TransactionStatusAmount {
                            status,
                            count: 0,
                            credit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                            debit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                        })
                    },
                    Ok,
                )
            })
            .collect()
    }

    pub fn zero_filled_account_transaction_status_breakdown(
        status_breakdown: Vec<AccountTransactionStatusAmount>,
        currency: Currency,
    ) -> OverviewResult<Vec<AccountTransactionStatusAmount>> {
        let mut status_map = status_breakdown
            .into_iter()
            .map(|status_amount| (status_amount.status, status_amount))
            .collect::<HashMap<_, _>>();

        ApiTransactionFilterStatus::iter()
            .filter(|status| *status != ApiTransactionFilterStatus::Archived)
            .map(|status| {
                status_map.remove(&status).map_or_else(
                    || {
                        Ok(AccountTransactionStatusAmount {
                            status,
                            credit_count: 0,
                            debit_count: 0,
                            credit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                            debit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                        })
                    },
                    Ok,
                )
            })
            .collect()
    }

    pub fn zero_filled_staging_entry_status_breakdown(
        status_breakdown: Vec<StagingEntryStatusAmount>,
        currency: Currency,
    ) -> OverviewResult<Vec<StagingEntryStatusAmount>> {
        let mut status_map = status_breakdown
            .into_iter()
            .map(|status_amount| (status_amount.status, status_amount))
            .collect::<HashMap<_, _>>();

        ApiStagingEntryCoarseStatus::iter()
            .filter(|status| *status != ApiStagingEntryCoarseStatus::Archived)
            .map(|status| {
                status_map.remove(&status).map_or_else(
                    || {
                        Ok(StagingEntryStatusAmount {
                            status,
                            count: 0,
                            credit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                            debit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                        })
                    },
                    Ok,
                )
            })
            .collect()
    }

    pub fn zero_filled_staging_entry_detailed_status_breakdown(
        detailed_status_breakdown: Vec<StagingEntryDetailedStatusAmount>,
        currency: Currency,
    ) -> OverviewResult<Vec<StagingEntryDetailedStatusAmount>> {
        let mut status_map = detailed_status_breakdown
            .into_iter()
            .map(|status_amount| (status_amount.detailed_status, status_amount))
            .collect::<HashMap<_, _>>();

        DieselStagingEntryStatus::iter()
            .filter(|status| *status != DieselStagingEntryStatus::Archived)
            .map(DomainStagingEntryStatus::from)
            .map(DomainStagingEntryStatus::to_api_response)
            .map(|detailed_status| {
                status_map.remove(&detailed_status).map_or_else(
                    || {
                        Ok(StagingEntryDetailedStatusAmount {
                            detailed_status,
                            count: 0,
                            credit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                            debit_amount: AmountResponse::from_minor_unit(
                                MinorUnit::zero(),
                                currency,
                            )
                            .change_context(OverviewError::InternalServerError)?,
                        })
                    },
                    Ok,
                )
            })
            .collect()
    }
}

impl ForeignTryFrom<TransactionOverviewRow> for Vec<TransactionStatusAmount> {
    type Error = Report<OverviewError>;

    fn foreign_try_from(row: TransactionOverviewRow) -> Result<Self, Self::Error> {
        let now = date_time::now();
        let mut amounts = Self::new();

        if row.missing_count > 0 {
            amounts.push(TransactionStatusAmount {
                status: ApiTransactionFilterStatus::Missing,
                count: row.missing_count,
                credit_amount: AmountResponse::from_minor_unit(row.missing_credit, row.currency)
                    .change_context(OverviewError::InternalServerError)?,
                debit_amount: AmountResponse::from_minor_unit(row.missing_debit, row.currency)
                    .change_context(OverviewError::InternalServerError)?,
            });
        }

        let remaining_count = row.total_count - row.missing_count;
        if remaining_count > 0 || row.missing_count == 0 {
            amounts.push(TransactionStatusAmount {
                status: ApiTransactionFilterStatus::from_domain_transaction_status(
                    &DomainTransactionStatus::from(row.status),
                    &AgingConfig::V1(AgingConfigV1::NoAging),
                    now,
                    now,
                ),
                count: remaining_count,
                credit_amount: AmountResponse::from_minor_unit(
                    row.total_credit - row.missing_credit,
                    row.currency,
                )
                .change_context(OverviewError::InternalServerError)?,
                debit_amount: AmountResponse::from_minor_unit(
                    row.total_debit - row.missing_debit,
                    row.currency,
                )
                .change_context(OverviewError::InternalServerError)?,
            });
        }

        Ok(amounts)
    }
}

impl ForeignTryFrom<RuleAccountTransactionOverviewRow> for Vec<AccountTransactionStatusAmount> {
    type Error = Report<OverviewError>;

    fn foreign_try_from(row: RuleAccountTransactionOverviewRow) -> Result<Self, Self::Error> {
        let now = date_time::now();
        let mut amounts = Self::new();

        if row.missing_credit_count > 0 || row.missing_debit_count > 0 {
            amounts.push(AccountTransactionStatusAmount {
                status: ApiTransactionFilterStatus::Missing,
                credit_count: row.missing_credit_count,
                debit_count: row.missing_debit_count,
                credit_amount: AmountResponse::from_minor_unit(row.missing_credit, row.currency)
                    .change_context(OverviewError::InternalServerError)?,
                debit_amount: AmountResponse::from_minor_unit(row.missing_debit, row.currency)
                    .change_context(OverviewError::InternalServerError)?,
            });
        }

        let remaining_credit_count = row.credit_count - row.missing_credit_count;
        let remaining_debit_count = row.debit_count - row.missing_debit_count;
        if remaining_credit_count > 0
            || remaining_debit_count > 0
            || (row.missing_credit_count == 0 && row.missing_debit_count == 0)
        {
            amounts.push(AccountTransactionStatusAmount {
                status: ApiTransactionFilterStatus::from_domain_transaction_status(
                    &DomainTransactionStatus::from(row.status),
                    &AgingConfig::V1(AgingConfigV1::NoAging),
                    now,
                    now,
                ),
                credit_count: remaining_credit_count,
                debit_count: remaining_debit_count,
                credit_amount: AmountResponse::from_minor_unit(
                    row.total_credit - row.missing_credit,
                    row.currency,
                )
                .change_context(OverviewError::InternalServerError)?,
                debit_amount: AmountResponse::from_minor_unit(
                    row.total_debit - row.missing_debit,
                    row.currency,
                )
                .change_context(OverviewError::InternalServerError)?,
            });
        }

        Ok(amounts)
    }
}

impl ForeignTryFrom<StagingEntryOverviewRow> for StagingEntryDetailedStatusAmount {
    type Error = Report<OverviewError>;

    fn foreign_try_from(row: StagingEntryOverviewRow) -> Result<Self, Self::Error> {
        Ok(Self {
            detailed_status: DomainStagingEntryStatus::from(row.status).to_api_response(),
            count: row.count,
            credit_amount: AmountResponse::from_minor_unit(row.credit_sum, row.currency)
                .change_context(OverviewError::InternalServerError)?,
            debit_amount: AmountResponse::from_minor_unit(row.debit_sum, row.currency)
                .change_context(OverviewError::InternalServerError)?,
        })
    }
}

pub struct OverviewTimeBuckets {
    pub ranges: Vec<DefinedTimeRange>,
    positions: HashMap<PrimitiveDateTime, usize>,
}

impl OverviewTimeBuckets {
    pub fn new(time_range: DefinedTimeRange, granularity: Granularity) -> OverviewResult<Self> {
        let bucket_ranges =
            granularity.bucket_ranges(time_range.start_time, time_range.end_time, MAX_BUCKETS)?;

        let positions = granularity.bucket_positions(
            bucket_ranges
                .iter()
                .map(|(bucket_start, _bucket_end)| *bucket_start),
        )?;

        let ranges = bucket_ranges
            .into_iter()
            .map(|(start_time, end_time)| DefinedTimeRange {
                start_time,
                end_time,
            })
            .collect();

        Ok(Self { ranges, positions })
    }

    pub fn position_for(&self, bucket_start: PrimitiveDateTime) -> Option<usize> {
        self.positions.get(&bucket_start).copied()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use time::{Date, Month, PrimitiveDateTime, Time};

    use super::*;

    fn datetime(hour: u8, minute: u8) -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::June, 22).unwrap(),
            Time::from_hms(hour, minute, 0).unwrap(),
        )
    }

    #[test]
    fn hourly_buckets_are_clipped_to_requested_range() {
        let buckets = OverviewTimeBuckets::new(
            DefinedTimeRange {
                start_time: datetime(10, 15),
                end_time: datetime(12, 20),
            },
            Granularity::Hour,
        )
        .unwrap();

        assert_eq!(buckets.ranges.len(), 3);
        assert_eq!(buckets.ranges[0].start_time, datetime(10, 15));
        assert_eq!(buckets.ranges[0].end_time, datetime(11, 0));
        assert_eq!(buckets.ranges[1].start_time, datetime(11, 0));
        assert_eq!(buckets.ranges[1].end_time, datetime(12, 0));
        assert_eq!(buckets.ranges[2].start_time, datetime(12, 0));
        assert_eq!(buckets.ranges[2].end_time, datetime(12, 20));
    }
}
