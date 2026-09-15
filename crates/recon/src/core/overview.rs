use std::collections::HashMap;

use api_models::{
    overview::{
        AccountStagingEntriesOverview, AccountStatusOverview, AccountTransactionStatusAmount,
        OverviewFilters, RuleAccountsOverview, RuleAccountsOverviewResponse,
        RuleTransactionsOverview, RuleTransactionsTimeSeries, StagingEntriesOverviewResponse,
        StagingEntryDetailedStatusAmount, StagingEntryOverviewFilters, StagingEntryStatusAmount,
        TimeSeriesFilters, TransactionStatusAmount, TransactionTimeBucket,
        TransactionsOverviewResponse, TransactionsTimeSeriesResponse,
    },
    staging_entry::ApiStagingEntryCoarseStatus,
};
use common_enums::Currency;
use common_types::{
    amount::AmountResponse, staging_entry::status::DomainStagingEntryStatus, units::MinorUnit,
};
use common_utils::{auth::AuthContext, date_time};
use diesel_models::staging_entry::StagingEntryOverviewRow;
use error_stack::{report, ResultExt};

use crate::{
    db::{
        account::AccountInterface, recon_rule::ReconRuleInterface,
        transaction::TransactionInterface,
    },
    domain::recon_engine::flows::ReconStrategyExt as ReconEngineStrategyExt,
    errors::overview::{OverviewError, OverviewResponse, OverviewResult},
    routes::AppState,
    types::{transformers::ForeignTryFrom, ApplicationResponse},
    utils::{
        overview::{OverviewAmount, OverviewTimeBuckets},
        recon_rule::RuleAccountExt,
        staging_entry::DomainStagingEntryStatusExt,
    },
};

type CoarseStatusKey = (ApiStagingEntryCoarseStatus, Currency);
type CoarseStatusTotals = (i64, MinorUnit, MinorUnit);

pub async fn get_transactions_overview(
    state: &AppState,
    auth: AuthContext,
    filters: OverviewFilters,
) -> OverviewResponse<TransactionsOverviewResponse> {
    filters
        .validate()
        .map_err(|_| OverviewError::TimeRangeValidationError)?;

    let now = date_time::now();
    let rules = match &filters.rule_ids {
        Some(rule_ids) => {
            state
                .store
                .list_recon_rules_by_ids(&auth.profile_id, rule_ids.clone())
                .await
        }
        None => {
            state
                .store
                .list_recon_rules_by_profile_id(&auth.profile_id, None)
                .await
        }
    }
    .change_context(OverviewError::InternalServerError)?;

    let aggregates = state
        .store
        .aggregate_transaction_status_counts_by_rule(
            &auth.profile_id,
            filters.time_range,
            rules
                .iter()
                .map(|rule| (rule.id.clone(), rule.aging_config.missing_before(now)))
                .collect(),
            filters.rule_ids,
        )
        .await
        .change_context(OverviewError::InternalServerError)?;

    let mut status_amounts_by_rule: HashMap<String, Vec<TransactionStatusAmount>> = HashMap::new();
    for row in aggregates {
        let rule_id = row.rule_id.clone();
        let status_amounts = Vec::<TransactionStatusAmount>::foreign_try_from(row)?;

        status_amounts_by_rule
            .entry(rule_id)
            .or_default()
            .extend(status_amounts);
    }

    let response: TransactionsOverviewResponse = rules
        .into_iter()
        .map(|rule| {
            let status_breakdown = status_amounts_by_rule.remove(&rule.id).unwrap_or_default();
            Ok(RuleTransactionsOverview {
                rule_id: rule.id,
                rule_name: rule.name,
                status_breakdown: OverviewAmount::zero_filled_transaction_status_breakdown(
                    status_breakdown,
                )?,
            })
        })
        .collect::<OverviewResult<_>>()?;

    Ok(ApplicationResponse::json(response))
}

pub async fn get_transactions_time_series(
    state: &AppState,
    auth: AuthContext,
    filters: TimeSeriesFilters,
) -> OverviewResponse<TransactionsTimeSeriesResponse> {
    filters
        .validate()
        .map_err(|_| OverviewError::TimeRangeValidationError)?;

    let now = date_time::now();
    let time_range = filters.time_range.into_defined(now);

    let buckets = OverviewTimeBuckets::new(time_range, filters.granularity)?;

    let rules = match &filters.rule_ids {
        Some(rule_ids) => {
            state
                .store
                .list_recon_rules_by_ids(&auth.profile_id, rule_ids.clone())
                .await
        }
        None => {
            state
                .store
                .list_recon_rules_by_profile_id(&auth.profile_id, None)
                .await
        }
    }
    .change_context(OverviewError::InternalServerError)?;

    let aggregates = state
        .store
        .aggregate_transaction_time_series_by_rule(
            &auth.profile_id,
            time_range,
            filters.granularity,
            rules
                .iter()
                .map(|rule| (rule.id.clone(), rule.aging_config.missing_before(now)))
                .collect(),
            filters.rule_ids,
        )
        .await
        .change_context(OverviewError::InternalServerError)?;

    // Scatter: drop each aggregate into its rule's bucket slot (slots align with `buckets.ranges`).
    let mut amounts_by_rule: HashMap<String, Vec<Vec<TransactionStatusAmount>>> = HashMap::new();
    for bucket_row in aggregates {
        let position = buckets
            .position_for(bucket_row.bucket_start)
            .ok_or_else(|| report!(OverviewError::InternalServerError))
            .attach_printable_lazy(|| {
                format!(
                    "time-series bucket {} has no matching axis slot",
                    bucket_row.bucket_start
                )
            })?;

        let rule_id = bucket_row.row.rule_id.clone();
        let status_amounts = Vec::<TransactionStatusAmount>::foreign_try_from(bucket_row.row)?;

        let rule_slots = amounts_by_rule
            .entry(rule_id)
            .or_insert_with(|| vec![Vec::new(); buckets.ranges.len()]);

        let slot = rule_slots
            .get_mut(position)
            .ok_or_else(|| report!(OverviewError::InternalServerError))
            .attach_printable_lazy(|| {
                format!("time-series bucket position {position} is outside the generated axis")
            })?;
        slot.extend(status_amounts);
    }

    // Gather: lay every rule's slots back onto the bucket axis, pairing each with its
    // `[start, end)` window and zero-filling the status breakdown.
    let response: TransactionsTimeSeriesResponse = rules
        .into_iter()
        .map(|rule| {
            let rule_slots = amounts_by_rule
                .remove(&rule.id)
                .unwrap_or_else(|| vec![Vec::new(); buckets.ranges.len()]);

            Ok(RuleTransactionsTimeSeries {
                rule_id: rule.id,
                rule_name: rule.name,
                time_series: buckets
                    .ranges
                    .iter()
                    .zip(rule_slots)
                    .map(|(time_range, status_breakdown)| {
                        Ok(TransactionTimeBucket {
                            time_range: (*time_range).into(),
                            status_breakdown:
                                OverviewAmount::zero_filled_transaction_status_breakdown(
                                    status_breakdown,
                                )?,
                        })
                    })
                    .collect::<OverviewResult<Vec<_>>>()?,
            })
        })
        .collect::<OverviewResult<_>>()?;

    Ok(ApplicationResponse::json(response))
}

pub async fn get_transactions_rule_account_breakdown(
    state: &AppState,
    auth: AuthContext,
    filters: OverviewFilters,
) -> OverviewResponse<RuleAccountsOverviewResponse> {
    filters
        .validate()
        .map_err(|_| OverviewError::TimeRangeValidationError)?;

    let rules = match &filters.rule_ids {
        Some(rule_ids) => {
            state
                .store
                .list_recon_rules_by_ids(&auth.profile_id, rule_ids.clone())
                .await
        }
        None => {
            state
                .store
                .list_recon_rules_by_profile_id(&auth.profile_id, None)
                .await
        }
    }
    .change_context(OverviewError::InternalServerError)?;

    let now = date_time::now();

    let (mut rule_accounts_by_rule, rule_missing_threshold_map) = rules.iter().fold(
        (
            HashMap::with_capacity(rules.len()),
            HashMap::with_capacity(rules.len()),
        ),
        |(mut accounts_by_rule, mut missing_thresholds), rule| {
            let rule_id = rule.id.clone();

            accounts_by_rule.insert(
                rule_id.clone(),
                rule.strategy.get_recon_engine_type().get_rule_accounts(),
            );

            missing_thresholds.insert(rule_id, rule.aging_config.missing_before(now));

            (accounts_by_rule, missing_thresholds)
        },
    );

    let aggregates = state
        .store
        .aggregate_transaction_status_counts_by_rule_and_account(
            &auth.profile_id,
            filters.time_range,
            rule_missing_threshold_map,
            filters.rule_ids,
        )
        .await
        .change_context(OverviewError::InternalServerError)?;

    let accounts = state
        .store
        .list_accounts_by_profile_id(&auth.profile_id)
        .await
        .change_context(OverviewError::InternalServerError)?;

    let accounts_by_id = accounts
        .into_iter()
        .map(|account| (account.id.clone(), account))
        .collect::<HashMap<_, _>>();

    let mut status_amounts_by_rule_account: HashMap<
        String,
        HashMap<String, Vec<AccountTransactionStatusAmount>>,
    > = HashMap::new();
    for row in aggregates {
        let rule_id = row.rule_id.clone();
        let account_id = row.account_id.clone();
        let status_amounts = Vec::<AccountTransactionStatusAmount>::foreign_try_from(row)?;
        status_amounts_by_rule_account
            .entry(rule_id)
            .or_default()
            .entry(account_id)
            .or_default()
            .extend(status_amounts);
    }

    let response: RuleAccountsOverviewResponse = rules
        .into_iter()
        .map(|rule| {
            let mut status_amounts_by_account = status_amounts_by_rule_account
                .remove(&rule.id)
                .unwrap_or_default();

            let mut accounts = Vec::new();
            for (account_id, rule_account_type) in rule_accounts_by_rule
                .remove(&rule.id)
                .unwrap_or_default()
                .into_iter()
            {
                let account = accounts_by_id
                    .get(&account_id)
                    .ok_or(OverviewError::InternalServerError)?;
                let status_breakdown = status_amounts_by_account
                    .remove(&account_id)
                    .unwrap_or_default();
                let status_breakdown =
                    OverviewAmount::zero_filled_account_transaction_status_breakdown(
                        status_breakdown,
                        account.currency,
                    )?;

                accounts.push(AccountStatusOverview {
                    account_id: account.id.clone(),
                    account_name: account.name.clone(),
                    account_type: account.balance_direction,
                    rule_account_type,
                    status_breakdown,
                });
            }

            Ok(RuleAccountsOverview {
                rule_id: rule.id,
                rule_name: rule.name,
                accounts,
            })
        })
        .collect::<OverviewResult<_>>()?;

    Ok(ApplicationResponse::json(response))
}

pub async fn get_staging_entries_overview(
    state: &AppState,
    auth: AuthContext,
    filters: StagingEntryOverviewFilters,
) -> OverviewResponse<StagingEntriesOverviewResponse> {
    filters
        .validate()
        .map_err(|_| OverviewError::TimeRangeValidationError)?;

    let aggregates = state
        .store
        .aggregate_staging_entry_status_counts_by_account(
            &auth.profile_id,
            filters.time_range,
            filters.account_ids.clone(),
            filters.transformation_history_ids,
        )
        .await
        .change_context(OverviewError::InternalServerError)?;

    let accounts = match filters.account_ids {
        Some(account_ids) => {
            state
                .store
                .list_accounts_by_profile_id_and_account_ids(&auth.profile_id, &account_ids)
                .await
        }
        None => {
            state
                .store
                .list_accounts_by_profile_id(&auth.profile_id)
                .await
        }
    }
    .change_context(OverviewError::InternalServerError)?;

    let (mut coarse_totals_by_account, mut detailed_rows_by_account) = aggregates.into_iter().fold(
        (
            HashMap::<String, HashMap<CoarseStatusKey, CoarseStatusTotals>>::new(),
            HashMap::<String, Vec<StagingEntryOverviewRow>>::new(),
        ),
        |(mut coarse, mut detailed), row| {
            let totals = coarse
                .entry(row.account_id.clone())
                .or_default()
                .entry((
                    DomainStagingEntryStatus::from(row.status)
                        .to_api_response()
                        .coarse(),
                    row.currency,
                ))
                .or_insert((0, MinorUnit::zero(), MinorUnit::zero()));
            totals.0 += row.count;
            totals.1 = totals.1 + row.credit_sum;
            totals.2 = totals.2 + row.debit_sum;

            detailed
                .entry(row.account_id.clone())
                .or_default()
                .push(row);

            (coarse, detailed)
        },
    );

    let response: StagingEntriesOverviewResponse = accounts
        .into_iter()
        .map(|account| {
            let status_breakdown = coarse_totals_by_account
                .remove(&account.id)
                .unwrap_or_default()
                .into_iter()
                .map(|((status, currency), (count, credit_sum, debit_sum))| {
                    Ok(StagingEntryStatusAmount {
                        status,
                        count,
                        credit_amount: AmountResponse::from_minor_unit(credit_sum, currency)
                            .change_context(OverviewError::InternalServerError)?,
                        debit_amount: AmountResponse::from_minor_unit(debit_sum, currency)
                            .change_context(OverviewError::InternalServerError)?,
                    })
                })
                .collect::<OverviewResult<Vec<_>>>()?;
            let status_breakdown = OverviewAmount::zero_filled_staging_entry_status_breakdown(
                status_breakdown,
                account.currency,
            )?;

            let detailed_status_breakdown = detailed_rows_by_account
                .remove(&account.id)
                .unwrap_or_default()
                .into_iter()
                .map(StagingEntryDetailedStatusAmount::foreign_try_from)
                .collect::<OverviewResult<Vec<_>>>()?;
            let detailed_status_breakdown =
                OverviewAmount::zero_filled_staging_entry_detailed_status_breakdown(
                    detailed_status_breakdown,
                    account.currency,
                )?;

            Ok(AccountStagingEntriesOverview {
                account_id: account.id,
                account_name: account.name,
                account_type: account.balance_direction,
                status_breakdown,
                detailed_status_breakdown,
            })
        })
        .collect::<OverviewResult<_>>()?;

    Ok(ApplicationResponse::json(response))
}
