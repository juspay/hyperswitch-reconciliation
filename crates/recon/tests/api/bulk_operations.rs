use std::collections::HashMap;

use crate::{
    api::{
        account_test::create_test_account,
        ingestion_test::create_ingestion_config,
        staging_entry_test::create_test_staging_entry,
        transaction_test::{create_test_recon_rule, create_test_transaction, DataMismatchType},
        transformation_test::create_transformation_config,
    },
    common::TestApp,
};
use api_models::{
    bulk_operations::{
        BulkOperationResultItem, BulkResultStatus, StagingEntryBulkAction,
        StagingEntryBulkOperationRequest, StagingEntryBulkSelection, TransactionBulkAction,
        TransactionBulkOperationRequest, TransactionBulkSelection,
    },
    recon_rule::aging_config::AgingConfigApiRequest,
    staging_entry::ApiListStagingEntriesFilters,
    transaction::{
        ApiTransactionStatus, DirectTransactionEntry, ForceReconcileRequest,
        GetTransactionResponse, TransactionEntry, TransactionRequest,
        TransactionWithEntriesResponse,
    },
};
use common_enums::{BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::{
    date_time::TimeRange,
    staging_entry::{PendingData, PendingDataV1, StagingEntryData, StagingEntryDataV2},
    transaction::status::TransactionMatchedData,
    units::FloatMajorUnit,
};
use masking::Secret;
use pretty_assertions::assert_eq;
use serde_json::json;
use time::Duration;

fn map_results_by_entity_id(
    results: Vec<BulkOperationResultItem>,
) -> HashMap<String, BulkOperationResultItem> {
    results
        .into_iter()
        .map(|result| (result.primary_id.clone(), result))
        .collect()
}

// ============================================================================
// STAGING ENTRY BULK OPERATIONS - IDS SELECTION
// ============================================================================

#[actix_web::test]
async fn staging_entry_bulk_void_ids_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let account_id = create_test_account(
        &client,
        &token,
        "Bulk Staging Account",
        BalanceDirection::Credit,
    )
    .await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let transformation_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config_response.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let staging_entry_1 = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id.clone(),
    )
    .await;

    let staging_entry_2 = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id,
    )
    .await;

    let bulk_request = StagingEntryBulkOperationRequest {
        action: StagingEntryBulkAction::Void {
            reason: Some("Bulk void test".to_string()),
        },
        selection: StagingEntryBulkSelection::Ids {
            ids: vec![staging_entry_1.id.clone(), staging_entry_2.id.clone()],
        },
    };

    let bulk_response = client
        .post("/staging_entries/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 2);

    let results_by_id = map_results_by_entity_id(results);
    assert_eq!(
        results_by_id.get(&staging_entry_1.id).unwrap().status,
        BulkResultStatus::Success {
            logical_id: staging_entry_1.staging_entry_id.clone()
        }
    );
    assert_eq!(
        results_by_id.get(&staging_entry_2.id).unwrap().status,
        BulkResultStatus::Success {
            logical_id: staging_entry_2.staging_entry_id.clone()
        }
    );
}

#[actix_web::test]
async fn staging_entry_bulk_void_ids_missing() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let account_id = create_test_account(
        &client,
        &token,
        "Bulk Staging Account",
        BalanceDirection::Credit,
    )
    .await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let transformation_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config_response.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id,
    )
    .await;

    let bulk_request = StagingEntryBulkOperationRequest {
        action: StagingEntryBulkAction::Void {
            reason: Some("Bulk void with missing".to_string()),
        },
        selection: StagingEntryBulkSelection::Ids {
            ids: vec![
                staging_entry.id.clone(),
                "non_existent_id_1".to_string(),
                "non_existent_id_2".to_string(),
            ],
        },
    };

    let bulk_response = client
        .post("/staging_entries/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 3);

    let results_by_id = map_results_by_entity_id(results);
    assert_eq!(
        results_by_id.get(&staging_entry.id).unwrap().status,
        BulkResultStatus::Success {
            logical_id: staging_entry.staging_entry_id.clone()
        }
    );
    assert_eq!(
        results_by_id.get("non_existent_id_1").unwrap().status,
        BulkResultStatus::Failed {
            error: "Staging Entry not found".to_string(),
            logical_id: None,
        }
    );
    assert_eq!(
        results_by_id.get("non_existent_id_2").unwrap().status,
        BulkResultStatus::Failed {
            error: "Staging Entry not found".to_string(),
            logical_id: None,
        }
    );
}

#[actix_web::test]
async fn staging_entry_bulk_void_ids_duplicates() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let account_id = create_test_account(
        &client,
        &token,
        "Bulk Staging Account",
        BalanceDirection::Credit,
    )
    .await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let transformation_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config_response.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id,
    )
    .await;

    // Request with duplicate IDs
    let bulk_request = StagingEntryBulkOperationRequest {
        action: StagingEntryBulkAction::Void {
            reason: Some("Duplicate IDs test".to_string()),
        },
        selection: StagingEntryBulkSelection::Ids {
            ids: vec![
                staging_entry.id.clone(),
                staging_entry.id.clone(),
                staging_entry.id.clone(),
            ],
        },
    };

    let bulk_response = client
        .post("/staging_entries/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    // Should deduplicate to single result
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].status,
        BulkResultStatus::Success {
            logical_id: staging_entry.staging_entry_id.clone()
        }
    );
}

#[actix_web::test]
async fn staging_entry_bulk_void_ids_non_voidable() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let account_id = create_test_account(
        &client,
        &token,
        "Bulk Staging Account",
        BalanceDirection::Credit,
    )
    .await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let transformation_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config_response.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id,
    )
    .await;

    let first_void_request = StagingEntryBulkOperationRequest {
        action: StagingEntryBulkAction::Void {
            reason: Some("First void".to_string()),
        },
        selection: StagingEntryBulkSelection::Ids {
            ids: vec![staging_entry.id.clone()],
        },
    };

    let first_response = client
        .post("/staging_entries/bulk_operations")
        .with_auth(&token)
        .json(&first_void_request)
        .send()
        .await;
    assert_eq!(first_response.status(), 200);
    let first_results: Vec<BulkOperationResultItem> = first_response.json().await.unwrap();
    assert_eq!(first_results.len(), 1);
    assert_eq!(
        first_results[0].status,
        BulkResultStatus::Success {
            logical_id: staging_entry.staging_entry_id.clone()
        }
    );

    let second_void_request = StagingEntryBulkOperationRequest {
        action: StagingEntryBulkAction::Void {
            reason: Some("Second void attempt".to_string()),
        },
        selection: StagingEntryBulkSelection::Ids {
            ids: vec![staging_entry.id.clone()],
        },
    };

    let second_response = client
        .post("/staging_entries/bulk_operations")
        .with_auth(&token)
        .json(&second_void_request)
        .send()
        .await;
    assert_eq!(second_response.status(), 200);

    let second_results: Vec<BulkOperationResultItem> = second_response.json().await.unwrap();
    assert_eq!(second_results.len(), 1);

    match &second_results[0].status {
        BulkResultStatus::Skipped { reason, logical_id } => {
            assert!(reason.contains("not voidable"));
            assert_eq!(logical_id, &staging_entry.staging_entry_id);
        }
        status => panic!("Expected skipped status, got: {status:?}"),
    }
}

#[actix_web::test]
async fn staging_entry_bulk_void_ids_profile_isolation() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id_1, profile_id_1, token_1) = client.bootstrap("Merchant 1", "Profile 1").await;
    let (merchant_id_2, profile_id_2, token_2) = client.bootstrap("Merchant 2", "Profile 2").await;

    let account_id_1 =
        create_test_account(&client, &token_1, "Account 1", BalanceDirection::Credit).await;
    let account_id_2 =
        create_test_account(&client, &token_2, "Account 2", BalanceDirection::Credit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id_1, &profile_id_1, &merchant_id_1, &client).await;

    let transformation_config = create_transformation_config(
        &profile_id_1,
        &account_id_1,
        Some(&ingestion_config_response.ingestion_id),
        &merchant_id_1,
        &token_1,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let entry_1 = create_test_staging_entry(
        &client,
        &token_1,
        &profile_id_1,
        &account_id_1,
        &merchant_id_1,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id.clone(),
    )
    .await;

    let entry_2 = create_test_staging_entry(
        &client,
        &token_2,
        &profile_id_2,
        &account_id_2,
        &merchant_id_2,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id,
    )
    .await;

    let bulk_request = StagingEntryBulkOperationRequest {
        action: StagingEntryBulkAction::Void {
            reason: Some("Profile isolation test".to_string()),
        },
        selection: StagingEntryBulkSelection::Ids {
            ids: vec![entry_1.id.clone(), entry_2.id.clone()],
        },
    };

    let bulk_response = client
        .post("/staging_entries/bulk_operations")
        .with_auth(&token_1)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 2);

    let results_by_id = map_results_by_entity_id(results);
    assert_eq!(
        results_by_id.get(&entry_1.id).unwrap().status,
        BulkResultStatus::Success {
            logical_id: entry_1.staging_entry_id.clone()
        }
    );
    assert_eq!(
        results_by_id.get(&entry_2.id).unwrap().status,
        BulkResultStatus::Failed {
            error: "Staging Entry not found".to_string(),
            logical_id: None,
        }
    );
}

// ============================================================================
// STAGING ENTRY BULK OPERATIONS - FILTERS SELECTION
// ============================================================================

#[actix_web::test]
async fn staging_entry_bulk_void_filters_by_status() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let account_id = create_test_account(
        &client,
        &token,
        "Bulk Staging Account",
        BalanceDirection::Credit,
    )
    .await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let transformation_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config_response.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Create entries with different statuses by voiding one
    let entry_1 = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id.clone(),
    )
    .await;

    let _entry_2 = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        ))),
        transformation_config.transformation_id,
    )
    .await;

    // Void entries using filters (all entries in account)
    let bulk_request = StagingEntryBulkOperationRequest {
        action: StagingEntryBulkAction::Void {
            reason: Some("Void filtered entries".to_string()),
        },
        selection: StagingEntryBulkSelection::Filters {
            filters: ApiListStagingEntriesFilters {
                status: None,
                entry_type: None,
                account_id: Some(account_id),
                transformation_history_id: None,
                currency: None,
                time_range: None,
                staging_entry_id: None,
            },
        },
    };

    let bulk_response = client
        .post("/staging_entries/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    // Should void both pending entries
    assert!(results.len() >= 2);

    let results_by_id = map_results_by_entity_id(results);
    assert_eq!(
        results_by_id.get(&entry_1.id).unwrap().status,
        BulkResultStatus::Success {
            logical_id: entry_1.staging_entry_id.clone()
        }
    );
}

// ============================================================================
// TRANSACTION BULK OPERATIONS - IDS SELECTION
// ============================================================================

#[actix_web::test]
async fn transaction_bulk_void_ids_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_, _, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let debit_account_id =
        create_test_account(&client, &token, "Bulk Txn Debit", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Bulk Txn Credit", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_txn_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id,
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(50.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "debit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "debit_order".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id,
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(50.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "credit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "credit_order".to_string(),
            }),
        ],
        rule_id,
    };

    let transaction_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;
    assert_eq!(transaction_response.status(), 201);
    let transaction: GetTransactionResponse = transaction_response.json().await.unwrap();

    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::Void {
            reason: Some("Bulk void test".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![transaction.id.clone()],
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].status,
        BulkResultStatus::Success {
            logical_id: transaction.transaction_id.clone()
        }
    );
}

#[actix_web::test]
async fn transaction_bulk_void_ids_missing() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_, _, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let debit_account_id =
        create_test_account(&client, &token, "Bulk Txn Debit", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Bulk Txn Credit", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_txn_missing_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id,
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(50.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "debit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "existing_order".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id,
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(50.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "credit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "existing_order_2".to_string(),
            }),
        ],
        rule_id,
    };

    let transaction_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;
    assert_eq!(transaction_response.status(), 201);
    let transaction: GetTransactionResponse = transaction_response.json().await.unwrap();

    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::Void {
            reason: Some("Bulk void with missing".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![
                transaction.id.clone(),
                "txn_missing_1".to_string(),
                "txn_missing_2".to_string(),
            ],
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 3);

    let results_by_id = map_results_by_entity_id(results);
    assert_eq!(
        results_by_id.get(&transaction.id).unwrap().status,
        BulkResultStatus::Success {
            logical_id: transaction.transaction_id.clone()
        }
    );
    assert_eq!(
        results_by_id.get("txn_missing_1").unwrap().status,
        BulkResultStatus::Failed {
            error: "Transaction not found".to_string(),
            logical_id: None,
        }
    );
    assert_eq!(
        results_by_id.get("txn_missing_2").unwrap().status,
        BulkResultStatus::Failed {
            error: "Transaction not found".to_string(),
            logical_id: None,
        }
    );
}

#[actix_web::test]
async fn transaction_bulk_void_ids_duplicates() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_, _, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let debit_account_id =
        create_test_account(&client, &token, "Bulk Txn Debit", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Bulk Txn Credit", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_txn_dup_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id,
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(50.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "debit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "dup_order".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id,
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(50.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "credit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "dup_order_2".to_string(),
            }),
        ],
        rule_id,
    };

    let transaction_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;
    assert_eq!(transaction_response.status(), 201);
    let transaction: GetTransactionResponse = transaction_response.json().await.unwrap();

    // Request with duplicate transaction IDs
    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::Void {
            reason: Some("Duplicate IDs test".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![
                transaction.id.clone(),
                transaction.id.clone(),
                transaction.id.clone(),
            ],
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    // Should deduplicate to single result
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].status,
        BulkResultStatus::Success {
            logical_id: transaction.transaction_id.clone()
        }
    );
}

#[actix_web::test]
async fn transaction_bulk_void_ids_non_voidable() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_, _, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let debit_account_id = create_test_account(
        &client,
        &token,
        "Bulk Txn Debit NonVoid",
        BalanceDirection::Debit,
    )
    .await;
    let credit_account_id = create_test_account(
        &client,
        &token,
        "Bulk Txn Credit NonVoid",
        BalanceDirection::Credit,
    )
    .await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_txn_nonvoid_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id,
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(75.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "debit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "nonvoid_order".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id,
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(75.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"side": "credit"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "nonvoid_order_2".to_string(),
            }),
        ],
        rule_id,
    };

    let transaction_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;
    assert_eq!(transaction_response.status(), 201);
    let transaction: GetTransactionResponse = transaction_response.json().await.unwrap();

    let first_void_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::Void {
            reason: Some("First void".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![transaction.id.clone()],
        },
    };

    let first_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&first_void_request)
        .send()
        .await;
    assert_eq!(first_response.status(), 200);
    let first_results: Vec<BulkOperationResultItem> = first_response.json().await.unwrap();
    assert_eq!(first_results.len(), 1);
    assert_eq!(
        first_results[0].status,
        BulkResultStatus::Success {
            logical_id: transaction.transaction_id.clone()
        }
    );

    let second_void_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::Void {
            reason: Some("Second void attempt".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![transaction.id.clone()],
        },
    };

    let second_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&second_void_request)
        .send()
        .await;
    assert_eq!(second_response.status(), 200);

    let second_results: Vec<BulkOperationResultItem> = second_response.json().await.unwrap();
    assert_eq!(second_results.len(), 1);

    match &second_results[0].status {
        BulkResultStatus::Skipped { reason, logical_id } => {
            assert!(reason.contains("not voidable"));
            assert_eq!(logical_id, &transaction.transaction_id);
        }
        status => panic!("Expected skipped status, got: {status:?}"),
    }
}

// ============================================================================
// TRANSACTION BULK OPERATIONS - FILTERS SELECTION
// ============================================================================

use api_models::transaction::ApiTransactionAccountFilters;

#[actix_web::test]
async fn transaction_bulk_void_filters_by_status() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_, _, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let debit_account_id =
        create_test_account(&client, &token, "Bulk Txn Debit", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Bulk Txn Credit", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_txn_filter_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create two transactions
    let transaction_request_1 = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"txn": "1"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "filter_order_1".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"txn": "1"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "filter_order_1_credit".to_string(),
            }),
        ],
        rule_id: rule_id.clone(),
    };

    let transaction_request_2 = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id,
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(200.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"txn": "2"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "filter_order_2".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id,
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(200.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"txn": "2"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "filter_order_2_credit".to_string(),
            }),
        ],
        rule_id,
    };

    let response_1 = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request_1)
        .send()
        .await;
    assert_eq!(response_1.status(), 201);

    let response_2 = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request_2)
        .send()
        .await;
    assert_eq!(response_2.status(), 201);

    // Void using filters (all posted transactions)
    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::Void {
            reason: Some("Void by filter".to_string()),
        },
        selection: TransactionBulkSelection::Filters {
            filters: ApiTransactionAccountFilters {
                transaction_id: None,
                status: None,
                time_range: Some(TimeRange {
                    start_time: common_utils::date_time::now() - Duration::hours(1),
                    end_time: None,
                }),
                rule_id: None,
                credit_account: None,
                debit_account: None,
            },
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    // Should void both transactions
    assert!(results.len() >= 2);
}

// ============================================================================
// TRANSACTION BULK OPERATIONS - MANUAL POST
// ============================================================================

#[actix_web::test]
async fn transaction_bulk_manual_post_ids_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let source_account_id =
        create_test_account(&client, &token, "Bulk MP Source", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Bulk MP Target", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_mp_rule",
        source_account_id.clone(),
        target_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create a DataMismatch transaction
    let mismatched_transaction = create_test_transaction(
        &client,
        &token,
        &source_account_id,
        &target_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::DataMismatch,
        Some(DataMismatchType::Metadata),
    )
    .await;

    // Force reconcile to get MatchedForce status
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Approved for manual posting".to_string()),
    };

    let force_response = client
        .put(&format!(
            "/exception_management/transactions/{}/force_reconcile",
            mismatched_transaction.id
        ))
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;
    assert_eq!(force_response.status(), 200);
    let matched_transaction: GetTransactionResponse = force_response.json().await.unwrap();
    assert_eq!(
        matched_transaction.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Force)
    );

    // Perform bulk manual post
    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::ManualPost {
            reason: Some("Approved for posting".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![matched_transaction.id.clone()],
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].status,
        BulkResultStatus::Success {
            logical_id: matched_transaction.transaction_id.clone()
        }
    );

    let get_response = client
        .get(&format!(
            "/transactions?transaction_id={}",
            matched_transaction.transaction_id
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(get_response.status(), 200);
    let transactions: Vec<TransactionWithEntriesResponse> = get_response.json().await.unwrap();
    assert!(!transactions.is_empty());

    let posted_transaction = transactions
        .iter()
        .find(|t| t.status != ApiTransactionStatus::Archived)
        .expect("Should have non-archived transaction");

    assert_eq!(
        posted_transaction.status,
        ApiTransactionStatus::Posted(
            common_types::transaction::status::TransactionPostedData::Manual
        )
    );

    for entry in &posted_transaction.entries {
        assert_eq!(entry.status, common_enums::EntryStatus::Posted);
    }
}

#[actix_web::test]
async fn transaction_bulk_manual_post_ids_missing() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let source_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP Missing Source",
        BalanceDirection::Debit,
    )
    .await;
    let target_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP Missing Target",
        BalanceDirection::Credit,
    )
    .await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_mp_missing_rule",
        source_account_id.clone(),
        target_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create a DataMismatch transaction
    let mismatched_transaction = create_test_transaction(
        &client,
        &token,
        &source_account_id,
        &target_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::DataMismatch,
        Some(DataMismatchType::Metadata),
    )
    .await;

    // Force reconcile to get MatchedForce status
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Approved".to_string()),
    };

    let force_response = client
        .put(&format!(
            "/exception_management/transactions/{}/force_reconcile",
            mismatched_transaction.id
        ))
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;
    assert_eq!(force_response.status(), 200);
    let matched_transaction: GetTransactionResponse = force_response.json().await.unwrap();

    // Bulk manual post with mix of real + fake IDs
    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::ManualPost {
            reason: Some("Approved".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![
                matched_transaction.id.clone(),
                "txn_missing_1".to_string(),
                "txn_missing_2".to_string(),
            ],
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 3);

    let results_by_id = map_results_by_entity_id(results);
    assert_eq!(
        results_by_id.get(&matched_transaction.id).unwrap().status,
        BulkResultStatus::Success {
            logical_id: matched_transaction.transaction_id.clone()
        }
    );
    assert_eq!(
        results_by_id.get("txn_missing_1").unwrap().status,
        BulkResultStatus::Failed {
            error: "Transaction not found".to_string(),
            logical_id: None,
        }
    );
    assert_eq!(
        results_by_id.get("txn_missing_2").unwrap().status,
        BulkResultStatus::Failed {
            error: "Transaction not found".to_string(),
            logical_id: None,
        }
    );
}

#[actix_web::test]
async fn transaction_bulk_manual_post_ids_non_postable() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let debit_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP NonPost Debit",
        BalanceDirection::Debit,
    )
    .await;
    let credit_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP NonPost Credit",
        BalanceDirection::Credit,
    )
    .await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_mp_nonpost_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create an Expected transaction (not eligible for manual post)
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id,
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "expected"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "expected_order".to_string(),
            }),
            TransactionEntry::Expected(api_models::transaction::ExpectedTransactionEntry {
                account_id: credit_account_id,
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "expected"})),
                search_expectation: api_models::transaction::SearchExpectation {
                    rule_id: rule_id.clone(),
                    source_value: "payment_expected".to_string(),
                    target_key: "metadata.transaction_id".parse().unwrap(),
                },
                order_id: "expected_order_credit".to_string(),
            }),
        ],
        rule_id: rule_id.clone(),
    };

    let transaction_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;
    assert_eq!(transaction_response.status(), 201);
    let transaction: GetTransactionResponse = transaction_response.json().await.unwrap();

    // Verify it's Expected status
    assert_eq!(transaction.status, ApiTransactionStatus::Expected);

    // Try bulk manual post on Expected transaction
    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::ManualPost {
            reason: Some("Attempt to post expected".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![transaction.id.clone()],
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 1);

    match &results[0].status {
        BulkResultStatus::Skipped { reason, logical_id } => {
            assert!(reason.contains("not eligible for manual posting"));
            assert_eq!(logical_id, &transaction.transaction_id);
        }
        status => panic!("Expected skipped status, got: {status:?}"),
    }
}

#[actix_web::test]
async fn transaction_bulk_manual_post_ids_already_posted() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let source_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP Already Source",
        BalanceDirection::Debit,
    )
    .await;
    let target_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP Already Target",
        BalanceDirection::Credit,
    )
    .await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_mp_already_rule",
        source_account_id.clone(),
        target_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create a DataMismatch transaction
    let mismatched_transaction = create_test_transaction(
        &client,
        &token,
        &source_account_id,
        &target_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::DataMismatch,
        Some(DataMismatchType::Metadata),
    )
    .await;

    // Force reconcile to get MatchedForce status
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Approved".to_string()),
    };

    let force_response = client
        .put(&format!(
            "/exception_management/transactions/{}/force_reconcile",
            mismatched_transaction.id
        ))
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;
    assert_eq!(force_response.status(), 200);
    let matched_transaction: GetTransactionResponse = force_response.json().await.unwrap();

    // First manual post - should succeed
    let first_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::ManualPost {
            reason: Some("First manual post".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![matched_transaction.id.clone()],
        },
    };

    let first_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&first_request)
        .send()
        .await;
    assert_eq!(first_response.status(), 200);
    let first_results: Vec<BulkOperationResultItem> = first_response.json().await.unwrap();
    assert_eq!(first_results.len(), 1);
    assert_eq!(
        first_results[0].status,
        BulkResultStatus::Success {
            logical_id: matched_transaction.transaction_id.clone()
        }
    );

    // Get the posted transaction
    let get_response = client
        .get(&format!("/transactions/{}", matched_transaction.id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(get_response.status(), 200);
    let posted_transaction: GetTransactionResponse = get_response.json().await.unwrap();

    // Second manual post attempt on already posted transaction - should be skipped
    let second_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::ManualPost {
            reason: Some("Second manual post attempt".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![posted_transaction.id.clone()],
        },
    };

    let second_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&second_request)
        .send()
        .await;
    assert_eq!(second_response.status(), 200);

    let second_results: Vec<BulkOperationResultItem> = second_response.json().await.unwrap();
    assert_eq!(second_results.len(), 1);

    match &second_results[0].status {
        BulkResultStatus::Skipped { reason, logical_id } => {
            assert!(reason.contains("not eligible for manual posting"));
            assert_eq!(logical_id, &posted_transaction.transaction_id);
        }
        status => panic!("Expected skipped status, got: {status:?}"),
    }
}

#[actix_web::test]
async fn transaction_bulk_manual_post_verify_balances() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Bulk Merchant", "Bulk Profile").await;
    let source_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP Balance Source",
        BalanceDirection::Debit,
    )
    .await;
    let target_account_id = create_test_account(
        &client,
        &token,
        "Bulk MP Balance Target",
        BalanceDirection::Credit,
    )
    .await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "bulk_mp_balance_rule",
        source_account_id.clone(),
        target_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create a DataMismatch transaction
    let mismatched_transaction = create_test_transaction(
        &client,
        &token,
        &source_account_id,
        &target_account_id,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::DataMismatch,
        Some(DataMismatchType::Metadata),
    )
    .await;

    // Force reconcile to get MatchedForce status
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Approved for posting".to_string()),
    };

    let force_response = client
        .put(&format!(
            "/exception_management/transactions/{}/force_reconcile",
            mismatched_transaction.id
        ))
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;
    assert_eq!(force_response.status(), 200);
    let matched_transaction: GetTransactionResponse = force_response.json().await.unwrap();

    let bulk_request = TransactionBulkOperationRequest {
        action: TransactionBulkAction::ManualPost {
            reason: Some("Approved".to_string()),
        },
        selection: TransactionBulkSelection::Ids {
            ids: vec![matched_transaction.id.clone()],
        },
    };

    let bulk_response = client
        .post("/transactions/bulk_operations")
        .with_auth(&token)
        .json(&bulk_request)
        .send()
        .await;
    assert_eq!(bulk_response.status(), 200);

    let results: Vec<BulkOperationResultItem> = bulk_response.json().await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].status,
        BulkResultStatus::Success {
            logical_id: matched_transaction.transaction_id.clone()
        }
    );

    let get_response = client
        .get(&format!(
            "/transactions?transaction_id={}",
            matched_transaction.transaction_id
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(get_response.status(), 200);
    let transactions: Vec<TransactionWithEntriesResponse> = get_response.json().await.unwrap();

    let posted_transaction = transactions
        .iter()
        .find(|t| t.status != ApiTransactionStatus::Archived)
        .expect("Should have non-archived transaction");

    assert_eq!(
        posted_transaction.status,
        ApiTransactionStatus::Posted(
            common_types::transaction::status::TransactionPostedData::Manual
        )
    );

    for entry in &posted_transaction.entries {
        assert_eq!(entry.status, common_enums::EntryStatus::Posted);
    }
}
