use crate::{
    api::{
        account_test::create_test_account,
        staging_entry_test::staging_entry_request,
        transaction_test::{
            create_test_recon_rule, create_test_transaction, create_transaction_for_filtering,
        },
        transformation_test::create_transformation_config,
    },
    common::TestApp,
};
use api_models::{
    overview::{
        AccountStatusOverview, RuleAccountsOverview, RuleAccountsOverviewResponse,
        StagingEntriesOverviewResponse, TransactionsOverviewResponse,
        TransactionsTimeSeriesResponse,
    },
    recon_rule::aging_config::{AgingConfigApiRequest, ThresholdApiRequest},
    staging_entry::{
        ApiStagingEntryCoarseStatus, GetStagingEntryResponse, StagingEntryCreateRequest,
        VoidStagingEntryRequest,
    },
    transaction::{
        ApiTransactionFilterStatus, ApiTransactionStatus, DirectTransactionEntry,
        ExpectedTransactionEntry, GetTransactionResponse, SearchExpectation, TransactionEntry,
        TransactionRequest,
    },
};
use common_enums::{BalanceDirection, Currency, ReconRuleAccountType, StagingEntryProcessingMode};
use common_types::{transaction::status::TransactionMatchedData, units::FloatMajorUnit};
use masking::Secret;
use pretty_assertions::assert_eq;
use serde_json::json;
use strum::IntoEnumIterator;
use time::PrimitiveDateTime;

fn format_time(time: PrimitiveDateTime) -> String {
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z").unwrap();
    time.format(&format).unwrap()
}

fn parse_time(time: &str) -> PrimitiveDateTime {
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z").unwrap();
    PrimitiveDateTime::parse(time, &format).unwrap()
}

async fn create_expected_transaction_with_effective_at(
    client: &crate::common::TestClient,
    token: &str,
    debit_account_id: &str,
    credit_account_id: &str,
    rule_id: &str,
    effective_at: PrimitiveDateTime,
) -> GetTransactionResponse {
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id.to_string(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at,
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Expected(ExpectedTransactionEntry {
                account_id: credit_account_id.to_string(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at,
                metadata: Secret::new(json!({"test": "data"})),
                search_expectation: SearchExpectation {
                    rule_id: rule_id.to_string(),
                    source_value: "payment_123".to_string(),
                    target_key: "metadata.transaction_id".parse().unwrap(),
                },
                order_id: "order_test".to_string(),
            }),
        ],
        rule_id: rule_id.to_string(),
    };

    let response = client
        .post("/transactions")
        .with_auth(token)
        .json(&transaction_request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    response.json().await.unwrap()
}

fn find_rule_accounts_overview<'a>(
    overview: &'a RuleAccountsOverviewResponse,
    rule_id: &str,
) -> &'a RuleAccountsOverview {
    overview
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .unwrap()
}

fn find_account_overview<'a>(
    rule: &'a RuleAccountsOverview,
    account_id: &str,
) -> &'a AccountStatusOverview {
    rule.accounts
        .iter()
        .find(|account| account.account_id == account_id)
        .unwrap()
}

fn total_credit_count(account: &AccountStatusOverview) -> i64 {
    account
        .status_breakdown
        .iter()
        .map(|status_amount| status_amount.credit_count)
        .sum()
}

fn total_debit_count(account: &AccountStatusOverview) -> i64 {
    account
        .status_breakdown
        .iter()
        .map(|status_amount| status_amount.debit_count)
        .sum()
}

fn total_credit(account: &AccountStatusOverview) -> FloatMajorUnit {
    account
        .status_breakdown
        .iter()
        .map(|status_amount| status_amount.credit_amount.value)
        .sum()
}

fn total_debit(account: &AccountStatusOverview) -> FloatMajorUnit {
    account
        .status_breakdown
        .iter()
        .map(|status_amount| status_amount.debit_amount.value)
        .sum()
}

#[actix_web::test]
async fn transactions_summary_groups_by_rule_and_status() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "overview_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    for _ in 0..2 {
        create_test_transaction(
            &client,
            &token,
            &debit_account_id,
            &credit_account_id,
            &rule_id,
            &merchant_id,
            &profile_id,
            ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
            None,
        )
        .await;
    }

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    // Summary with explicit end_time: zero-filled array, no time_series.
    let response = client
        .get(&format!(
            "/overview/transactions?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsOverviewResponse = response.json().await.unwrap();

    assert_eq!(overview.len(), 1);
    let rule_overview = &overview[0];
    assert_eq!(rule_overview.rule_id, rule_id);
    assert_eq!(rule_overview.rule_name, "Test Rule overview_rule");
    assert_eq!(
        rule_overview.status_breakdown.len(),
        ApiTransactionFilterStatus::iter().count() - 1 // Excluding `archived` status
    );
    let matched = rule_overview
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiTransactionFilterStatus::MatchedAuto)
        .unwrap();
    assert_eq!(matched.count, 2);
    assert_eq!(matched.credit_amount.value, FloatMajorUnit::new(200.0));
    assert_eq!(matched.debit_amount.value, FloatMajorUnit::new(200.0));
    assert_eq!(matched.credit_amount.currency, Currency::USD);
    assert_eq!(matched.debit_amount.currency, Currency::USD);

    // Summary with only start_time: end_time defaults to now, still finds the data.
    let response = client
        .get(&format!("/overview/transactions?start_time={start_time}"))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsOverviewResponse = response.json().await.unwrap();
    let matched = overview[0]
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiTransactionFilterStatus::MatchedAuto)
        .unwrap();
    assert_eq!(matched.count, 2);
}

#[actix_web::test]
async fn transactions_summary_and_time_series_filter_by_rule_ids() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;

    let rule_a_id = create_test_recon_rule(
        &client,
        &token,
        "overview_rule_filter_a",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_b_id = create_test_recon_rule(
        &client,
        &token,
        "overview_rule_filter_b",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_c_id = create_test_recon_rule(
        &client,
        &token,
        "overview_rule_filter_c",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    for rule_id in [&rule_a_id, &rule_b_id, &rule_c_id] {
        create_test_transaction(
            &client,
            &token,
            &debit_account_id,
            &credit_account_id,
            rule_id,
            &merchant_id,
            &profile_id,
            ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
            None,
        )
        .await;
    }

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    // Summary filtered by a single rule_ids entry only returns that rule.
    let response = client
        .get(&format!(
            "/overview/transactions?start_time={start_time}&end_time={end_time}&rule_ids={rule_a_id}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsOverviewResponse = response.json().await.unwrap();
    assert_eq!(overview.len(), 1);
    assert_eq!(overview[0].rule_id, rule_a_id);

    // Summary filtered by comma-separated rule_ids returns exactly those rules, excluding rule_c.
    let response = client
        .get(&format!(
            "/overview/transactions?start_time={start_time}&end_time={end_time}&rule_ids={rule_a_id},{rule_b_id}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsOverviewResponse = response.json().await.unwrap();
    let mut returned_rule_ids: Vec<_> = overview.iter().map(|rule| rule.rule_id.clone()).collect();
    returned_rule_ids.sort();
    let mut expected_rule_ids = vec![rule_a_id.clone(), rule_b_id.clone()];
    expected_rule_ids.sort();
    assert_eq!(returned_rule_ids, expected_rule_ids);

    // Time-series filtered by comma-separated rule_ids returns exactly those rules.
    let response = client
        .get(&format!(
            "/overview/transactions/time_series?start_time={start_time}&end_time={end_time}&granularity=day&rule_ids={rule_b_id},{rule_c_id}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsTimeSeriesResponse = response.json().await.unwrap();
    let mut returned_rule_ids: Vec<_> = overview.iter().map(|rule| rule.rule_id.clone()).collect();
    returned_rule_ids.sort();
    let mut expected_rule_ids = vec![rule_b_id.clone(), rule_c_id.clone()];
    expected_rule_ids.sort();
    assert_eq!(returned_rule_ids, expected_rule_ids);
}

#[actix_web::test]
async fn transactions_time_series_buckets_by_granularity() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "overview_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    for _ in 0..2 {
        create_test_transaction(
            &client,
            &token,
            &debit_account_id,
            &credit_account_id,
            &rule_id,
            &merchant_id,
            &profile_id,
            ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
            None,
        )
        .await;
    }

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));
    let response = client
        .get(&format!(
            "/overview/transactions/time_series?start_time={start_time}&end_time={end_time}&granularity=day"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsTimeSeriesResponse = response.json().await.unwrap();

    assert_eq!(overview.len(), 1);
    let rule_overview = &overview[0];
    assert_eq!(rule_overview.rule_id, rule_id);

    // One day bucket, zero-filled to the full status enum.
    let status_breakdown = rule_overview
        .time_series
        .iter()
        .flat_map(|bucket| &bucket.status_breakdown)
        .collect::<Vec<_>>();
    assert_eq!(
        status_breakdown.len(),
        ApiTransactionFilterStatus::iter().count() - 1 // Excluding `archived` status
    );
    let matched_count = rule_overview
        .time_series
        .iter()
        .flat_map(|bucket| &bucket.status_breakdown)
        .filter(|status_amount| status_amount.status == ApiTransactionFilterStatus::MatchedAuto)
        .map(|status_amount| status_amount.count)
        .sum::<i64>();
    assert_eq!(matched_count, 2);

    // A single-bucket window (granularity=hour) keeps the clipped time_range start.
    let response = client
        .get(&format!(
            "/overview/transactions/time_series?start_time={start_time}&end_time={end_time}&granularity=hour"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsTimeSeriesResponse = response.json().await.unwrap();
    assert_eq!(
        overview[0]
            .time_series
            .first()
            .unwrap()
            .time_range
            .start_time,
        parse_time(&start_time)
    );
}

#[actix_web::test]
async fn time_series_window_after_data_still_lists_rule_empty() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "overview_time_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    create_test_transaction(
        &client,
        &token,
        &debit_account_id,
        &credit_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;

    let now = common_utils::date_time::now();
    let start_time = format_time(now + time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(2));
    let response = client
        .get(&format!(
            "/overview/transactions/time_series?start_time={start_time}&end_time={end_time}&granularity=hour"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsTimeSeriesResponse = response.json().await.unwrap();
    assert_eq!(overview.len(), 1);
    assert_eq!(overview[0].rule_id, rule_id);
    // The rule is still listed; every bucket in the window is zero-filled.
    assert!(overview[0].time_series.iter().all(|bucket| bucket
        .status_breakdown
        .iter()
        .all(|status_amount| status_amount.count == 0)));
}

#[actix_web::test]
async fn transactions_reports_aged_expected_as_missing() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "overview_missing_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::WithThreshold {
            threshold: ThresholdApiRequest::WeekDays { value: 5 },
        },
    )
    .await;

    let now = common_utils::date_time::now();
    let effective_at = now - time::Duration::days(10);
    create_expected_transaction_with_effective_at(
        &client,
        &token,
        &debit_account_id,
        &credit_account_id,
        &rule_id,
        effective_at,
    )
    .await;

    let start_time = format_time(effective_at - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    // Summary reports aged expected as missing.
    let response = client
        .get(&format!(
            "/overview/transactions?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsOverviewResponse = response.json().await.unwrap();
    let rule_overview = overview
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .unwrap();
    let missing = rule_overview
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiTransactionFilterStatus::Missing)
        .unwrap();
    assert_eq!(missing.count, 1);
    assert_eq!(missing.credit_amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(missing.debit_amount.value, FloatMajorUnit::new(100.0));

    // Time-series reports it too.
    let response = client
        .get(&format!(
            "/overview/transactions/time_series?start_time={start_time}&end_time={end_time}&granularity=day"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsTimeSeriesResponse = response.json().await.unwrap();
    let missing_count = overview
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .unwrap()
        .time_series
        .iter()
        .flat_map(|bucket| &bucket.status_breakdown)
        .filter(|status_amount| status_amount.status == ApiTransactionFilterStatus::Missing)
        .map(|status_amount| status_amount.count)
        .sum::<i64>();
    assert_eq!(missing_count, 1);
}

#[actix_web::test]
async fn time_series_splits_expected_rows_by_aging_cutoff() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "overview_split_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::WithThreshold {
            threshold: ThresholdApiRequest::WeekDays { value: 5 },
        },
    )
    .await;

    // Two transactions, both stored as `Expected`; only the aged one is past the rule's cutoff.
    let now = common_utils::date_time::now();
    let aged_effective_at = now - time::Duration::days(60);
    let recent_effective_at = now - time::Duration::hours(1);
    create_expected_transaction_with_effective_at(
        &client,
        &token,
        &debit_account_id,
        &credit_account_id,
        &rule_id,
        aged_effective_at,
    )
    .await;
    create_expected_transaction_with_effective_at(
        &client,
        &token,
        &debit_account_id,
        &credit_account_id,
        &rule_id,
        recent_effective_at,
    )
    .await;

    let start_time = format_time(aged_effective_at - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));
    let response = client
        .get(&format!(
            "/overview/transactions/time_series?start_time={start_time}&end_time={end_time}&granularity=month"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsTimeSeriesResponse = response.json().await.unwrap();

    let rule = overview
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .unwrap();
    let count_for = |status: ApiTransactionFilterStatus| -> i64 {
        rule.time_series
            .iter()
            .flat_map(|bucket| &bucket.status_breakdown)
            .filter(|status_amount| status_amount.status == status)
            .map(|status_amount| status_amount.count)
            .sum()
    };
    // The DB derives `missing` per row (`effective_at < cutoff`), so two rows that are *both*
    // stored as `Expected` come back split: the aged one as Missing, the recent one as Expected.
    // This per-row split is exactly what lets a single bucket straddling the cutoff report both.
    assert_eq!(count_for(ApiTransactionFilterStatus::Missing), 1);
    assert_eq!(count_for(ApiTransactionFilterStatus::Expected), 1);
}

#[actix_web::test]
async fn rule_accounts_summary_groups_by_rule_and_configured_accounts() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id = create_test_account(
        &client,
        &token,
        "Rule Source Account",
        BalanceDirection::Debit,
    )
    .await;
    let credit_account_id = create_test_account(
        &client,
        &token,
        "Rule Target Account",
        BalanceDirection::Credit,
    )
    .await;

    let rule_a_id = create_test_recon_rule(
        &client,
        &token,
        "rule_accounts_rule_a",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_b_id = create_test_recon_rule(
        &client,
        &token,
        "rule_accounts_rule_b",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // This fixture places both credit and debit entries on the source account,
    // which verifies per-account credit/debit splitting in one rule bucket.
    create_test_transaction(
        &client,
        &token,
        &debit_account_id,
        &credit_account_id,
        &rule_a_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;
    create_transaction_for_filtering(
        &client,
        &token,
        &debit_account_id,
        &credit_account_id,
        &rule_b_id,
    )
    .await;

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    let response = client
        .get(&format!(
            "/overview/transactions/rule_account_breakdown?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: RuleAccountsOverviewResponse = response.json().await.unwrap();
    assert_eq!(overview.len(), 2);

    let rule_a = find_rule_accounts_overview(&overview, &rule_a_id);
    assert_eq!(rule_a.accounts.len(), 2);

    let rule_a_source = find_account_overview(rule_a, &debit_account_id);
    assert_eq!(rule_a_source.account_name, "Rule Source Account");
    assert_eq!(rule_a_source.account_type, BalanceDirection::Debit);
    assert_eq!(
        rule_a_source.rule_account_type,
        ReconRuleAccountType::Source
    );
    assert_eq!(
        rule_a_source.status_breakdown.len(),
        ApiTransactionFilterStatus::iter().count() - 1 // Excluding `archived` status
    );
    // The source account holds one credit-direction entry and one debit-direction
    // entry from the same transaction, so credit/debit counts must be reported
    // separately (1 each) rather than deduplicated into a single overall count.
    assert_eq!(total_credit_count(rule_a_source), 1);
    assert_eq!(total_debit_count(rule_a_source), 1);
    assert_eq!(total_credit(rule_a_source), FloatMajorUnit::new(100.0));
    assert_eq!(total_debit(rule_a_source), FloatMajorUnit::new(100.0));

    let rule_a_target = find_account_overview(rule_a, &credit_account_id);
    assert_eq!(rule_a_target.account_name, "Rule Target Account");
    assert_eq!(rule_a_target.account_type, BalanceDirection::Credit);
    assert_eq!(
        rule_a_target.rule_account_type,
        ReconRuleAccountType::Target
    );
    assert_eq!(
        rule_a_target.status_breakdown.len(),
        ApiTransactionFilterStatus::iter().count() - 1 // Excluding `archived` status
    );
    assert_eq!(total_credit_count(rule_a_target), 0);
    assert_eq!(total_debit_count(rule_a_target), 0);

    let rule_b = find_rule_accounts_overview(&overview, &rule_b_id);
    let rule_b_source = find_account_overview(rule_b, &debit_account_id);
    let rule_b_target = find_account_overview(rule_b, &credit_account_id);
    assert_eq!(total_credit_count(rule_b_source), 0);
    assert_eq!(total_debit_count(rule_b_source), 1);
    assert_eq!(total_debit(rule_b_source), FloatMajorUnit::new(100.0));
    assert_eq!(total_credit_count(rule_b_target), 1);
    assert_eq!(total_debit_count(rule_b_target), 0);
    assert_eq!(total_credit(rule_b_target), FloatMajorUnit::new(100.0));

    let response = client
        .get(&format!(
            "/overview/transactions/rule_account_breakdown?start_time={start_time}&end_time={end_time}&rule_ids={rule_a_id}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: RuleAccountsOverviewResponse = response.json().await.unwrap();
    assert_eq!(overview.len(), 1);
    assert_eq!(overview[0].rule_id, rule_a_id);

    let response = client
        .get(&format!(
            "/overview/transactions/rule_account_breakdown?start_time={start_time}&end_time={end_time}&rule_ids=missing_rule"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: RuleAccountsOverviewResponse = response.json().await.unwrap();
    assert!(overview.is_empty());
}

#[actix_web::test]
async fn staging_entries_summary_groups_by_account() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id = create_test_account(
        &client,
        &token,
        "Staging Overview Account A",
        BalanceDirection::Debit,
    )
    .await;
    let credit_account_id = create_test_account(
        &client,
        &token,
        "Staging Overview Account B",
        BalanceDirection::Credit,
    )
    .await;

    let transformation = create_transformation_config(
        &profile_id,
        &debit_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let create_request = staging_entry_request(
        &debit_account_id,
        BalanceDirection::Debit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation.transformation_id,
    );
    let create_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&create_request)
        .send()
        .await;
    assert_eq!(create_response.status(), 201);

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    let response = client
        .get(&format!(
            "/overview/staging_entries?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: StagingEntriesOverviewResponse = response.json().await.unwrap();
    assert_eq!(overview.len(), 2);

    let touched = overview
        .iter()
        .find(|account| account.account_id == debit_account_id)
        .unwrap();
    assert_eq!(touched.account_name, "Staging Overview Account A");
    assert_eq!(touched.account_type, BalanceDirection::Debit);
    // The coarse breakdown carries only the umbrella statuses, and Archived is excluded from the
    // breakdown entirely (superseded versions aren't counted).
    assert_eq!(touched.status_breakdown.len(), 4);
    assert!(touched
        .status_breakdown
        .iter()
        .all(|status_amount| status_amount.status != ApiStagingEntryCoarseStatus::Archived));
    let status_amount = touched
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiStagingEntryCoarseStatus::Pending)
        .unwrap();
    assert_eq!(status_amount.count, 1);
    assert_eq!(status_amount.debit_amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(status_amount.credit_amount.value, FloatMajorUnit::new(0.0));

    let untouched = overview
        .iter()
        .find(|account| account.account_id == credit_account_id)
        .unwrap();
    assert_eq!(untouched.account_type, BalanceDirection::Credit);
    assert!(untouched
        .status_breakdown
        .iter()
        .all(|status_amount| status_amount.count == 0));
}

#[actix_web::test]
async fn staging_entries_summary_excludes_archived() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id = create_test_account(
        &client,
        &token,
        "Archive Test Account",
        BalanceDirection::Debit,
    )
    .await;

    let transformation = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let create_request = staging_entry_request(
        &account_id,
        BalanceDirection::Debit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation.transformation_id,
    );
    let create_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&create_request)
        .send()
        .await;
    assert_eq!(create_response.status(), 201);
    let original_entry: GetStagingEntryResponse = create_response.json().await.unwrap();

    // Voiding creates a new Void-status version and archives the original version —
    // the archived version must not be counted anywhere in the overview.
    let void_response = client
        .put(&format!("/staging_entries/{}/void", original_entry.id))
        .with_auth(&token)
        .json(&VoidStagingEntryRequest {
            reason: Some("test archive exclusion".to_string()),
        })
        .send()
        .await;
    assert_eq!(void_response.status(), 200);

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    let response = client
        .get(&format!(
            "/overview/staging_entries?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: StagingEntriesOverviewResponse = response.json().await.unwrap();
    let account = overview
        .iter()
        .find(|account| account.account_id == account_id)
        .unwrap();

    // No "archived" key at all — not even zero-filled.
    assert!(account
        .status_breakdown
        .iter()
        .all(|status_amount| status_amount.status != ApiStagingEntryCoarseStatus::Archived));

    // The Void version is still counted normally.
    let void_count = account
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiStagingEntryCoarseStatus::Void)
        .map(|status_amount| status_amount.count)
        .unwrap_or_default();
    assert_eq!(void_count, 1);
}

#[actix_web::test]
async fn staging_entries_summary_filters_by_account_ids_and_transformation_history_ids() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_a_id =
        create_test_account(&client, &token, "Filter Account A", BalanceDirection::Debit).await;
    let account_b_id =
        create_test_account(&client, &token, "Filter Account B", BalanceDirection::Debit).await;

    let transformation_a = create_transformation_config(
        &profile_id,
        &account_a_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let transformation_b = create_transformation_config(
        &profile_id,
        &account_b_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let request_a = StagingEntryCreateRequest {
        transformation_history_id: Some("trans_his_test_a".to_string()),
        ..staging_entry_request(
            &account_a_id,
            BalanceDirection::Debit,
            StagingEntryProcessingMode::Transaction,
            100.0,
            Currency::USD,
            transformation_a.transformation_id,
        )
    };
    let response_a = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request_a)
        .send()
        .await;
    assert_eq!(response_a.status(), 201);

    let request_b = StagingEntryCreateRequest {
        transformation_history_id: Some("trans_his_test_b".to_string()),
        ..staging_entry_request(
            &account_b_id,
            BalanceDirection::Debit,
            StagingEntryProcessingMode::Transaction,
            50.0,
            Currency::USD,
            transformation_b.transformation_id,
        )
    };
    let response_b = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request_b)
        .send()
        .await;
    assert_eq!(response_b.status(), 201);

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    // account_ids narrows the account list itself: only the requested account comes back.
    let response = client
        .get(&format!(
            "/overview/staging_entries?start_time={start_time}&end_time={end_time}&account_ids={account_a_id}"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: StagingEntriesOverviewResponse = response.json().await.unwrap();
    assert_eq!(overview.len(), 1);
    assert_eq!(overview[0].account_id, account_a_id);

    // transformation_history_ids narrows the aggregation only: every account for the
    // profile still appears, but only the matching account's count is non-zero.
    let response = client
        .get(&format!(
            "/overview/staging_entries?start_time={start_time}&end_time={end_time}&transformation_history_ids=trans_his_test_a"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: StagingEntriesOverviewResponse = response.json().await.unwrap();
    assert_eq!(overview.len(), 2);

    let matched_account = overview
        .iter()
        .find(|account| account.account_id == account_a_id)
        .unwrap();
    let matched_count = matched_account
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiStagingEntryCoarseStatus::Pending)
        .map(|status_amount| status_amount.count)
        .unwrap_or_default();
    assert_eq!(matched_count, 1);

    let other_account = overview
        .iter()
        .find(|account| account.account_id == account_b_id)
        .unwrap();
    assert!(other_account
        .status_breakdown
        .iter()
        .all(|status_amount| status_amount.count == 0));
}

#[actix_web::test]
async fn staging_entries_summary_transformation_history_ids_narrows_within_account() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Both staging entries live on the SAME account, so this isolates the
    // transformation_history_id predicate itself from account-level narrowing.
    let account_id =
        create_test_account(&client, &token, "Same Account", BalanceDirection::Debit).await;

    let transformation = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let request_x = StagingEntryCreateRequest {
        transformation_history_id: Some("trans_his_test_x".to_string()),
        ..staging_entry_request(
            &account_id,
            BalanceDirection::Debit,
            StagingEntryProcessingMode::Transaction,
            100.0,
            Currency::USD,
            transformation.transformation_id.clone(),
        )
    };
    let response_x = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request_x)
        .send()
        .await;
    assert_eq!(response_x.status(), 201);

    let request_y = StagingEntryCreateRequest {
        transformation_history_id: Some("trans_his_test_y".to_string()),
        ..staging_entry_request(
            &account_id,
            BalanceDirection::Debit,
            StagingEntryProcessingMode::Transaction,
            50.0,
            Currency::USD,
            transformation.transformation_id,
        )
    };
    let response_y = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request_y)
        .send()
        .await;
    assert_eq!(response_y.status(), 201);

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    // Filtering to trans_his_test_x alone must count only that one entry, not both.
    let response = client
        .get(&format!(
            "/overview/staging_entries?start_time={start_time}&end_time={end_time}&transformation_history_ids=trans_his_test_x"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: StagingEntriesOverviewResponse = response.json().await.unwrap();
    let account = overview
        .iter()
        .find(|account| account.account_id == account_id)
        .unwrap();
    let count = account
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiStagingEntryCoarseStatus::Pending)
        .map(|status_amount| status_amount.count)
        .unwrap_or_default();
    assert_eq!(count, 1);

    // A filter matching neither transformation_history_id must count zero.
    let response = client
        .get(&format!(
            "/overview/staging_entries?start_time={start_time}&end_time={end_time}&transformation_history_ids=trans_his_test_nonexistent"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: StagingEntriesOverviewResponse = response.json().await.unwrap();
    let account = overview
        .iter()
        .find(|account| account.account_id == account_id)
        .unwrap();
    assert!(account
        .status_breakdown
        .iter()
        .all(|status_amount| status_amount.count == 0));

    // Comma-separated list matches both entries.
    let response = client
        .get(&format!(
            "/overview/staging_entries?start_time={start_time}&end_time={end_time}&transformation_history_ids=trans_his_test_x,trans_his_test_y"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: StagingEntriesOverviewResponse = response.json().await.unwrap();
    let account = overview
        .iter()
        .find(|account| account.account_id == account_id)
        .unwrap();
    let count = account
        .status_breakdown
        .iter()
        .find(|status_amount| status_amount.status == ApiStagingEntryCoarseStatus::Pending)
        .map(|status_amount| status_amount.count)
        .unwrap_or_default();
    assert_eq!(count, 2);
}

#[actix_web::test]
async fn overview_is_scoped_to_token_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "overview_scope_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &debit_account_id,
        &credit_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;

    let (_other_merchant_id, _other_profile_id, other_token) =
        client.bootstrap("Other Merchant", "Other Profile").await;

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    let response = client
        .get(&format!(
            "/overview/transactions?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&other_token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: TransactionsOverviewResponse = response.json().await.unwrap();
    assert!(overview.is_empty());

    let response = client
        .get(&format!(
            "/overview/transactions/rule_account_breakdown?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&other_token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let overview: RuleAccountsOverviewResponse = response.json().await.unwrap();
    assert!(overview.is_empty());
}

#[actix_web::test]
async fn overview_rejects_invalid_time_range() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let now = common_utils::date_time::now();
    let start_time = format_time(now + time::Duration::hours(2));
    let end_time = format_time(now + time::Duration::hours(1));

    for path in [
        "/overview/transactions",
        "/overview/transactions/rule_account_breakdown",
    ] {
        let response = client
            .get(&format!(
                "{path}?start_time={start_time}&end_time={end_time}"
            ))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(response.status(), 400);
    }

    for path in ["/overview/transactions/time_series"] {
        let response = client
            .get(&format!(
                "{path}?start_time={start_time}&end_time={end_time}&granularity=day"
            ))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(response.status(), 400);
    }
}

#[actix_web::test]
async fn overview_requires_start_time() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let now = common_utils::date_time::now();
    let end_time = format_time(now + time::Duration::hours(1));

    for path in [
        "/overview/transactions",
        "/overview/transactions/rule_account_breakdown",
    ] {
        let response = client
            .get(&format!("{path}?end_time={end_time}"))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(response.status(), 400);
    }
}

#[actix_web::test]
async fn time_series_requires_granularity() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    // Missing granularity => 400 (Granularity is a required query param).
    for path in ["/overview/transactions/time_series"] {
        let response = client
            .get(&format!(
                "{path}?start_time={start_time}&end_time={end_time}"
            ))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(response.status(), 400);
    }
}

#[actix_web::test]
async fn overview_requires_authentication() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let now = common_utils::date_time::now();
    let start_time = format_time(now - time::Duration::hours(1));
    let end_time = format_time(now + time::Duration::hours(1));

    for path in [
        "/overview/transactions",
        "/overview/transactions/rule_account_breakdown",
    ] {
        let response = client
            .get(&format!(
                "{path}?start_time={start_time}&end_time={end_time}"
            ))
            .send()
            .await;
        assert_eq!(response.status(), 401);
    }
}
