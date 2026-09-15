#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::too_many_arguments,
    clippy::panic
)]
use api_models::staging_entry::ApiStagingEntryCoarseStatus;

use std::collections::HashMap;

use crate::{
    api::{
        account_test::create_test_account,
        transaction_test::{create_test_recon_rule, create_test_transaction, DataMismatchType},
        transformation_test::create_transformation_config,
    },
    common::{TestApp, TestClient},
};
use actix_web::test;
use api_models::recon_rule::aging_config::AgingConfigApiRequest;
use api_models::{
    entry::{EntryData, GetEntryResponse},
    staging_entry::{
        ApiLinkableStagingEntryListFiltersV2, GetStagingEntryResponse,
        LinkableStagingEntryListV2Page, ListLinkableStagingEntriesV2Request, NeedsManualReviewData,
        StagingEntryCreateRequest, StagingEntryData,
    },
    transaction::{
        ApiTransactionStatus, DirectTransactionEntry, ExpectedTransactionEntry,
        GetTransactionResponse, ManualReconciliationRequest, SearchExpectation, TransactionEntry,
        TransactionEntryWithData, TransactionRequest,
    },
    transformation::{ValidateEntryDataRequest, ValidateEntryDataResponse},
};
use common_enums::{BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::{
    date_time::TimeRange,
    pagination::{CursorDirection, CursorPagination, SortOrder},
    recon_rule::entry_field::EntryField,
    staging_entry::StagingEntryListSortField,
    transaction::status::{TransactionAmountMismatchData, TransactionMatchedData},
    units::FloatMajorUnit,
};
use common_utils::{date_time, id_type::MerchantId};
use masking::Secret;
use serde_json::json;
use time::{macros::format_description, Date, Duration, PrimitiveDateTime, Time};

// ================================================================================================
// HELPER FUNCTIONS
// ================================================================================================

/// Bootstrap test environment and return basic test data
async fn setup_test_environment() -> (
    TestApp,
    TestClient,
    String,
    MerchantId,
    String,
    String,
    String,
    String,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts
    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Credit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Debit).await;

    // Create recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_replace_entry",
        source_account_id.to_string(),
        target_account_id.to_string(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    (
        app,
        client,
        token,
        merchant_id,
        profile_id,
        source_account_id,
        target_account_id,
        rule_id,
    )
}

/// Test for linkable staging entries API
#[test]
async fn test_get_linkable_staging_entries_for_transaction() {
    let (
        _app,
        client,
        token,
        merchant_id,
        profile_id,
        source_account_id,
        target_account_id,
        rule_id,
    ) = setup_test_environment().await;

    // Create mismatched transaction to test against
    let original_transaction = create_test_transaction(
        &client,
        &token,
        &source_account_id,
        &target_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 10.0 }),
    )
    .await;

    // Create third account for NoRulesFound staging entry
    let third_account_id =
        create_test_account(&client, &token, "Third Account", BalanceDirection::Debit).await;

    let third_transformation_config = create_transformation_config(
        &profile_id,
        &third_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Create only target transformation config (for confirmation mode staging entries)
    let target_transformation_config = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // ===== Step 1: Create 3 staging entries (all Confirmation mode) =====

    // Equal timestamps exercise the primary-ID tie-breaker in effective-at pagination.
    let linkable_effective_at = date_time::now();

    // 1. NoExpectationEntryFound staging entry - different account (third account)
    let no_expectation_staging_response1 = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&StagingEntryCreateRequest {
            account_id: third_account_id.clone(), // Different account
            entry_type: BalanceDirection::Debit,
            processing_mode: StagingEntryProcessingMode::Confirmation,
            amount: FloatMajorUnit::new(40.0), // Amount that will help balance
            currency: Currency::USD,           // Same currency as transaction
            effective_at: linkable_effective_at,
            metadata: Secret::new(json!({"reference_id": "ORDER_12345"})),
            transformation_id: third_transformation_config.transformation_id.clone(),
            transformation_history_id: Some("trans_his_linkable_1".to_string()),
            order_id: "order_test".to_string(),
        })
        .send()
        .await;

    assert_eq!(no_expectation_staging_response1.status(), 201);
    let no_expectation_staging1: GetStagingEntryResponse =
        no_expectation_staging_response1.json().await.unwrap();

    // 2. NoExpectationEntryFound staging entry - same account as mismatched entry (target account)
    let no_expectation_staging_response2 = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&StagingEntryCreateRequest {
            account_id: target_account_id.clone(), // Same account as mismatched entry
            entry_type: BalanceDirection::Debit,
            processing_mode: StagingEntryProcessingMode::Confirmation,
            amount: FloatMajorUnit::new(60.0), // Amount that will help balance (40 + 60 = 100)
            currency: Currency::USD,           // Same currency as transaction
            effective_at: linkable_effective_at,
            metadata: Secret::new(json!({"reference_id": "NON_EXISTENT_ORDER_12345"})), // No matching expectation
            transformation_id: target_transformation_config.transformation_id.clone(),
            transformation_history_id: Some("trans_his_linkable_2".to_string()),
            order_id: "order_test".to_string(),
        })
        .send()
        .await;

    assert_eq!(no_expectation_staging_response2.status(), 201);
    let no_expectation_staging2: GetStagingEntryResponse =
        no_expectation_staging_response2.json().await.unwrap();

    // 3. MissingUniqueField staging entry - same account as mismatched entry (target account)
    let missing_unique_staging_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&StagingEntryCreateRequest {
            account_id: target_account_id.clone(), // Same account as mismatched entry
            entry_type: BalanceDirection::Debit,
            processing_mode: StagingEntryProcessingMode::Confirmation,
            amount: FloatMajorUnit::new(30.0),
            currency: Currency::USD, // Same currency as transaction
            effective_at: linkable_effective_at,
            metadata: Secret::new(json!({"some_other_field": "missing_unique"})), // Missing unique field "metadata.reference_id"
            transformation_id: target_transformation_config.transformation_id.clone(),
            transformation_history_id: None,
            order_id: "order_test".to_string(),
        })
        .send()
        .await;

    assert_eq!(missing_unique_staging_response.status(), 201);
    let missing_unique_staging: GetStagingEntryResponse =
        missing_unique_staging_response.json().await.unwrap();

    // 4. Linkable staging entry with a currency that does not match the transaction.
    let other_currency_staging_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&StagingEntryCreateRequest {
            account_id: third_account_id.clone(),
            entry_type: BalanceDirection::Debit,
            processing_mode: StagingEntryProcessingMode::Confirmation,
            amount: FloatMajorUnit::new(25.0),
            currency: Currency::EUR,
            effective_at: linkable_effective_at,
            metadata: Secret::new(json!({"reference_id": "ORDER_EUR"})),
            transformation_id: third_transformation_config.transformation_id.clone(),
            transformation_history_id: Some("trans_his_linkable_eur".to_string()),
            order_id: "order_eur".to_string(),
        })
        .send()
        .await;

    assert_eq!(other_currency_staging_response.status(), 201);
    let other_currency_staging: GetStagingEntryResponse =
        other_currency_staging_response.json().await.unwrap();

    // ===== Step 2: Process all 4 staging entries through recon engine =====

    // Process NoExpectationEntryFound staging entry 1
    let no_expectation_process_response1 = client
        .post(&format!(
            "/recon_engine/{}/process",
            no_expectation_staging1.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(no_expectation_process_response1.status(), 200);

    // Process NoExpectationEntryFound staging entry 2
    let no_expectation_process_response2 = client
        .post(&format!(
            "/recon_engine/{}/process",
            no_expectation_staging2.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(no_expectation_process_response2.status(), 200);

    // Process MissingUniqueField staging entry
    let missing_unique_process_response = client
        .post(&format!(
            "/recon_engine/{}/process",
            missing_unique_staging.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(missing_unique_process_response.status(), 200);

    let other_currency_process_response = client
        .post(&format!(
            "/recon_engine/{}/process",
            other_currency_staging.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(other_currency_process_response.status(), 200);

    // ===== Step 3: Get all staging entries and find the latest version (max version) of each =====

    // Get all staging entries for the profile
    let all_staging_entries_response = client
        .get("/staging_entries")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(all_staging_entries_response.status(), 200);
    let all_staging_entries: Vec<GetStagingEntryResponse> =
        all_staging_entries_response.json().await.unwrap();

    // Find latest version of NoExpectationEntryFound staging entry 1
    let no_expectation_latest1 = all_staging_entries
        .iter()
        .filter(|e| e.staging_entry_id == no_expectation_staging1.staging_entry_id)
        .max_by_key(|e| e.version)
        .unwrap();

    // Verify it's in NeedsManualReview status with NoExpectationEntryFound
    assert_eq!(
        no_expectation_latest1.status.to_string(),
        "needs_manual_review"
    );
    if let StagingEntryData::NeedsManualReview(manual_review_data) = &no_expectation_latest1.data {
        if let NeedsManualReviewData::NoExpectationEntryFound = manual_review_data {
            // Expected variant
        } else {
            panic!("Expected NoExpectationEntryFound but got: {manual_review_data:?}");
        }
    } else {
        panic!(
            "Expected NeedsManualReview data but got: {:?}",
            no_expectation_latest1.data
        );
    }

    // Find latest version of NoExpectationEntryFound staging entry 2
    let no_expectation_latest2 = all_staging_entries
        .iter()
        .filter(|e| e.staging_entry_id == no_expectation_staging2.staging_entry_id)
        .max_by_key(|e| e.version)
        .unwrap();

    // Verify it's in NeedsManualReview status with NoExpectationEntryFound
    assert_eq!(
        no_expectation_latest2.status.to_string(),
        "needs_manual_review"
    );
    if let StagingEntryData::NeedsManualReview(manual_review_data) = &no_expectation_latest2.data {
        if let NeedsManualReviewData::NoExpectationEntryFound = manual_review_data {
            // Expected variant
        } else {
            panic!("Expected NoExpectationEntryFound but got: {manual_review_data:?}");
        }
    } else {
        panic!(
            "Expected NeedsManualReview data but got: {:?}",
            no_expectation_latest2.data
        );
    }

    // Find latest version of MissingUniqueField staging entry
    let missing_unique_latest = all_staging_entries
        .iter()
        .filter(|e| e.staging_entry_id == missing_unique_staging.staging_entry_id)
        .max_by_key(|e| e.version)
        .unwrap();

    // Verify it's in NeedsManualReview status with MissingUniqueField
    assert_eq!(
        missing_unique_latest.status.to_string(),
        "needs_manual_review"
    );
    if let StagingEntryData::NeedsManualReview(manual_review_data) = &missing_unique_latest.data {
        if let NeedsManualReviewData::MissingUniqueField { missing_field } = manual_review_data {
            assert_eq!(missing_field.to_string(), "metadata.reference_id"); // Should be the unique field from transformation config
        } else {
            panic!("Expected MissingUniqueField but got: {manual_review_data:?}");
        }
    } else {
        panic!(
            "Expected NeedsManualReview data but got: {:?}",
            missing_unique_latest.data
        );
    }

    let other_currency_latest = all_staging_entries
        .iter()
        .filter(|entry| entry.staging_entry_id == other_currency_staging.staging_entry_id)
        .max_by_key(|entry| entry.version)
        .unwrap();
    assert!(matches!(
        &other_currency_latest.data,
        StagingEntryData::NeedsManualReview(NeedsManualReviewData::NoExpectationEntryFound)
    ));

    // ===== Step 4: Test the linkable staging entries List API =====

    let linkable_response = client
        .get(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries",
            original_transaction.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(linkable_response.status(), 200);
    let linkable_staging_entries: Vec<GetStagingEntryResponse> =
        linkable_response.json().await.unwrap();

    // Should return only 2 staging entries (NoExpectationEntryFound entries are linkable, MissingUniqueField is not)
    assert_eq!(linkable_staging_entries.len(), 2);

    // Verify the returned entries are exactly the correct ones by checking IDs
    let returned_ids: Vec<&String> = linkable_staging_entries.iter().map(|e| &e.id).collect();
    assert!(returned_ids.contains(&&no_expectation_latest1.id));
    assert!(returned_ids.contains(&&no_expectation_latest2.id));
    assert!(!returned_ids.contains(&&missing_unique_latest.id)); // This should not be included (not linkable)
    assert!(!returned_ids.contains(&&other_currency_latest.id));

    // Verify all returned entries have matching currency and status
    for entry in &linkable_staging_entries {
        assert_eq!(entry.amount.currency, Currency::USD);
        assert_eq!(entry.status.to_string(), "needs_manual_review");
    }

    // Verify the amounts add up to 100 (40 + 60 = 100)
    let total_amount: f64 = linkable_staging_entries
        .iter()
        .map(|e| e.amount.value.0)
        .sum();
    assert_eq!(total_amount, 100.0);

    // The paginated endpoint applies linkability before taking the page window.
    let first_page_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2::default(),
            cursor_payload: CursorPagination {
                limit: Some(1),
                direction: CursorDirection::Next,
                sort_by: StagingEntryListSortField::EffectiveAt { cursor_value: None },
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(first_page_response.status(), 200);
    let first_page: LinkableStagingEntryListV2Page = first_page_response.json().await.unwrap();
    assert_eq!(first_page.items.len(), 1);
    assert!(first_page.prev_cursor.is_none());
    let next_cursor = first_page
        .next_cursor
        .clone()
        .expect("first page should have a next cursor");

    let second_page_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2::default(),
            cursor_payload: CursorPagination {
                limit: Some(1),
                direction: CursorDirection::Next,
                sort_by: next_cursor,
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(second_page_response.status(), 200);
    let second_page: LinkableStagingEntryListV2Page = second_page_response.json().await.unwrap();
    assert_eq!(second_page.items.len(), 1);
    assert!(second_page.next_cursor.is_none());
    assert!(second_page.prev_cursor.is_some());

    let paginated_ids = first_page
        .items
        .iter()
        .chain(&second_page.items)
        .map(|entry| &entry.id)
        .collect::<Vec<_>>();
    assert!(paginated_ids.contains(&&no_expectation_latest1.id));
    assert!(paginated_ids.contains(&&no_expectation_latest2.id));
    assert!(!paginated_ids.contains(&&missing_unique_latest.id));
    assert!(!paginated_ids.contains(&&other_currency_latest.id));

    let previous_page_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2::default(),
            cursor_payload: CursorPagination {
                limit: Some(1),
                direction: CursorDirection::Previous,
                sort_by: second_page
                    .prev_cursor
                    .expect("second page should have a previous cursor"),
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(previous_page_response.status(), 200);
    let previous_page: LinkableStagingEntryListV2Page =
        previous_page_response.json().await.unwrap();
    assert_eq!(previous_page.items[0].id, first_page.items[0].id);

    let ascending_page_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2::default(),
            cursor_payload: CursorPagination {
                limit: None,
                direction: CursorDirection::Next,
                sort_by: StagingEntryListSortField::Id { id: None },
                order: SortOrder::Asc,
            },
        })
        .send()
        .await;

    assert_eq!(ascending_page_response.status(), 200);
    let ascending_page: LinkableStagingEntryListV2Page =
        ascending_page_response.json().await.unwrap();
    assert_eq!(ascending_page.items.len(), 2);
    let ascending_ids = ascending_page
        .items
        .iter()
        .map(|entry| &entry.id)
        .collect::<Vec<_>>();
    let reversed_descending_ids = paginated_ids.iter().rev().copied().collect::<Vec<_>>();
    assert_eq!(ascending_ids, reversed_descending_ids);

    let filter_cases = [
        (
            ApiLinkableStagingEntryListFiltersV2 {
                entry_type: Some(vec![BalanceDirection::Debit]),
                ..Default::default()
            },
            2,
        ),
        (
            ApiLinkableStagingEntryListFiltersV2 {
                account_ids: Some(vec![target_account_id.clone()]),
                ..Default::default()
            },
            1,
        ),
        (
            ApiLinkableStagingEntryListFiltersV2 {
                transformation_config_ids: Some(vec![target_transformation_config
                    .transformation_id
                    .clone()]),
                ..Default::default()
            },
            1,
        ),
        (
            ApiLinkableStagingEntryListFiltersV2 {
                transformation_history_ids: Some(vec!["trans_his_linkable_2".to_string()]),
                ..Default::default()
            },
            1,
        ),
        (
            ApiLinkableStagingEntryListFiltersV2 {
                time_range: Some(TimeRange {
                    start_time: linkable_effective_at,
                    end_time: Some(linkable_effective_at),
                }),
                ..Default::default()
            },
            2,
        ),
        (
            ApiLinkableStagingEntryListFiltersV2 {
                staging_entry_id: Some(no_expectation_staging2.staging_entry_id.clone()),
                ..Default::default()
            },
            1,
        ),
        (
            ApiLinkableStagingEntryListFiltersV2 {
                order_id: Some("order_test".to_string()),
                ..Default::default()
            },
            2,
        ),
    ];

    for (filters, expected_count) in filter_cases {
        let response = client
            .post(&format!(
                "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
                original_transaction.id
            ))
            .with_auth(&token)
            .json(&ListLinkableStagingEntriesV2Request {
                filters,
                cursor_payload: CursorPagination {
                    limit: Some(10),
                    direction: CursorDirection::Next,
                    sort_by: StagingEntryListSortField::Id { id: None },
                    order: SortOrder::Desc,
                },
            })
            .send()
            .await;

        assert_eq!(response.status(), 200);
        let page: LinkableStagingEntryListV2Page = response.json().await.unwrap();
        assert_eq!(page.items.len(), expected_count);
    }

    let filtered_page_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2 {
                entry_type: Some(vec![BalanceDirection::Debit]),
                account_ids: Some(vec![target_account_id]),
                transformation_config_ids: Some(vec![
                    target_transformation_config.transformation_id,
                ]),
                transformation_history_ids: Some(vec!["trans_his_linkable_2".to_string()]),
                staging_entry_id: Some(no_expectation_staging2.staging_entry_id),
                order_id: Some("order_test".to_string()),
                ..Default::default()
            },
            cursor_payload: CursorPagination {
                limit: Some(10),
                direction: CursorDirection::Next,
                sort_by: StagingEntryListSortField::Id { id: None },
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(filtered_page_response.status(), 200);
    let filtered_page: LinkableStagingEntryListV2Page =
        filtered_page_response.json().await.unwrap();
    assert_eq!(filtered_page.items.len(), 1);
    assert_eq!(filtered_page.items[0].id, no_expectation_latest2.id);

    let empty_page_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2 {
                account_ids: Some(vec!["acc_not_found".to_string()]),
                ..Default::default()
            },
            cursor_payload: CursorPagination {
                limit: Some(10),
                direction: CursorDirection::Next,
                sort_by: StagingEntryListSortField::Id { id: None },
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(empty_page_response.status(), 200);
    let empty_page: LinkableStagingEntryListV2Page = empty_page_response.json().await.unwrap();
    assert!(empty_page.items.is_empty());
    assert!(empty_page.next_cursor.is_none());
    assert!(empty_page.prev_cursor.is_none());

    let invalid_time_range_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2 {
                time_range: Some(TimeRange {
                    start_time: linkable_effective_at,
                    end_time: Some(linkable_effective_at - Duration::days(1)),
                }),
                ..Default::default()
            },
            cursor_payload: CursorPagination {
                limit: Some(10),
                direction: CursorDirection::Next,
                sort_by: StagingEntryListSortField::Id { id: None },
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(invalid_time_range_response.status(), 400);

    let missing_transaction_response = client
        .post("/exception_management/transactions/txn_not_found/linkable_staging_entries/v2/list")
        .with_auth(&token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2::default(),
            cursor_payload: CursorPagination {
                limit: Some(10),
                direction: CursorDirection::Next,
                sort_by: StagingEntryListSortField::Id { id: None },
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(missing_transaction_response.status(), 404);

    let (_, _, other_profile_token) = client.bootstrap("Other Merchant", "Other Profile").await;
    let cross_profile_response = client
        .post(&format!(
            "/exception_management/transactions/{}/linkable_staging_entries/v2/list",
            original_transaction.id
        ))
        .with_auth(&other_profile_token)
        .json(&ListLinkableStagingEntriesV2Request {
            filters: ApiLinkableStagingEntryListFiltersV2::default(),
            cursor_payload: CursorPagination {
                limit: Some(10),
                direction: CursorDirection::Next,
                sort_by: StagingEntryListSortField::Id { id: None },
                order: SortOrder::Desc,
            },
        })
        .send()
        .await;

    assert_eq!(cross_profile_response.status(), 404);
}

#[test]
async fn test_manual_reconciliation_success() {
    let (
        _app,
        client,
        token,
        merchant_id,
        profile_id,
        source_account_id,
        target_account_id,
        rule_id,
    ) = setup_test_environment().await;

    let mismatched_transaction = create_test_transaction(
        &client,
        &token,
        &source_account_id,
        &target_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 10.0 }),
    )
    .await;

    assert_eq!(
        mismatched_transaction.status,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
    );

    let partial_response = client
        .post(&format!(
            "/exception_management/transactions/{}/manual_reconciliation",
            mismatched_transaction.id
        ))
        .with_auth(&token)
        .json(&ManualReconciliationRequest {
            transaction_entries: vec![
                TransactionEntryWithData {
                    account_id: source_account_id.clone(),
                    entry_type: BalanceDirection::Debit,
                    amount: FloatMajorUnit::new(100.0),
                    currency: Currency::USD,
                    effective_at: date_time::now(),
                    metadata: Secret::new(json!({"test": "data"})),
                    data: EntryData::Pending,
                    staging_entry_id: None,
                    order_id: "order_test".to_string(),
                },
                TransactionEntryWithData {
                    account_id: target_account_id.clone(),
                    entry_type: BalanceDirection::Credit,
                    amount: FloatMajorUnit::new(80.0),
                    currency: Currency::USD,
                    effective_at: date_time::now(),
                    metadata: Secret::new(json!({"test": "data"})),
                    data: EntryData::Pending,
                    staging_entry_id: None,
                    order_id: "order_test".to_string(),
                },
            ],
            reason: Some("Manual reconciliation test".to_string()),
        })
        .send()
        .await;
    assert_eq!(partial_response.status(), 200);
    let partial_transaction: GetTransactionResponse = partial_response.json().await.unwrap();
    assert_eq!(
        partial_transaction.status,
        ApiTransactionStatus::PartiallyReconciled
    );

    let transformation = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Create staging entry
    let staging_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&StagingEntryCreateRequest {
            account_id: target_account_id.to_string(),
            entry_type: BalanceDirection::Credit,
            processing_mode: StagingEntryProcessingMode::Confirmation,
            amount: FloatMajorUnit::new(20.0),
            currency: Currency::USD,
            effective_at: date_time::now(),
            metadata: Secret::new(json!({
                "reference_id": "ORDER-EXCEPTION-TEST",
                "payment_method": "card"
            })),
            transformation_id: transformation.transformation_id,
            transformation_history_id: None,
            order_id: "order_test".to_string(),
        })
        .send()
        .await;

    assert_eq!(staging_response.status(), 201);
    let target_staging_entry: GetStagingEntryResponse = staging_response.json().await.unwrap();

    let response = client
        .post(&format!(
            "/recon_engine/{}/process",
            target_staging_entry.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);

    // Fetch all staging entries
    let staging_entries_response = client
        .get("/staging_entries")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(staging_entries_response.status(), 200);
    let staging_entries: Vec<GetStagingEntryResponse> =
        staging_entries_response.json().await.unwrap();

    let review_staging_entry = staging_entries
        .iter()
        .filter(|entry| entry.staging_entry_id == target_staging_entry.staging_entry_id)
        .max_by_key(|entry| entry.version)
        .unwrap();

    // Partially Reconciled to Posted
    let partial_response = client
        .post(&format!(
            "/exception_management/transactions/{}/manual_reconciliation",
            partial_transaction.id
        ))
        .with_auth(&token)
        .json(&ManualReconciliationRequest {
            transaction_entries: vec![
                TransactionEntryWithData {
                    account_id: source_account_id.clone(),
                    entry_type: BalanceDirection::Debit,
                    amount: FloatMajorUnit::new(100.0),
                    currency: Currency::USD,
                    effective_at: date_time::now(),
                    metadata: Secret::new(json!({"test": "data"})),
                    data: EntryData::Pending,
                    staging_entry_id: None,
                    order_id: "order_test".to_string(),
                },
                TransactionEntryWithData {
                    account_id: target_account_id.clone(),
                    entry_type: BalanceDirection::Credit,
                    amount: FloatMajorUnit::new(80.0),
                    currency: Currency::USD,
                    effective_at: date_time::now(),
                    metadata: Secret::new(json!({"test": "data"})),
                    data: EntryData::Pending,
                    staging_entry_id: None,
                    order_id: "order_test".to_string(),
                },
                TransactionEntryWithData {
                    account_id: target_account_id.clone(),
                    entry_type: BalanceDirection::Credit,
                    amount: FloatMajorUnit::new(20.0),
                    currency: Currency::USD,
                    effective_at: date_time::now(),
                    metadata: Secret::new(json!({"test": "data"})),
                    data: EntryData::Pending,
                    staging_entry_id: Some(review_staging_entry.id.clone()),
                    order_id: "order_test".to_string(),
                },
            ],
            reason: Some("Manual reconciliation test".to_string()),
        })
        .send()
        .await;
    assert_eq!(partial_response.status(), 200);
    let partial_transaction: GetTransactionResponse = partial_response.json().await.unwrap();
    assert_eq!(
        partial_transaction.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
    );

    let staging_entries_response = client
        .get("/staging_entries")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(staging_entries_response.status(), 200);
    let staging_entries: Vec<GetStagingEntryResponse> =
        staging_entries_response.json().await.unwrap();

    let final_staging_entry = staging_entries
        .iter()
        .filter(|entry| entry.staging_entry_id == target_staging_entry.staging_entry_id)
        .max_by_key(|entry| entry.version)
        .unwrap();

    assert_eq!(
        final_staging_entry.status,
        ApiStagingEntryCoarseStatus::Processed
    );
}

#[test]
async fn test_manual_reconciliation_preserves_expected_entries_when_balanced() {
    let (
        _app,
        client,
        token,
        _merchant_id,
        _profile_id,
        source_account_id,
        target_account_id,
        rule_id,
    ) = setup_test_environment().await;

    // Create an Expected transaction first (not mismatched)
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: source_account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: date_time::now(),
                metadata: Secret::new(json!({"description": "Direct credit entry"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Expected(ExpectedTransactionEntry {
                account_id: target_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: date_time::now(),
                metadata: Secret::new(json!({"description": "Expected debit entry"})),
                search_expectation: SearchExpectation {
                    rule_id: rule_id.clone(),
                    source_value: "test_order_expected_preservation".to_string(),
                    target_key: "metadata.reference_id".parse().unwrap(),
                },
                order_id: "order_test".to_string(),
            }),
        ],
        rule_id: rule_id.clone(),
    };

    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let original_transaction: GetTransactionResponse = response.json().await.unwrap();
    assert_eq!(original_transaction.status, ApiTransactionStatus::Expected);

    // Perform manual reconciliation with balanced entries but include Expected entry
    let manual_recon_request = ManualReconciliationRequest {
        transaction_entries: vec![
            TransactionEntryWithData {
                account_id: source_account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: date_time::now(),
                metadata: Secret::new(json!({"description": "Pending credit entry"})),
                data: EntryData::Pending, // This should remain Pending since Expected is present
                staging_entry_id: None,
                order_id: "order_test".to_string(),
            },
            TransactionEntryWithData {
                account_id: target_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: date_time::now(),
                metadata: Secret::new(json!({"description": "Expected debit entry"})),
                data: EntryData::Expected, // This should remain Expected
                staging_entry_id: None,
                order_id: "order_test".to_string(),
            },
        ],
        reason: Some("Manual reconciliation with Expected entry preservation test".to_string()),
    };

    let response = client
        .post(&format!(
            "/exception_management/transactions/{}/manual_reconciliation",
            original_transaction.id
        ))
        .with_auth(&token)
        .json(&manual_recon_request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let reconciled_transaction: GetTransactionResponse = response.json().await.unwrap();

    // Verify transaction is PartiallyReconciled despite being balanced (due to Expected entry)
    assert_eq!(
        reconciled_transaction.status,
        ApiTransactionStatus::PartiallyReconciled
    );

    // Verify the transaction is balanced
    assert_eq!(
        reconciled_transaction.credit_amount.value,
        FloatMajorUnit::new(100.0)
    );
    assert_eq!(
        reconciled_transaction.debit_amount.value,
        FloatMajorUnit::new(100.0)
    );

    // Verify version incremented
    assert_eq!(
        reconciled_transaction.version,
        original_transaction.version + 1
    );

    // Fetch individual entries to verify their statuses
    let credit_entry_response = client
        .get(&format!("/entries/{}", reconciled_transaction.entry_id[0]))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(credit_entry_response.status(), 200);
    let credit_entry: GetEntryResponse = credit_entry_response.json().await.unwrap();

    let debit_entry_response = client
        .get(&format!("/entries/{}", reconciled_transaction.entry_id[1]))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(debit_entry_response.status(), 200);
    let debit_entry: GetEntryResponse = debit_entry_response.json().await.unwrap();

    // Verify entry statuses: Pending stays Pending, Expected stays Expected
    match &credit_entry.data {
        EntryData::Pending => {
            // This is correct - the Pending entry should remain Pending when Expected entries are present
        }
        _ => panic!(
            "Expected Pending entry to remain Pending when Expected entries are present, got: {:?}",
            credit_entry.data
        ),
    }

    match &debit_entry.data {
        EntryData::Expected => {
            // This is correct - the Expected entry should remain Expected
        }
        _ => panic!(
            "Expected Expected entry to remain Expected, got: {:?}",
            debit_entry.data
        ),
    }
}

#[test]
async fn test_validate_raw_entry_data_success() {
    let (
        _app,
        client,
        token,
        merchant_id,
        profile_id,
        source_account_id,
        _target_account_id,
        _rule_id,
    ) = setup_test_environment().await;

    let transformation_config = create_transformation_config(
        &profile_id,
        &source_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let response = client
        .post("/exception_management/validate_entry_data")
        .with_auth(&token)
        .json(&ValidateEntryDataRequest {
            transformation_id: transformation_config.transformation_id.clone(),
            raw_entry_data: HashMap::from([
                (EntryField::Currency.to_string(), Some("USD".to_string())),
                (EntryField::Amount.to_string(), Some("100".to_string())),
                (
                    EntryField::EffectiveAt.to_string(),
                    Some("01-01-2024".to_string()),
                ),
                (
                    EntryField::OrderId.to_string(),
                    Some("order_id".to_string()),
                ),
                (
                    EntryField::EntryType.to_string(),
                    Some("CREDIT".to_string()),
                ),
            ]),
        })
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let response_body: ValidateEntryDataResponse = response.json().await.unwrap();
    match response_body {
        ValidateEntryDataResponse::Success(validated_entry_data) => {
            assert_eq!(validated_entry_data.entry_type, BalanceDirection::Credit);
            assert_eq!(validated_entry_data.amount.get_amount_as_i64(), 100);
            assert_eq!(validated_entry_data.currency, Currency::USD);
            assert_eq!(
                validated_entry_data.effective_at,
                PrimitiveDateTime::new(
                    Date::parse("01-01-2024", &format_description!("[day]-[month]-[year]"))
                        .unwrap(),
                    Time::MIDNIGHT
                )
            );
            assert_eq!(validated_entry_data.order_id, "order_id");
        }
        ValidateEntryDataResponse::Failure(row_error) => {
            panic!("Expected success, got failure: {row_error:?}");
        }
    }
}

#[test]
async fn test_validate_raw_entry_data_failure() {
    let (
        _app,
        client,
        token,
        merchant_id,
        profile_id,
        source_account_id,
        _target_account_id,
        _rule_id,
    ) = setup_test_environment().await;

    let transformation_config = create_transformation_config(
        &profile_id,
        &source_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let response = client
        .post("/exception_management/validate_entry_data")
        .with_auth(&token)
        .json(&ValidateEntryDataRequest {
            transformation_id: transformation_config.transformation_id.clone(),
            raw_entry_data: HashMap::from([
                ("currency".to_string(), Some("USD".to_string())),
                ("effective_at".to_string(), Some("01-01-2024".to_string())),
                ("order_id".to_string(), Some("order_id".to_string())),
                ("balance_direction".to_string(), Some("CREDIT".to_string())),
            ]),
        })
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let response_body: ValidateEntryDataResponse = response.json().await.unwrap();
    match response_body {
        ValidateEntryDataResponse::Success(_) => {
            panic!("Expected failure, got success");
        }
        ValidateEntryDataResponse::Failure(row_error) => {
            assert_eq!(row_error.errors.len(), 1);
            assert!(row_error.errors.contains_key("amount"));
        }
    }
}
