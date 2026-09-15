use core::panic;

use crate::{
    api::{
        account_test::create_test_account,
        ingestion_test::create_ingestion_config,
        staging_entry_test::{
            create_test_recon_rule_for_staging_entry, list_staging_entries_v2,
            list_staging_entries_v2_request, staging_entry_request,
        },
        transformation_test::{create_transformation_config, transformation_config_request},
    },
    common::TestApp,
};
use api_models::recon_rule::aging_config::{AgingConfigApiRequest, ThresholdApiRequest};
use api_models::{
    entry::{EntryData, FieldMismatchDetail, GetEntryResponse, MetadataMismatchData},
    metadata_schema::{
        FieldType, MetadataExposeConfig, MetadataSchemaData, UniqueConstraint, UniqueConstraintType,
    },
    recon_engine::ReconTriggerResponse,
    recon_rule::{
        LinkWithRuleUpdate, OneToOneSingleSingle, OneToOneSingleSingleSource,
        OneToOneSingleSingleTarget, OneToOneStrategy, ReconRuleCreateRequest,
        ReconRuleUpdateRequest, ReconStrategy,
    },
    search_entry_key::{SearchEntryKeyListRequest, SearchEntryKeyResponse},
    staging_entry::{
        ApiStagingEntryCoarseStatus, ApiStagingEntryListFiltersV2, ApiStagingEntryStatus,
        GetStagingEntryResponse, StagingEntryCreateRequest,
    },
    transaction::{
        ApiTransactionFilterStatus, ApiTransactionListFiltersV2, ApiTransactionStatus,
        DirectTransactionEntry, ExpectedTransactionEntry, ForceReconcileRequest,
        GetTransactionResponse, ListTransactionsV2Request, ManualReconciliationRequest,
        SearchExpectation, TransactionData, TransactionEntry, TransactionEntryWithData,
        TransactionListResponse, TransactionListV2Page, TransactionRequest, VoidTransactionRequest,
    },
    transformation::TransformationConfigResponse,
};
use common_enums::{BalanceDirection, Currency, ResolutionActionType, StagingEntryProcessingMode};
use common_types::{
    composite_field::CompositeDelimiter,
    date_time::TimeRange,
    pagination::{CursorDirection, CursorPagination, SortOrder},
    recon_rule::{
        entry_field::EntryField,
        link_with_rule::{LinkWithRule, LinkWithRuleTargetData, LinkWithRuleV1},
        match_rules::{MatchOperator, MatchRuleV1, MatchRules, MatchRulesV1},
        search_identifier::{SearchIdentifier, SearchIdentifierV1},
        trigger::{
            archived::TriggerV1, BasicComparisonOperator, Trigger, TriggerOperator,
            TriggerOperatorV1,
        },
    },
    staging_entry::{status::StagingEntryManualReviewData, StagingEntryListSortField},
    transaction::status::{
        TransactionAmountMismatchData, TransactionMatchedData, TransactionPostedData,
    },
    transaction::TransactionListSortField,
    units::FloatMajorUnit,
};
use common_utils::id_type::MerchantId;
use masking::Secret;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::json;
use uuid;
// Local test fixtures
fn direct_transaction_entry(account_id: &str, amount: f64) -> DirectTransactionEntry {
    DirectTransactionEntry {
        account_id: account_id.to_string(),
        entry_type: BalanceDirection::Debit,
        amount: FloatMajorUnit::new(amount),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(serde_json::json!({"test": "data"})),
        staging_entry_id: None,
        transformation_id: None,
        order_id: "order_test".to_string(),
    }
}

fn simple_transaction_request(account_id: &str, rule_id: &str) -> TransactionRequest {
    TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(direct_transaction_entry(account_id, 100.0)),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.to_string(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(serde_json::json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
        ],
        rule_id: rule_id.to_string(),
    }
}

/// # Panics
/// Panics if the HTTP response cannot be deserialized as JSON.
pub async fn create_test_recon_rule(
    client: &crate::common::TestClient,
    token: &str,
    rule_name: &str,
    source_account_id: String,
    target_account_id: String,
    aging_config: AgingConfigApiRequest,
) -> String {
    let request = ReconRuleCreateRequest {
        rule_name: format!("Test Rule {rule_name}"),
        rule_description: format!("Test rule for {rule_name}"),
        priority: 10,
        aging_config,
        strategy: ReconStrategy::OneToOne(OneToOneStrategy::SingleSingle(OneToOneSingleSingle {
            source_account: OneToOneSingleSingleSource {
                account_id: source_account_id,
                trigger: Trigger::V1(TriggerV1 {
                    field: "metadata.payment_type".parse().unwrap(),
                    operator: TriggerOperator::V1(TriggerOperatorV1 {
                        value: BasicComparisonOperator::Equals,
                    }),
                    value: "card".to_string(),
                }),
            },
            target_account: OneToOneSingleSingleTarget {
                account_id: target_account_id,
                tolerance_config: None,
            },
            match_rules: MatchRules::V1(MatchRulesV1 {
                rules: vec![
                    MatchRuleV1 {
                        source_field: "amount".parse().unwrap(),
                        target_field: "amount".parse().unwrap(),
                        operator: MatchOperator::Equals,
                        label: None,
                    },
                    MatchRuleV1 {
                        source_field: "metadata.merchant_name".parse().unwrap(),
                        target_field: "metadata.merchant_name".parse().unwrap(),
                        operator: MatchOperator::Equals,
                        label: None,
                    },
                ],
            }),
            search_identifier: SearchIdentifier::V1(SearchIdentifierV1 {
                source_field: "metadata.order_id".parse().unwrap(),
                target_field: "metadata.reference_id".parse().unwrap(),
            }),
        })),
    };

    let response = client
        .post("/recon_rules/v2")
        .with_auth(token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let rule: api_models::recon_rule::GetReconRuleResponse = response.json().await.unwrap();
    rule.rule_id
}

#[actix_web::test]
async fn create_transaction_with_posted_entries_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_001",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction with posted entries
    let transaction_request = simple_transaction_request(&account_id, &rule_id);

    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    assert_eq!(response.status(), 201);

    let transaction: GetTransactionResponse = response.json().await.unwrap();
    assert!(transaction.id.starts_with("txn_"));
    assert_eq!(
        transaction.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
    );
    assert_eq!(transaction.entry_id.len(), 2);
    assert_eq!(transaction.version, 1);
    assert!(transaction.discarded_at.is_none());
    assert!(transaction.discarded_status.is_none());

    // Validate that manually created entries have NO staging_entry_id or transformation_id
    for entry_id in &transaction.entry_id {
        let entry_response = client
            .get(&format!("/entries/{entry_id}"))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(entry_response.status(), 200);
        let entry: GetEntryResponse = entry_response.json().await.unwrap();

        // Manual transaction entries should NOT have staging tracking
        assert!(
            entry.staging_entry_id.is_none(),
            "Manual entry {entry_id} should NOT have staging_entry_id"
        );
        assert!(
            entry.transformation_id.is_none(),
            "Manual entry {entry_id} should NOT have transformation_id"
        );
    }
}

#[actix_web::test]
async fn get_transaction_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_001",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction first
    let transaction_request = simple_transaction_request(&account_id, &rule_id);

    let create_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    let created_transaction: GetTransactionResponse = create_response.json().await.unwrap();

    // Get the transaction
    let get_response = client
        .get(&format!("/transactions/{}", created_transaction.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(get_response.status(), 200);

    let retrieved_transaction: GetTransactionResponse = get_response.json().await.unwrap();
    assert_eq!(retrieved_transaction.id, created_transaction.id);
    assert_eq!(
        retrieved_transaction.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
    );
    assert_eq!(retrieved_transaction.entry_id, created_transaction.entry_id);
    assert_eq!(retrieved_transaction.version, created_transaction.version);
    assert_eq!(
        retrieved_transaction.profile_id,
        created_transaction.profile_id
    );
}

#[actix_web::test]
async fn get_transaction_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to get a non-existent transaction
    let response = client
        .get("/transactions/txn_nonexistent")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

#[rstest]
#[case(vec![], "test_rule_validation", 400)] // Empty entries — InsufficientEntries
#[case(vec![TransactionEntry::Direct(DirectTransactionEntry {
    account_id: "invalid_account".to_string(),
    entry_type: BalanceDirection::Debit,
    amount: FloatMajorUnit::new(100.0),
    currency: Currency::USD,
    effective_at: common_utils::date_time::now(),
    metadata: Secret::new(json!({"test": "data"})),
    staging_entry_id: None,
    transformation_id: None,
    order_id: "order_test".to_string(),
})], "test_rule_validation", 400)] // Single entry — InsufficientEntries
#[case(vec![
    TransactionEntry::Direct(DirectTransactionEntry {
        account_id: "any_account".to_string(),
        entry_type: BalanceDirection::Debit,
        amount: FloatMajorUnit::new(-100.0), // Negative — InvalidAmount
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"test": "data"})),
        staging_entry_id: None,
        transformation_id: None,
        order_id: "order_test".to_string(),
    }),
    TransactionEntry::Direct(DirectTransactionEntry {
        account_id: "any_account".to_string(),
        entry_type: BalanceDirection::Credit,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"test": "data"})),
        staging_entry_id: None,
        transformation_id: None,
        order_id: "order_test".to_string(),
    }),
], "test_rule_validation", 400)] // Negative amount — InvalidAmount (validate_positive_amounts runs before account check)
#[case(vec![
    TransactionEntry::Direct(DirectTransactionEntry {
        account_id: "any_account".to_string(),
        entry_type: BalanceDirection::Debit,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"test": "data"})),
        staging_entry_id: None,
        transformation_id: None,
        order_id: "order_test".to_string(),
    }),
    TransactionEntry::Direct(DirectTransactionEntry {
        account_id: "any_account".to_string(),
        entry_type: BalanceDirection::Credit,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"test": "data"})),
        staging_entry_id: None,
        transformation_id: None,
        order_id: "order_test".to_string(),
    }),
], "   ", 400)] // Whitespace-only rule_id — InvalidRuleId (trim() check)
#[actix_web::test]
async fn create_transaction_validation_errors(
    #[case] entries: Vec<TransactionEntry>,
    #[case] rule_id: &str,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let transaction_request = TransactionRequest {
        entries,
        rule_id: rule_id.to_string(),
    };

    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);
}

#[actix_web::test]
async fn create_transaction_currency_mismatch_with_account_returns_error() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create accounts in USD (the default in `create_test_account`).
    let debit_account =
        create_test_account(&client, &token, "Debit USD", BalanceDirection::Debit).await;
    let credit_account =
        create_test_account(&client, &token, "Credit USD", BalanceDirection::Credit).await;

    // Submit entries with EUR currency — mismatched against USD accounts.
    // `validate_accounts_and_currencies` should reject before any DB write.
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account,
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::EUR,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account,
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::EUR,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
        ],
        rule_id: "test_rule_currency_mismatch".to_string(),
    };

    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    assert_eq!(response.status(), 400); // EntryCurrencyMismatch → BadRequest
}

#[actix_web::test]
async fn create_transaction_validation_failure_no_db_operations() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create one valid test account
    let valid_account_id =
        create_test_account(&client, &token, "Valid Account", BalanceDirection::Debit).await;

    // Create transaction with one valid entry and one invalid entry
    // This will fail during validation BEFORE any database operations
    let transaction_request = TransactionRequest {
        entries: vec![
            // Valid entry - should normally succeed
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: valid_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            // Invalid entry - non-existent account should cause validation failure
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: "nonexistent_account_id".to_string(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
        ],
        rule_id: "test_rule_validation_failure".to_string(),
    };

    // Attempt to create transaction - should fail during validation
    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    // Should return 404 Not Found due to invalid account during validation
    assert_eq!(response.status(), 404);

    // Verify no entries were created (this is due to validation failure, not atomicity)
    let entries_response = client
        .get(&format!("/accounts/{valid_account_id}/entries"))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(entries_response.status(), 200);
    let entries: Vec<GetEntryResponse> = entries_response.json().await.unwrap();
    assert_eq!(entries.len(), 0);
}

// ================================================================================================
// TRANSACTION CREATION VALIDATION TESTS
// ================================================================================================

#[rstest]
#[case(
    100.0,
    100.0,
    201,
    ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
)] // Balanced amounts
#[case(
    75.0,
    75.0,
    201,
    ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
)] // Different balanced amounts
#[case(
    100.0,
    50.0,
    400,
    ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
)] // Unbalanced amounts
#[actix_web::test]
async fn create_transaction_balance_validation(
    #[case] debit_amount: f64,
    #[case] credit_amount: f64,
    #[case] expected_status: u16,
    #[case] expected_transaction_status: ApiTransactionStatus,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_balance",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction with specified amounts
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(debit_amount),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(credit_amount),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
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

    assert_eq!(response.status(), expected_status);

    if expected_status != 400 {
        let transaction: GetTransactionResponse = response.json().await.unwrap();
        assert!(transaction.id.starts_with("txn_"));
        assert_eq!(transaction.status, expected_transaction_status);
        assert_eq!(transaction.entry_id.len(), 2);
        assert_eq!(transaction.version, 1);
    }
}

// ================================================================================================
// LIST TRANSACTIONS API TESTS
// ================================================================================================

// Helper function to create a transaction with specific properties for testing

#[allow(dead_code)]
pub enum DataMismatchType {
    Amount { variance: f64 },
    BalanceDirection,
    Metadata,
    Currency,
}

/// # Panics
/// Panics if the HTTP response cannot be deserialized as JSON.
pub async fn create_test_transaction(
    client: &crate::common::TestClient,
    token: &str,
    account_id1: &str,
    account_id2: &str,
    rule_id: &str,
    merchant_id: &MerchantId,
    profile_id: &str,
    transaction_status: ApiTransactionStatus,
    data_mismatch_type: Option<DataMismatchType>,
) -> GetTransactionResponse {
    match transaction_status {
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
        | ApiTransactionStatus::Matched(TransactionMatchedData::Force)
        | ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
        | ApiTransactionStatus::Matched(TransactionMatchedData::WithTolerance)
        | ApiTransactionStatus::Posted(TransactionPostedData::Manual) => {
            let transaction_request = TransactionRequest {
                entries: vec![
                    TransactionEntry::Direct(DirectTransactionEntry {
                        account_id: account_id1.to_string(),
                        entry_type: BalanceDirection::Debit,
                        amount: FloatMajorUnit::new(100.0),
                        currency: Currency::USD,
                        effective_at: common_utils::date_time::now(),
                        metadata: Secret::new(json!({"test": "data"})),
                        staging_entry_id: None,
                        transformation_id: None,
                        order_id: "order_test".to_string(),
                    }),
                    TransactionEntry::Direct(DirectTransactionEntry {
                        account_id: account_id1.to_string(),
                        entry_type: BalanceDirection::Credit,
                        amount: FloatMajorUnit::new(100.0),
                        currency: Currency::USD,
                        effective_at: common_utils::date_time::now(),
                        metadata: Secret::new(json!({"test": "data"})),
                        staging_entry_id: None,
                        transformation_id: None,
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
            let created_transaction: GetTransactionResponse = response.json().await.unwrap();
            created_transaction
        }
        ApiTransactionStatus::Expected | ApiTransactionStatus::Missing => {
            let transaction_request = TransactionRequest {
                entries: vec![
                    TransactionEntry::Direct(DirectTransactionEntry {
                        account_id: account_id1.to_string(),
                        entry_type: BalanceDirection::Debit,
                        amount: FloatMajorUnit::new(100.0),
                        currency: Currency::USD,
                        effective_at: common_utils::date_time::now(),
                        metadata: Secret::new(json!({"test": "data"})),
                        staging_entry_id: None,
                        transformation_id: None,
                        order_id: "order_test".to_string(),
                    }),
                    TransactionEntry::Expected(ExpectedTransactionEntry {
                        account_id: account_id1.to_string(),
                        entry_type: BalanceDirection::Credit,
                        amount: FloatMajorUnit::new(100.0),
                        currency: Currency::USD,
                        effective_at: common_utils::date_time::now(),
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
            let created_transaction: GetTransactionResponse = response.json().await.unwrap();
            created_transaction
        }
        ApiTransactionStatus::DataMismatch
        | ApiTransactionStatus::CurrencyMismatch
        | ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch)
        | ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
        | ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected)
        | ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected)
        | ApiTransactionStatus::Archived
        | ApiTransactionStatus::Void
        | ApiTransactionStatus::PartiallyReconciled => {
            let source_account_response = client
                .get(&format!("/accounts/{account_id1}"))
                .with_auth(token)
                .send()
                .await;
            assert_eq!(source_account_response.status(), 200);
            let source_account: api_models::account::GetAccountResponse =
                source_account_response.json().await.unwrap();

            let target_account_response = client
                .get(&format!("/accounts/{account_id2}"))
                .with_auth(token)
                .send()
                .await;

            assert_eq!(target_account_response.status(), 200);
            let target_account: api_models::account::GetAccountResponse =
                target_account_response.json().await.unwrap();

            let source_transformation_config = create_transformation_config(
                profile_id,
                &source_account.account_id,
                None,
                merchant_id,
                token,
                client,
                "order_id".parse().unwrap(),
                StagingEntryProcessingMode::Transaction,
                vec![],
            )
            .await;

            let target_transformation_config = create_transformation_config(
                profile_id,
                &target_account.account_id,
                None,
                merchant_id,
                token,
                client,
                "metadata.reference_id".parse().unwrap(),
                StagingEntryProcessingMode::Transaction,
                vec![],
            )
            .await;

            let response = client
                .post("/staging_entries")
                .with_auth(token)
                .json(&StagingEntryCreateRequest {
                    account_id: source_account.account_id.clone(),
                    entry_type: BalanceDirection::Credit,
                    processing_mode: StagingEntryProcessingMode::Transaction,
                    amount: FloatMajorUnit::new(100.0),
                    currency: Currency::USD,
                    effective_at: common_utils::date_time::now(),
                    metadata: Secret::new(json!({
                        "payment_type": "card",
                        "order_id": "ORDER-FORCE-RECONCILE-TEST",
                        "merchant_name": "Amazon Store" // Source metadata value
                    })),
                    transformation_id: source_transformation_config.transformation_id.clone(),
                    transformation_history_id: None,
                    order_id: "order_test".to_string(),
                })
                .send()
                .await;
            assert_eq!(response.status(), 201);
            let transaction_staging: GetStagingEntryResponse = response.json().await.unwrap();

            let response = client
                .post(&format!("/recon_engine/{}/process", transaction_staging.id))
                .with_auth(token)
                .send()
                .await;
            assert_eq!(response.status(), 200);
            let transaction_result: ReconTriggerResponse = response.json().await.unwrap();

            match transaction_result {
                ReconTriggerResponse::Success(transaction) => {
                    assert_eq!(transaction.status, ApiTransactionStatus::Expected);
                    assert_eq!(transaction.entry_id.len(), 2); // Source + Expected entry
                    transaction
                }
                ReconTriggerResponse::Failure(reason) => {
                    panic!("Transaction processing failed: {reason}");
                }
            };

            let (
                confirmation_amount,
                confirmation_currency,
                confirmation_entry_type,
                confirmation_metadata,
                confirmation_transaction_status,
            ) = match data_mismatch_type {
                Some(DataMismatchType::BalanceDirection) => (
                    FloatMajorUnit::new(100.0), // Same amount
                    Currency::USD,
                    BalanceDirection::Credit, // Different direction (should be Debit)
                    json!({
                        "reference_id": "ORDER-FORCE-RECONCILE-TEST",
                        "payment_method": "card",
                        "merchant_name": "Amazon Store" // Same metadata
                    }),
                    ApiTransactionStatus::DataMismatch,
                ),
                Some(DataMismatchType::Currency) => (
                    FloatMajorUnit::new(100.0), // Same amount
                    Currency::EUR,              // Different currency
                    BalanceDirection::Debit,
                    json!({
                        "reference_id": "ORDER-FORCE-RECONCILE-TEST",
                        "payment_method": "card",
                        "merchant_name": "Amazon Store" // Same metadata
                    }),
                    ApiTransactionStatus::CurrencyMismatch,
                ),
                Some(DataMismatchType::Metadata) | None => (
                    FloatMajorUnit::new(100.0), // Same amount
                    Currency::USD,
                    BalanceDirection::Debit,
                    json!({
                        "reference_id": "ORDER-FORCE-RECONCILE-TEST", // Same reference
                        "payment_method": "card",
                        "merchant_name": "eBay Store" // DIFFERENT metadata value - causes mismatch
                    }),
                    ApiTransactionStatus::DataMismatch,
                ),
                Some(DataMismatchType::Amount { variance }) => {
                    match transaction_status {
                        ApiTransactionStatus::UnderAmount(
                            TransactionAmountMismatchData::Mismatch,
                        ) => {
                            (
                                FloatMajorUnit::new(100.0 - variance),
                                Currency::USD,
                                BalanceDirection::Debit,
                                json!({
                                    "reference_id": "ORDER-FORCE-RECONCILE-TEST", // Same reference
                                    "payment_method": "card",
                                    "merchant_name": "Amazon Store" // Same metadata
                                }),
                                ApiTransactionStatus::UnderAmount(
                                    TransactionAmountMismatchData::Mismatch,
                                ),
                            )
                        }
                        ApiTransactionStatus::OverAmount(
                            TransactionAmountMismatchData::Mismatch,
                        ) => {
                            (
                                FloatMajorUnit::new(100.0 + variance),
                                Currency::USD,
                                BalanceDirection::Debit,
                                json!({
                                    "reference_id": "ORDER-FORCE-RECONCILE-TEST", // Same reference
                                    "payment_method": "card",
                                    "merchant_name": "Amazon Store" // Same metadata
                                }),
                                ApiTransactionStatus::OverAmount(
                                    TransactionAmountMismatchData::Mismatch,
                                ),
                            )
                        }
                        _ => {
                            panic!("Not implemented")
                        }
                    }
                }
            };

            let response = client
                .post("/staging_entries")
                .with_auth(token)
                .json(&StagingEntryCreateRequest {
                    account_id: target_account.account_id.clone(),
                    entry_type: confirmation_entry_type,
                    processing_mode: StagingEntryProcessingMode::Confirmation,
                    amount: confirmation_amount,
                    currency: confirmation_currency,
                    effective_at: common_utils::date_time::now(),
                    metadata: Secret::new(confirmation_metadata),
                    transformation_id: target_transformation_config.transformation_id.clone(),
                    transformation_history_id: None,
                    order_id: "order_test".to_string(),
                })
                .send()
                .await;
            assert_eq!(response.status(), 201);

            let confirmation_staging: GetStagingEntryResponse = response.json().await.unwrap();

            let response = client
                .post(&format!(
                    "/recon_engine/{}/process",
                    confirmation_staging.id
                ))
                .with_auth(token)
                .send()
                .await;
            assert_eq!(response.status(), 200);
            let confirmation_result: ReconTriggerResponse = response.json().await.unwrap();

            let mismatched_transaction = match confirmation_result {
                ReconTriggerResponse::Success(transaction) => {
                    assert_eq!(transaction.status, confirmation_transaction_status);
                    assert_eq!(transaction.entry_id.len(), 2); // Source + Target entries
                    assert_eq!(transaction.version, 2); // Incremented from original transaction
                    if matches!(data_mismatch_type, Some(DataMismatchType::Metadata) | None) {
                        assert_eq!(
                            transaction.data,
                            TransactionData::DataMismatch(MetadataMismatchData {
                                mismatched_fields: vec![FieldMismatchDetail {
                                    field_name: "metadata.merchant_name".parse().unwrap(),
                                    label: None,
                                    expected_value: "Amazon Store".to_string(),
                                    actual_value: "eBay Store".to_string(),
                                }],
                            })
                        );
                    }
                    transaction
                }
                ReconTriggerResponse::Failure(reason) => {
                    panic!("Expected metadata mismatch but got Failure: {reason}");
                }
            };

            // Check if we need to void the transaction for Void or Archived status
            if transaction_status == ApiTransactionStatus::Void
                || transaction_status == ApiTransactionStatus::Archived
            {
                let void_response = client
                    .put(&format!("/transactions/{}/void", mismatched_transaction.id))
                    .with_auth(token)
                    .json(&VoidTransactionRequest {
                        reason: Some("Void test".to_string()),
                    })
                    .send()
                    .await;

                assert_eq!(void_response.status(), 200);
                let voided_transaction: GetTransactionResponse =
                    void_response.json().await.unwrap();

                // For Archived status, return the original (now archived) transaction
                // For Void status, return the new voided transaction
                if transaction_status == ApiTransactionStatus::Archived {
                    // Get the original transaction which should now be archived
                    let original_response = client
                        .get(&format!("/transactions/{}", mismatched_transaction.id))
                        .with_auth(token)
                        .send()
                        .await;

                    assert_eq!(original_response.status(), 200);
                    let archived_transaction: GetTransactionResponse =
                        original_response.json().await.unwrap();
                    assert_eq!(archived_transaction.status, ApiTransactionStatus::Archived);
                    archived_transaction
                } else {
                    // Return the new voided transaction
                    assert_eq!(voided_transaction.status, ApiTransactionStatus::Void);
                    voided_transaction
                }
            } else if transaction_status == ApiTransactionStatus::PartiallyReconciled {
                let partial_response = client
                    .post(&format!(
                        "/exception_management/transactions/{}/manual_reconciliation",
                        mismatched_transaction.id
                    ))
                    .with_auth(token)
                    .json(&ManualReconciliationRequest {
                        transaction_entries: vec![
                            TransactionEntryWithData {
                                account_id: source_account.account_id.clone(),
                                entry_type: BalanceDirection::Debit,
                                amount: FloatMajorUnit::new(100.0),
                                currency: Currency::USD,
                                effective_at: common_utils::date_time::now(),
                                metadata: Secret::new(json!({"test": "data"})),
                                data: EntryData::Pending,
                                staging_entry_id: None,
                                order_id: "order_test".to_string(),
                            },
                            TransactionEntryWithData {
                                account_id: target_account.account_id.clone(),
                                entry_type: BalanceDirection::Credit,
                                amount: FloatMajorUnit::new(80.0),
                                currency: Currency::USD,
                                effective_at: common_utils::date_time::now(),
                                metadata: Secret::new(json!({"test": "data"})),
                                data: EntryData::Pending,
                                staging_entry_id: None,
                                order_id: "order_test".to_string(),
                            },
                        ],
                        reason: Some("Testing".to_string()),
                    })
                    .send()
                    .await;
                assert_eq!(partial_response.status(), 200);
                let partial_transaction: GetTransactionResponse =
                    partial_response.json().await.unwrap();
                assert_eq!(
                    partial_transaction.status,
                    ApiTransactionStatus::PartiallyReconciled
                );
                partial_transaction
            } else {
                // Return the mismatched transaction as-is
                *mismatched_transaction
            }
        }
        ApiTransactionStatus::SplitMismatch => panic!("Split mismatch not handled"),
    }
}

// Helper function to create a transaction with specific debit and credit accounts
pub async fn create_transaction_for_filtering(
    client: &crate::common::TestClient,
    token: &str,
    debit_account_id: &str,
    credit_account_id: &str,
    rule_id: &str,
) -> GetTransactionResponse {
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id.to_string(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id.to_string(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
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

fn list_transactions_v2_request(
    filters: ApiTransactionListFiltersV2,
    sort_by: TransactionListSortField,
    limit: Option<u16>,
    direction: CursorDirection,
    order: SortOrder,
) -> ListTransactionsV2Request {
    ListTransactionsV2Request {
        filters,
        cursor_payload: CursorPagination {
            limit,
            direction,
            sort_by,
            order,
        },
    }
}

async fn list_transactions_v2(
    client: &crate::common::TestClient,
    token: &str,
    request: ListTransactionsV2Request,
) -> TransactionListV2Page {
    let response = client
        .post("/transactions/v2/list")
        .with_auth(token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

fn transaction_ids(page: &TransactionListV2Page) -> Vec<String> {
    page.items
        .iter()
        .map(|transaction| transaction.id.clone())
        .collect()
}

async fn create_transaction_for_v2_listing(
    client: &crate::common::TestClient,
    token: &str,
    debit_account_id: &str,
    credit_account_id: &str,
    rule_id: &str,
    order_id: &str,
    effective_at: time::PrimitiveDateTime,
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
                order_id: order_id.to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id.to_string(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at,
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: order_id.to_string(),
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

async fn create_transaction_with_entry_counts(
    client: &crate::common::TestClient,
    token: &str,
    debit_account_id: &str,
    credit_account_id: &str,
    rule_id: &str,
    order_id: &str,
    effective_at: time::PrimitiveDateTime,
    debit_count: u32,
    credit_count: u32,
) -> GetTransactionResponse {
    let debit_amount = FloatMajorUnit::new(f64::from(credit_count));
    let credit_amount = FloatMajorUnit::new(f64::from(debit_count));

    let build = |account_id: &str, entry_type: BalanceDirection, amount: FloatMajorUnit| {
        TransactionEntry::Direct(DirectTransactionEntry {
            account_id: account_id.to_string(),
            entry_type,
            amount,
            currency: Currency::USD,
            effective_at,
            metadata: Secret::new(json!({"test": "data"})),
            staging_entry_id: None,
            transformation_id: None,
            order_id: order_id.to_string(),
        })
    };

    let entries = (0..debit_count)
        .map(|_| build(debit_account_id, BalanceDirection::Debit, debit_amount))
        .chain(
            (0..credit_count)
                .map(|_| build(credit_account_id, BalanceDirection::Credit, credit_amount)),
        )
        .collect::<Vec<_>>();

    let response = client
        .post("/transactions")
        .with_auth(token)
        .json(&TransactionRequest {
            entries,
            rule_id: rule_id.to_string(),
        })
        .send()
        .await;

    assert_eq!(response.status(), 201);
    response.json().await.unwrap()
}

async fn bootstrap_for_entry_cap(
    client: &crate::common::TestClient,
    rule_name: &str,
) -> (String, String, String, String) {
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account =
        create_test_account(client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account =
        create_test_account(client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        client,
        &token,
        rule_name,
        debit_account.clone(),
        credit_account.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    (token, debit_account, credit_account, rule_id)
}

async fn list_all_transactions_v2(
    client: &crate::common::TestClient,
    token: &str,
) -> TransactionListV2Page {
    list_transactions_v2(
        client,
        token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            TransactionListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await
}

#[actix_web::test]
async fn test_list_transactions_v2_caps_entries_and_flags_more() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_cap").await;

    create_transaction_with_entry_counts(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_entry_cap_order",
        common_utils::date_time::now(),
        4,
        4,
    )
    .await;

    let page = list_all_transactions_v2(&client, &token).await;

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].entries.len(), 5);
    assert!(page.items[0].has_more_entries);
}

#[actix_web::test]
async fn test_list_transactions_v2_entry_cap_boundary_exactly_five() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_cap_five").await;

    create_transaction_with_entry_counts(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_entry_cap_five_order",
        common_utils::date_time::now(),
        2,
        3,
    )
    .await;

    let page = list_all_transactions_v2(&client, &token).await;

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].entries.len(), 5);
    assert!(!page.items[0].has_more_entries);
}

#[actix_web::test]
async fn test_list_transactions_v2_entry_cap_boundary_exactly_six() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_cap_six").await;

    create_transaction_with_entry_counts(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_entry_cap_six_order",
        common_utils::date_time::now(),
        3,
        3,
    )
    .await;

    let page = list_all_transactions_v2(&client, &token).await;

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].entries.len(), 5);
    assert!(page.items[0].has_more_entries);
}

#[actix_web::test]
async fn test_list_transactions_v2_small_transaction_reports_no_more_entries() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_cap_small").await;

    create_transaction_for_v2_listing(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_entry_cap_small_order",
        common_utils::date_time::now(),
    )
    .await;

    let page = list_all_transactions_v2(&client, &token).await;

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].entries.len(), 2);
    assert!(!page.items[0].has_more_entries);
}

#[actix_web::test]
async fn test_list_transactions_v2_truncates_each_transaction_independently() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_cap_mixed").await;

    let small = create_transaction_for_v2_listing(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_entry_cap_mixed_small",
        common_utils::date_time::now(),
    )
    .await;

    let large = create_transaction_with_entry_counts(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_entry_cap_mixed_large",
        common_utils::date_time::now(),
        4,
        4,
    )
    .await;

    let page = list_all_transactions_v2(&client, &token).await;
    assert_eq!(page.items.len(), 2);

    let small_item = page
        .items
        .iter()
        .find(|item| item.id == small.id)
        .expect("small transaction missing from page");
    let large_item = page
        .items
        .iter()
        .find(|item| item.id == large.id)
        .expect("large transaction missing from page");

    assert_eq!(small_item.entries.len(), 2);
    assert!(!small_item.has_more_entries);
    assert_eq!(large_item.entries.len(), 5);
    assert!(large_item.has_more_entries);
}

#[actix_web::test]
async fn test_list_transactions_v2_returns_first_entries_in_creation_order() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_cap_stable").await;

    // 4 debits are created before 4 credits, so the retained 5 must be all 4 debits then 1 credit.
    create_transaction_with_entry_counts(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_entry_cap_stable_order",
        common_utils::date_time::now(),
        4,
        4,
    )
    .await;

    let entry_ids = |page: &TransactionListV2Page| -> Vec<String> {
        page.items[0]
            .entries
            .iter()
            .map(|entry| entry.entry_id.clone())
            .collect()
    };

    let first_page = list_all_transactions_v2(&client, &token).await;
    let second_page = list_all_transactions_v2(&client, &token).await;

    assert_eq!(
        entry_ids(&first_page),
        entry_ids(&second_page),
        "entry truncation must be stable across calls"
    );

    let entry_types: Vec<BalanceDirection> = first_page.items[0]
        .entries
        .iter()
        .map(|entry| entry.entry_type)
        .collect();

    assert_eq!(
        entry_types,
        vec![
            BalanceDirection::Debit,
            BalanceDirection::Debit,
            BalanceDirection::Debit,
            BalanceDirection::Debit,
            BalanceDirection::Credit,
        ],
        "entries must be returned in creation order, not sorted by id"
    );
}

#[actix_web::test]
async fn test_list_transactions_v1_still_returns_all_entries() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v1_no_entry_cap").await;

    create_transaction_with_entry_counts(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v1_no_entry_cap_order",
        common_utils::date_time::now(),
        4,
        4,
    )
    .await;

    let response = client.get("/transactions").with_auth(&token).send().await;
    assert_eq!(response.status(), 200);

    let transactions: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transactions.len(), 1);
    assert_eq!(transactions[0].entries.len(), 8);
    assert!(!transactions[0].has_more_entries);
}

#[actix_web::test]
async fn test_list_transactions_v2_previous_direction_flags_entries() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_cap_previous").await;

    for index in 0..2 {
        create_transaction_with_entry_counts(
            &client,
            &token,
            &debit_account,
            &credit_account,
            &rule_id,
            &format!("v2_entry_cap_previous_order_{index}"),
            common_utils::date_time::now(),
            4,
            4,
        )
        .await;
    }

    let first_page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            TransactionListSortField::Id { id: None },
            Some(1),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.items[0].entries.len(), 5);
    assert!(first_page.items[0].has_more_entries);

    let next_cursor = first_page.next_cursor.expect("next cursor missing");

    let second_page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            next_cursor,
            Some(1),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(second_page.items.len(), 1);
    let prev_cursor = second_page.prev_cursor.expect("prev cursor missing");

    let previous_page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            prev_cursor,
            Some(1),
            CursorDirection::Previous,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(previous_page.items.len(), 1);
    assert_eq!(previous_page.items[0].id, first_page.items[0].id);
    assert_eq!(previous_page.items[0].entries.len(), 5);
    assert!(previous_page.items[0].has_more_entries);
}

#[actix_web::test]
async fn test_list_transactions_v2_paginates_by_id_cursor() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "v2_id_cursor",
        debit_account.clone(),
        credit_account.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    for index in 0..3 {
        create_transaction_for_v2_listing(
            &client,
            &token,
            &debit_account,
            &credit_account,
            &rule_id,
            &format!("v2_id_cursor_order_{index}"),
            common_utils::date_time::now(),
        )
        .await;
    }

    let first_page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            TransactionListSortField::Id { id: None },
            Some(2),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(first_page.items.len(), 2);
    assert!(first_page.prev_cursor.is_none());
    let first_page_ids = transaction_ids(&first_page);
    let next_cursor = first_page
        .next_cursor
        .clone()
        .expect("first page should include a next cursor");

    match &next_cursor {
        TransactionListSortField::Id { id: Some(id) } => {
            assert_eq!(Some(id), first_page_ids.last());
        }
        cursor => panic!("expected id cursor, got {cursor:?}"),
    }

    let second_page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            next_cursor,
            Some(2),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(second_page.items.len(), 1);
    assert!(second_page.next_cursor.is_none());
    assert!(second_page.prev_cursor.is_some());

    let second_page_ids = transaction_ids(&second_page);
    assert!(first_page_ids
        .iter()
        .all(|id| !second_page_ids.contains(id)));

    let previous_page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            second_page
                .prev_cursor
                .expect("second page should include a previous cursor"),
            Some(2),
            CursorDirection::Previous,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(transaction_ids(&previous_page), first_page_ids);
}

#[actix_web::test]
async fn test_list_transactions_v2_filters_status_rule_order_id_and_time_range() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "v2_filter_rule",
        debit_account.clone(),
        credit_account.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let other_rule_id = create_test_recon_rule(
        &client,
        &token,
        "v2_filter_other_rule",
        debit_account.clone(),
        credit_account.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let included_effective_at = common_utils::date_time::now() - time::Duration::hours(2);
    let included = create_transaction_for_v2_listing(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_filter_order",
        included_effective_at,
    )
    .await;
    create_transaction_for_v2_listing(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &rule_id,
        "v2_filter_other_order",
        included_effective_at,
    )
    .await;
    create_transaction_for_v2_listing(
        &client,
        &token,
        &debit_account,
        &credit_account,
        &other_rule_id,
        "v2_filter_order",
        included_effective_at,
    )
    .await;

    let page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2 {
                status: Some(vec![ApiTransactionFilterStatus::MatchedAuto]),
                rule_id: Some(rule_id),
                order_id: Some("v2_filter_order".to_string()),
                time_range: Some(TimeRange {
                    start_time: included_effective_at - time::Duration::hours(1),
                    end_time: Some(included_effective_at + time::Duration::hours(1)),
                }),
                ..Default::default()
            },
            TransactionListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(transaction_ids(&page), vec![included.id]);
    assert_eq!(
        page.items[0].status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
    );
    assert!(page.items[0]
        .entries
        .iter()
        .all(|entry| entry.order_id == "v2_filter_order"));
}

#[actix_web::test]
async fn test_list_transactions_v2_rejects_invalid_filters() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let invalid_time_response = client
        .post("/transactions/v2/list")
        .with_auth(&token)
        .json(&list_transactions_v2_request(
            ApiTransactionListFiltersV2 {
                time_range: Some(TimeRange {
                    start_time: common_utils::date_time::now() + time::Duration::hours(1),
                    end_time: Some(common_utils::date_time::now()),
                }),
                ..Default::default()
            },
            TransactionListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ))
        .send()
        .await;
    assert_eq!(invalid_time_response.status(), 400);

    let unsupported_status_response = client
        .post("/transactions/v2/list")
        .with_auth(&token)
        .json(&list_transactions_v2_request(
            ApiTransactionListFiltersV2 {
                status: Some(vec![ApiTransactionFilterStatus::Archived]),
                ..Default::default()
            },
            TransactionListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ))
        .send()
        .await;
    assert_eq!(unsupported_status_response.status(), 400);
}

#[actix_web::test]
async fn test_list_transactions_basic() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts
    let account_id1 =
        create_test_account(&client, &token, "Test Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rules first
    let rule_001 = create_test_recon_rule(
        &client,
        &token,
        "rule_001",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_002 = create_test_recon_rule(
        &client,
        &token,
        "rule_002",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_003 = create_test_recon_rule(
        &client,
        &token,
        "rule_003",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create multiple transactions
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_001,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_002,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Expected,
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_003,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 10.0 }),
    )
    .await;

    // Test basic list without filters
    let response = client.get("/transactions").with_auth(&token).send().await;

    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 4); // 3 created + 1 from mismatched processing

    // Verify response structure and version field
    for transaction in &transaction_list {
        assert!(transaction.id.starts_with("txn_"));
        if transaction.status
            == ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
        {
            assert_eq!(transaction.version, 2); // Mismatched transaction should have version 2
        } else {
            assert_eq!(transaction.version, 1);
        }
        assert!(!transaction.entries.is_empty());

        // Entry-level `mismatch` reason must be populated exactly when the
        // entry's own status is `Mismatched`, and absent otherwise.
        for entry in &transaction.entries {
            if entry.status == common_enums::EntryStatus::Mismatched {
                assert!(
                    entry.mismatch.is_some(),
                    "mismatched entry should carry a reason"
                );
            } else {
                assert!(
                    entry.mismatch.is_none(),
                    "non-mismatched entry should not carry a reason"
                );
            }
        }
    }
}

// Test list transactions with various filters using rstest
// Test list transactions with various filters - split into separate tests for better control
#[actix_web::test]
async fn test_list_transactions_filters_status_only() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts
    let account_id1 =
        create_test_account(&client, &token, "Test Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rules first
    let rule_001 = create_test_recon_rule(
        &client,
        &token,
        "rule_001",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_002 = create_test_recon_rule(
        &client,
        &token,
        "rule_002",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_003 = create_test_recon_rule(
        &client,
        &token,
        "rule_003",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transactions with different properties
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_001,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_002,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Expected,
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_003,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 20.0 }),
    )
    .await;

    // Test status filters
    let test_cases = vec![
        ("matched_auto", 1),
        ("expected", 1),
        ("over_amount_mismatch", 1),
        ("matched_auto,expected", 2),
    ];

    for (status, expected_count) in test_cases {
        let response = client
            .get(&format!("/transactions?status={status}"))
            .with_auth(&token)
            .send()
            .await;

        assert_eq!(response.status(), 200);
        let transaction_list: TransactionListResponse = response.json().await.unwrap();
        assert_eq!(transaction_list.len(), expected_count);
    }
}

#[actix_web::test]
async fn test_list_transactions_filters_rule_id() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts
    let account_id1 =
        create_test_account(&client, &token, "Test Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rules first
    let rule_001 = create_test_recon_rule(
        &client,
        &token,
        "rule_001",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_002 = create_test_recon_rule(
        &client,
        &token,
        "rule_002",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_003 = create_test_recon_rule(
        &client,
        &token,
        "rule_003",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transactions with different properties
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_001,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_002,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Expected,
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_003,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 10.0 }),
    )
    .await;

    // Test rule_id filter with actual rule IDs
    let response = client
        .get(&format!("/transactions?rule_id={rule_001}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 1);

    // Test combined status + rule filter
    let response = client
        .get(&format!(
            "/transactions?transaction_status=posted&rule_id={rule_001}"
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 1);
}

#[actix_web::test]
async fn test_list_transactions_filters_credit_debit_accounts() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts
    let account_a =
        create_test_account(&client, &token, "Account A", BalanceDirection::Debit).await;
    let account_b =
        create_test_account(&client, &token, "Account B", BalanceDirection::Credit).await;
    let account_c =
        create_test_account(&client, &token, "Account C", BalanceDirection::Debit).await;

    // Create recon rules
    let rule_001 = create_test_recon_rule(
        &client,
        &token,
        "rule_001",
        account_a.clone(),
        account_b.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_002 = create_test_recon_rule(
        &client,
        &token,
        "rule_002",
        account_b.clone(),
        account_c.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_003 = create_test_recon_rule(
        &client,
        &token,
        "rule_003",
        account_a.clone(),
        account_c.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transactions
    // Txn 1: Debit A, Credit B
    create_transaction_for_filtering(&client, &token, &account_a, &account_b, &rule_001).await;
    // Txn 2: Debit B, Credit C
    create_transaction_for_filtering(&client, &token, &account_b, &account_c, &rule_002).await;
    // Txn 3: Debit A, Credit C
    create_transaction_for_filtering(&client, &token, &account_a, &account_c, &rule_003).await;

    // Test cases
    let test_cases = vec![
        // Credit account filters
        (
            format!("credit_account={account_b}"),
            1,
            "Filter by credit_account_b",
        ),
        (
            format!("credit_account={account_c}"),
            2,
            "Filter by credit_account_c",
        ),
        (
            format!("credit_account={account_a}"),
            0,
            "Filter by credit_account_a (no credits)",
        ),
        (
            format!("credit_account={account_b},{account_c}"),
            3,
            "Filter by multiple credit accounts",
        ),
        // Debit account filters
        (
            format!("debit_account={account_a}"),
            2,
            "Filter by debit_account_a",
        ),
        (
            format!("debit_account={account_b}"),
            1,
            "Filter by debit_account_b",
        ),
        (
            format!("debit_account={account_c}"),
            0,
            "Filter by debit_account_c (no debits)",
        ),
        (
            format!("debit_account={account_a},{account_b}"),
            3,
            "Filter by multiple debit accounts",
        ),
        // Combined filters
        (
            format!("credit_account={account_b}&debit_account={account_a}"),
            1,
            "Combined credit_b and debit_a",
        ),
        (
            format!("credit_account={account_c}&debit_account={account_a}"),
            1,
            "Combined credit_c and debit_a",
        ),
        // Edge cases
        (
            "credit_account=non_existent_account".to_string(),
            0,
            "Non-existent credit account",
        ),
        (
            "debit_account=non_existent_account".to_string(),
            0,
            "Non-existent debit account",
        ),
    ];

    for (query, expected_count, description) in test_cases {
        let response = client
            .get(&format!("/transactions?{query}"))
            .with_auth(&token)
            .send()
            .await;

        assert_eq!(response.status(), 200, "Failed test: {}", description);
        let transaction_list: TransactionListResponse = response.json().await.unwrap();
        assert_eq!(
            transaction_list.len(),
            expected_count,
            "Failed count for test: {}",
            description
        );
    }
}

#[rstest]
#[case(Some(1), None, 200)] // start_time only - valid
#[case(Some(1), Some(2), 200)] // start < end - valid
#[case(Some(2), Some(1), 400)] // start > end - invalid
#[case(Some(1), Some(1), 200)] // start = end - valid
#[case(None, None, 200)] // no time filter - valid
#[actix_web::test]
async fn test_list_transactions_time_range_validation(
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account and transaction
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;
    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        "test_rule_posted",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id,
        &account_id2,
        &recon_rule,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;

    // Build query string with time range
    let mut query_params = Vec::new();
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z").unwrap();

    if let Some(start_h) = start_hour {
        let start_time = (common_utils::date_time::now() + time::Duration::hours(start_h))
            .format(&format)
            .unwrap();
        query_params.push(format!("start_time={start_time}"));
    }

    if let Some(end_h) = end_hour {
        let end_time = (common_utils::date_time::now() + time::Duration::hours(end_h))
            .format(&format)
            .unwrap();
        query_params.push(format!("end_time={end_time}"));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    // Make request
    let response = client
        .get(&format!("/transactions{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 200 {
        let transaction_list: TransactionListResponse = response.json().await.unwrap();
        // For valid requests, we should get some response (exact count depends on time range)
        assert!(transaction_list.len() <= 1); // At most 1 transaction we created
    } else {
        // For invalid requests, check error response
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

#[actix_web::test]
async fn test_time_range_filter_uses_effective_at_not_created_at() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts - need separate source and target accounts
    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    // Create test recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_effective_at",
        source_account_id.clone(),
        target_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction with effective_at in the past (yesterday)
    // but created_at will be now
    let past_effective_at = common_utils::date_time::now() - time::Duration::days(1);
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z").unwrap();

    let past_transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: source_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: past_effective_at,
                metadata: Secret::new(serde_json::json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: target_account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: past_effective_at,
                metadata: Secret::new(serde_json::json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
        ],
        rule_id,
    };

    // Create the past transaction (created_at = now, effective_at = yesterday)
    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&past_transaction_request)
        .send()
        .await;
    assert_eq!(response.status(), 201);

    // Test: Filter for yesterday should find the transaction (by effective_at)
    let start_time = (past_effective_at - time::Duration::hours(1))
        .format(&format)
        .unwrap();
    let end_time = (past_effective_at + time::Duration::hours(1))
        .format(&format)
        .unwrap();

    let response = client
        .get(&format!(
            "/transactions?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let filtered_transactions: TransactionListResponse = response.json().await.unwrap();

    // Should find the transaction (filtered by effective_at, not created_at)
    assert_eq!(filtered_transactions.len(), 1);

    // Test: Filter for today should NOT find the transaction (even though created_at is today)
    let now = common_utils::date_time::now();
    let start_time = (now - time::Duration::hours(1)).format(&format).unwrap();
    let end_time = (now + time::Duration::hours(1)).format(&format).unwrap();

    let response = client
        .get(&format!(
            "/transactions?start_time={start_time}&end_time={end_time}"
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let filtered_transactions: TransactionListResponse = response.json().await.unwrap();

    // Should NOT find the transaction (proves filtering uses effective_at, not created_at)
    assert_eq!(filtered_transactions.len(), 0);
}

// Test filter edge cases and specific scenarios
#[actix_web::test]
async fn test_list_transactions_filter_edge_cases() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account and transaction
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_effective_at",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    create_test_transaction(
        &client,
        &token,
        &account_id,
        &account_id2,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;

    // Test with non-existent rule_id
    let response = client
        .get("/transactions?rule_id=nonexistent_rule")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 0);

    // Test with non-existent transaction_id
    let response = client
        .get("/transactions?transaction_id=nonexistent_txn")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 0);
}

#[actix_web::test]
async fn test_list_transactions_filter_edge_cases_rstest() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account and recon rules
    let account_id1 =
        create_test_account(&client, &token, "Test Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let rule_001 = create_test_recon_rule(
        &client,
        &token,
        "rule_001",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_002 = create_test_recon_rule(
        &client,
        &token,
        "rule_002",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_003 = create_test_recon_rule(
        &client,
        &token,
        "rule_003",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create test transactions
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_001,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_002,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Expected,
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_003,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 10.0 }),
    )
    .await;

    // Test with non-existent rule_id
    let response = client
        .get("/transactions?rule_id=nonexistent_rule")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 0);

    // Test with empty rule filter returns all
    let response = client.get("/transactions").with_auth(&token).send().await;

    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 4); // 3 created + 1 from mismatched processing
}

#[actix_web::test]
async fn test_list_transactions_profile_isolation() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap two different profiles
    let (merchant_id1, profile_id1, token1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let (merchant_id2, profile_id2, token2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Create accounts for each profile
    let account_id1 =
        create_test_account(&client, &token1, "Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token2, "Account 2", BalanceDirection::Debit).await;
    let account_id3 =
        create_test_account(&client, &token1, "Account 3", BalanceDirection::Debit).await;
    let account_id4 =
        create_test_account(&client, &token2, "Account 4", BalanceDirection::Debit).await;

    // Create recon rules for each profile
    let rule_001 = create_test_recon_rule(
        &client,
        &token1,
        "rule_001",
        account_id1.clone(),
        account_id3.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_002 = create_test_recon_rule(
        &client,
        &token2,
        "rule_002",
        account_id2.clone(),
        account_id4.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transactions for each profile
    create_test_transaction(
        &client,
        &token1,
        &account_id1,
        "account_id2",
        &rule_001,
        &merchant_id1,
        &profile_id1,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token2,
        &account_id2,
        "account_id3",
        &rule_002,
        &merchant_id2,
        &profile_id2,
        ApiTransactionStatus::Expected,
        None,
    )
    .await;

    // Profile 1 should only see its own transactions
    let response1 = client.get("/transactions").with_auth(&token1).send().await;
    assert_eq!(response1.status(), 200);
    let transaction_list1: TransactionListResponse = response1.json().await.unwrap();
    assert_eq!(transaction_list1.len(), 1);

    // Profile 2 should only see its own transactions
    let response2 = client.get("/transactions").with_auth(&token2).send().await;
    assert_eq!(response2.status(), 200);
    let transaction_list2: TransactionListResponse = response2.json().await.unwrap();
    assert_eq!(transaction_list2.len(), 1);
}

#[actix_web::test]
async fn test_list_transactions_version_field_comprehensive() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts
    let account_id1 = create_test_account(
        &client,
        &token,
        "Version Test Account 1",
        BalanceDirection::Debit,
    )
    .await;
    let account_id2 = create_test_account(
        &client,
        &token,
        "Version Test Account 2",
        BalanceDirection::Credit,
    )
    .await;

    // Create recon rules first
    let rule_001 = create_test_recon_rule(
        &client,
        &token,
        "rule_001",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_002 = create_test_recon_rule(
        &client,
        &token,
        "rule_002",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_003 = create_test_recon_rule(
        &client,
        &token,
        "rule_003",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transactions with different statuses
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_001,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_002,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Expected,
        None,
    )
    .await;
    create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_003,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 20.0 }),
    )
    .await;

    // List all transactions
    let response = client.get("/transactions").with_auth(&token).send().await;

    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 4); // 3 created + 1 from mismatched processing

    // Verify version field is present and correct for all transactions and entries
    for transaction in &transaction_list {
        // Transaction version should be 1
        if transaction.status
            == ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
        {
            // Mismatched transactions may have higher version due to processing
            assert!(transaction.version == 2);
        } else {
            assert_eq!(transaction.version, 1);
        }

        // Verify transaction structure
        assert!(transaction.id.starts_with("txn_"));
        assert!(!transaction.transaction_id.is_empty());
    }
}

#[actix_web::test]
async fn test_list_transactions_empty_results() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Test list without any transactions
    let response = client.get("/transactions").with_auth(&token).send().await;

    assert_eq!(response.status(), 200);
    let transaction_list: TransactionListResponse = response.json().await.unwrap();
    assert_eq!(transaction_list.len(), 0);
}

#[actix_web::test]
async fn test_list_transactions_unauthorized() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Test without authentication
    let response = client.get("/transactions").send().await;

    assert_eq!(response.status(), 401);
}

#[actix_web::test]
async fn create_transaction_multiple_entries_balanced_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_multiple_balanced",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction with multiple entries that are balanced
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(75.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(25.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
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

    // Should succeed with 201 Created since credits (100) = debits (75+25=100)
    assert_eq!(response.status(), 201);

    let transaction: GetTransactionResponse = response.json().await.unwrap();
    assert!(transaction.id.starts_with("txn_"));
    assert_eq!(
        transaction.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
    );
    assert_eq!(transaction.entry_id.len(), 3);
}

#[actix_web::test]
async fn create_transaction_mixed_entry_types_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_mixed_types",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction with mixed entry types (Direct + Expected)
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Expected(ExpectedTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                search_expectation: SearchExpectation {
                    rule_id: rule_id.clone(),
                    source_value: "payment_123".to_string(),
                    target_key: "metadata.transaction_id".parse().unwrap(),
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

    // Should succeed and return Expected status due to Expected entries
    assert_eq!(response.status(), 201);

    let transaction: GetTransactionResponse = response.json().await.unwrap();
    assert!(transaction.id.starts_with("txn_"));
    assert_eq!(transaction.status, ApiTransactionStatus::Expected);
    assert_eq!(transaction.entry_id.len(), 2);
    assert_eq!(transaction.version, 1);
}

#[actix_web::test]
async fn create_transaction_with_expected_entries_only() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create recon rule
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_expected_only",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction with only Expected entries
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Expected(ExpectedTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                search_expectation: SearchExpectation {
                    rule_id: rule_id.clone(),
                    source_value: "payment_123".to_string(),
                    target_key: "metadata.transaction_id".parse().unwrap(),
                },
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Expected(ExpectedTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                search_expectation: SearchExpectation {
                    rule_id: rule_id.clone(),
                    source_value: "payment_456".to_string(),
                    target_key: "metadata.transaction_id".parse().unwrap(),
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

    // Should succeed and return Expected status
    assert_eq!(response.status(), 201);

    let transaction: GetTransactionResponse = response.json().await.unwrap();
    assert!(transaction.id.starts_with("txn_"));
    assert_eq!(transaction.status, ApiTransactionStatus::Expected);
    assert_eq!(transaction.entry_id.len(), 2);
    assert_eq!(transaction.version, 1);
}

#[actix_web::test]
async fn create_transaction_mixed_entry_types_unbalanced_returns_error() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        "test_rule_mixed_unbalanced",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create transaction with mixed entry types (Direct + Expected) that are UNBALANCED
    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                staging_entry_id: None,
                transformation_id: None,
                order_id: "order_test".to_string(),
            }),
            TransactionEntry::Expected(ExpectedTransactionEntry {
                account_id: account_id2.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(75.0), // Unbalanced: 100 debit vs 75 credit
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({"test": "data"})),
                search_expectation: SearchExpectation {
                    rule_id: recon_rule.clone(),
                    source_value: "payment_123".to_string(),
                    target_key: "metadata.transaction_id".parse().unwrap(),
                },
                order_id: "order_test".to_string(),
            }),
        ],
        rule_id: recon_rule.clone(),
    };

    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    // Should throw error due to unbalanced entries
    assert_eq!(response.status(), 400);
}

// ================================================================================================
// VOID TRANSACTION API TESTS
// ================================================================================================

#[actix_web::test]
async fn void_transaction_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        "test_rule_void",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create a transaction first
    let original_transaction = create_test_transaction(
        &client,
        &token,
        &account_id,
        &account_id2,
        &recon_rule,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;

    // Void the transaction
    let void_request = VoidTransactionRequest {
        reason: Some("Test void transaction".to_string()),
    };

    let response = client
        .put(&format!("/transactions/{}/void", original_transaction.id))
        .with_auth(&token)
        .json(&void_request)
        .send()
        .await;

    assert_eq!(response.status(), 200);

    let void_transaction: GetTransactionResponse = response.json().await.unwrap();

    // Verify void transaction properties
    assert!(void_transaction.id.starts_with("txn_"));
    assert_ne!(void_transaction.id, original_transaction.id); // Different primary key
    assert_eq!(
        void_transaction.transaction_id,
        original_transaction.transaction_id
    ); // Same logical ID
    assert_eq!(void_transaction.status, ApiTransactionStatus::Void);
    assert_eq!(void_transaction.version, original_transaction.version + 1);
    assert_eq!(
        void_transaction.entry_id.len(),
        original_transaction.entry_id.len()
    );

    // Verify original transaction is archived
    let get_original_response = client
        .get(&format!("/transactions/{}", original_transaction.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(get_original_response.status(), 200);
    let archived_transaction: GetTransactionResponse = get_original_response.json().await.unwrap();
    assert_eq!(archived_transaction.status, ApiTransactionStatus::Archived);
    assert!(archived_transaction.discarded_at.is_some());
    assert!(archived_transaction.discarded_status.is_some());
}

#[actix_web::test]
async fn void_transaction_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to void a non-existent transaction
    let void_request = VoidTransactionRequest {
        reason: Some("Test void non-existent transaction".to_string()),
    };

    let response = client
        .put("/transactions/txn_nonexistent/void")
        .with_auth(&token)
        .json(&void_request)
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

#[actix_web::test]
async fn test_search_duplicate_after_void() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    create_test_recon_rule_for_staging_entry(
        &client,
        &token,
        "processed_test_rule",
        source_account_id.clone(),
        target_account_id.clone(),
    )
    .await;

    let transformation = create_transformation_config(
        &profile_id,
        &source_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Create staging entry
    let mut staging_entry_request = staging_entry_request(
        &source_account_id,
        BalanceDirection::Credit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation.transformation_id.clone(),
    );
    staging_entry_request.transformation_id = transformation.transformation_id;
    staging_entry_request.metadata =
        Secret::new(json!({"transaction_id": "test_txn_123", "test": "data"}));
    let create_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    let staging_entry: GetStagingEntryResponse = create_response.json().await.unwrap();

    // Process the staging entry
    let process_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(process_response.status(), 200);
    let recon_trigger_response: ReconTriggerResponse = process_response.json().await.unwrap();

    let transaction_id = match recon_trigger_response {
        ReconTriggerResponse::Success(transaction) => {
            assert_eq!(transaction.status, ApiTransactionStatus::Expected);
            transaction.id
        }
        ReconTriggerResponse::Failure(error) => {
            panic!("{error}");
        }
    };

    // Create duplicate staging entry
    let create_response_duplicate = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request)
        .send()
        .await;

    assert_eq!(create_response_duplicate.status(), 201);
    let staging_entry_duplicate: GetStagingEntryResponse =
        create_response_duplicate.json().await.unwrap();

    // Process the duplicate staging entry
    let process_response_duplicate = client
        .post(&format!(
            "/recon_engine/{}/process",
            staging_entry_duplicate.id
        ))
        .with_auth(&token)
        .send()
        .await;

    // Should fail with duplicate transaction
    assert_eq!(process_response_duplicate.status(), 200);
    let recon_trigger_response_duplicate: ReconTriggerResponse =
        process_response_duplicate.json().await.unwrap();

    match recon_trigger_response_duplicate {
        ReconTriggerResponse::Success(_) => {
            panic!("The duplicate transaction should not be created");
        }
        ReconTriggerResponse::Failure(error) => {
            assert_eq!(error, "Duplicate entry detected");
        }
    }

    // Void the transaction created.
    let void_response = client
        .put(&format!("/transactions/{transaction_id}/void"))
        .with_auth(&token)
        .json(&VoidTransactionRequest {
            reason: Some("Test void transaction".to_string()),
        })
        .send()
        .await;

    assert_eq!(void_response.status(), 200);

    // Create duplicate staging entry again
    let create_response_duplicate2 = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request)
        .send()
        .await;

    assert_eq!(create_response_duplicate2.status(), 201);
    let staging_entry_duplicate2: GetStagingEntryResponse =
        create_response_duplicate2.json().await.unwrap();

    // Process the duplicate staging entry again
    let process_response_duplicate2 = client
        .post(&format!(
            "/recon_engine/{}/process",
            staging_entry_duplicate2.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(process_response_duplicate2.status(), 200);
    let recon_trigger_response_duplicate2: ReconTriggerResponse =
        process_response_duplicate2.json().await.unwrap();

    // Should process since search duplicate is archived
    match recon_trigger_response_duplicate2 {
        ReconTriggerResponse::Success(transaction) => {
            assert_eq!(transaction.status, ApiTransactionStatus::Expected);
        }
        ReconTriggerResponse::Failure(error) => {
            panic!("{error}");
        }
    }
}

#[actix_web::test]
async fn test_search_duplicate_for_composite_unique_constraint() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    create_test_recon_rule_for_staging_entry(
        &client,
        &token,
        "processed_test_rule",
        source_account_id.clone(),
        target_account_id.clone(),
    )
    .await;

    let ingestion_config =
        create_ingestion_config(&source_account_id, &profile_id, &merchant_id, &client).await;

    let string_field = || FieldType::String {
        validation_rules: vec![],
        transformation_rules: vec![],
    };

    // Same base config `create_transformation_config` builds, but with a composite
    // (customer_id + order_ref) unique constraint instead of the default single-field one.
    let mut config_request = transformation_config_request(
        &merchant_id,
        &profile_id,
        &source_account_id,
        &ingestion_config.ingestion_id,
        "order_id".parse().unwrap(), // placeholder; overwritten below
        StagingEntryProcessingMode::Transaction,
        vec![
            (
                "metadata.customer_id".parse().unwrap(),
                true,
                string_field(),
                None,
            ),
            (
                "metadata.order_ref".parse().unwrap(),
                true,
                string_field(),
                None,
            ),
        ],
        vec![],
    );
    let MetadataSchemaData::BasicSchemaData {
        unique_constraint, ..
    } = &mut config_request.metadata_schema_data;
    *unique_constraint = UniqueConstraint {
        constraint_type: UniqueConstraintType::CompositeField {
            field_names: vec![
                "metadata.customer_id".parse().unwrap(),
                "metadata.order_ref".parse().unwrap(),
            ],
            delimiter: CompositeDelimiter::Pipe,
        },
        description: "customer_id + order_ref must be unique per account".to_string(),
    };

    let config_response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&config_request)
        .send()
        .await;
    assert_eq!(config_response.status(), 201);
    let transformation: TransformationConfigResponse = config_response.json().await.unwrap();

    let mut first_staging_entry_request = staging_entry_request(
        &source_account_id,
        BalanceDirection::Credit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation.transformation_id.clone(),
    );
    first_staging_entry_request.metadata = Secret::new(
        json!({"customer_id": "cust_1", "order_ref": "ord_9", "test": "data", "transaction_id": "txn_composite_1"}),
    );

    let create_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&first_staging_entry_request)
        .send()
        .await;
    assert_eq!(create_response.status(), 201);
    let staging_entry: GetStagingEntryResponse = create_response.json().await.unwrap();

    let process_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(process_response.status(), 200);
    match process_response
        .json::<ReconTriggerResponse>()
        .await
        .unwrap()
    {
        ReconTriggerResponse::Success(transaction) => {
            assert_eq!(transaction.status, ApiTransactionStatus::Expected);
        }
        ReconTriggerResponse::Failure(error) => panic!("{error}"),
    }

    // Second staging entry: identical (customer_id, order_ref) pair — the composite key, not
    // either field alone, must be what's checked for uniqueness.
    let create_response_duplicate = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&first_staging_entry_request)
        .send()
        .await;
    assert_eq!(create_response_duplicate.status(), 201);
    let staging_entry_duplicate: GetStagingEntryResponse =
        create_response_duplicate.json().await.unwrap();

    let process_response_duplicate = client
        .post(&format!(
            "/recon_engine/{}/process",
            staging_entry_duplicate.id
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(process_response_duplicate.status(), 200);
    match process_response_duplicate
        .json::<ReconTriggerResponse>()
        .await
        .unwrap()
    {
        ReconTriggerResponse::Success(_) => {
            panic!("Duplicate composite-key staging entry should not create a transaction")
        }
        ReconTriggerResponse::Failure(error) => assert_eq!(error, "Duplicate entry detected"),
    }

    let manual_review_page = list_staging_entries_v2(
        &client,
        &token,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2 {
                detailed_status: Some(vec![ApiStagingEntryStatus::NeedsManualReview(Some(
                    StagingEntryManualReviewData::DuplicateEntry,
                ))]),
                account_ids: Some(vec![source_account_id.clone()]),
                ..Default::default()
            },
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;
    let duplicate_entry = manual_review_page
        .items
        .iter()
        .find(|entry| entry.staging_entry_id == staging_entry_duplicate.staging_entry_id)
        .expect("duplicate composite-key entry should be NeedsManualReview/DuplicateEntry");
    assert_eq!(
        duplicate_entry.status,
        ApiStagingEntryCoarseStatus::NeedsManualReview
    );
    assert_eq!(
        duplicate_entry.detailed_status,
        ApiStagingEntryStatus::NeedsManualReview(Some(
            StagingEntryManualReviewData::DuplicateEntry
        ))
    );

    let mut staging_entry_request_partial = staging_entry_request(
        &source_account_id,
        BalanceDirection::Credit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation.transformation_id.clone(),
    );
    staging_entry_request_partial.metadata = Secret::new(
        json!({"customer_id": "cust_1", "order_ref": "ord_different", "test": "data", "transaction_id": "txn_composite_2"}),
    );

    let create_response_partial = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request_partial)
        .send()
        .await;
    assert_eq!(create_response_partial.status(), 201);
    let staging_entry_partial: GetStagingEntryResponse =
        create_response_partial.json().await.unwrap();

    let process_response_partial = client
        .post(&format!(
            "/recon_engine/{}/process",
            staging_entry_partial.id
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(process_response_partial.status(), 200);
    match process_response_partial
        .json::<ReconTriggerResponse>()
        .await
        .unwrap()
    {
        ReconTriggerResponse::Success(transaction) => {
            assert_eq!(transaction.status, ApiTransactionStatus::Expected);
        }
        ReconTriggerResponse::Failure(error) => {
            panic!("Partial-overlap entry should not be treated as a duplicate: {error}")
        }
    }
}

// Test transaction resolutions endpoint for different status scenarios
#[rstest]
#[case("posted",ApiTransactionStatus::Matched(TransactionMatchedData::Auto), vec![ResolutionActionType::VoidTransaction])] // Posted transactions can be voided
#[case("expected", ApiTransactionStatus::Expected, vec![ResolutionActionType::ReplaceEntries, ResolutionActionType::VoidTransaction, ResolutionActionType::LinkStagingEntriesToTransaction, ResolutionActionType::CreateEntries])] // Expected transactions can be voided
#[case("under_amount(expected)", ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected), vec![ResolutionActionType::CreateEntries, ResolutionActionType::ReplaceEntries, ResolutionActionType::VoidTransaction, ResolutionActionType::ForceReconcile, ResolutionActionType::LinkStagingEntriesToTransaction])] // Mismatched transactions can be voided // Mismatched transactions can be voided
#[case("archived",ApiTransactionStatus::Archived, vec![])] // Archived transactions have no resolutions
#[case("void", ApiTransactionStatus::Void, vec![])] // Void transactions have no resolutions
#[case("partially_reconciled", ApiTransactionStatus::PartiallyReconciled, vec![ResolutionActionType::CreateEntries, ResolutionActionType::ReplaceEntries, ResolutionActionType::VoidTransaction, ResolutionActionType::LinkStagingEntriesToTransaction])]
#[actix_web::test]
async fn test_get_transaction_resolutions(
    #[case] test_name: &str,
    #[case] target_status: ApiTransactionStatus,
    #[case] expected_resolutions: Vec<ResolutionActionType>,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id1 =
        create_test_account(&client, &token, "Test Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        &format!("test_rule_{test_name}"),
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let original_transaction = create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_id,
        &merchant_id,
        &profile_id,
        target_status.clone(),
        Some(DataMismatchType::Metadata),
    )
    .await;

    let resolutions_response = client
        .get(&format!(
            "/exception_management/transactions/{}/resolutions",
            original_transaction.id.clone()
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(resolutions_response.status(), 200);
    let resolutions: Vec<ResolutionActionType> = resolutions_response.json().await.unwrap();

    // Assert that the resolutions match expected
    assert_eq!(
        resolutions.len(),
        expected_resolutions.len(),
        "Resolution count mismatch for test case: {} (status: {:?})",
        test_name,
        target_status
    );

    for expected_resolution in expected_resolutions {
        assert!(
            resolutions.contains(&expected_resolution),
            "Missing resolution {expected_resolution:?} for test case: {test_name} (status: {target_status:?})",
        );
    }
}

#[actix_web::test]
async fn test_get_transaction_resolutions_with_mismatch_non_force_reconciliable() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id1 =
        create_test_account(&client, &token, "Test Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        &format!("test_rule_{}", "mismatched_non_force_reconciliable"),
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let original_transaction = create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 20.0 }),
    )
    .await;

    let resolutions_response = client
        .get(&format!(
            "/exception_management/transactions/{}/resolutions",
            original_transaction.id.clone()
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(resolutions_response.status(), 200);
    let resolutions: Vec<ResolutionActionType> = resolutions_response.json().await.unwrap();

    assert_eq!(resolutions.len(), 4);
    assert!(resolutions.contains(&ResolutionActionType::CreateEntries));
    assert!(resolutions.contains(&ResolutionActionType::LinkStagingEntriesToTransaction));
    assert!(resolutions.contains(&ResolutionActionType::ReplaceEntries));
    assert!(resolutions.contains(&ResolutionActionType::VoidTransaction));
    assert!(!resolutions.contains(&ResolutionActionType::ForceReconcile));
}

// ================================================================================================
// FORCE RECONCILE TRANSACTION API TESTS
// ================================================================================================

#[actix_web::test]
async fn force_reconcile_metadata_mismatch_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Step 1: Create source and target accounts
    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;

    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    // Step 2: Create recon rule with metadata match requirements
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "rule_001",
        source_account_id.clone(),
        target_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    //Step 3: Create Transaction with Mismatched status to be force reconciled later
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

    // Step 7: FORCE RECONCILE the metadata mismatch (this is what we're testing)
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Manually resolving metadata mismatch after business validation".to_string()),
    };

    let response = client
        .put(&format!(
            "/exception_management/transactions/{}/force_reconcile",
            mismatched_transaction.id
        ))
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let force_reconciled_transaction: GetTransactionResponse = response.json().await.unwrap();

    // Step 8: Verify force reconcile was successful
    assert_eq!(
        force_reconciled_transaction.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Force),
    );
    assert_eq!(
        force_reconciled_transaction.version,
        mismatched_transaction.version + 1
    ); // Version incremented
    assert_eq!(
        force_reconciled_transaction.transaction_id,
        mismatched_transaction.transaction_id
    ); // Same logical transaction
    assert_ne!(force_reconciled_transaction.id, mismatched_transaction.id); // Different primary key
    assert_eq!(force_reconciled_transaction.entry_id.len(), 2); // Still has both entries

    // Step 9: Verify original mismatched transaction is now archived
    let get_original_response = client
        .get(&format!("/transactions/{}", mismatched_transaction.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(get_original_response.status(), 200);
    let archived_transaction: GetTransactionResponse = get_original_response.json().await.unwrap();
    assert_eq!(archived_transaction.status, ApiTransactionStatus::Archived);
    assert!(archived_transaction.discarded_at.is_some());
    assert!(archived_transaction.discarded_status.is_some());
}

#[actix_web::test]
async fn force_reconcile_non_mismatched_transaction_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        "test_rule_posted",
        account_id.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    // Create a Posted transaction (not mismatched)
    let posted_transaction = create_test_transaction(
        &client,
        &token,
        &account_id,
        &account_id2,
        &recon_rule,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
        None,
    )
    .await;

    // Verify it's Posted (not Mismatched)
    assert_eq!(
        posted_transaction.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
    );

    // Try to force reconcile a Posted transaction - should fail
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Should not work on non-mismatched transaction".to_string()),
    };

    let response = client
        .put(&format!(
            "/exception_management/transactions/{}/force_reconcile",
            posted_transaction.id
        ))
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;

    // Should fail with 400 Bad Request
    assert_eq!(response.status(), 400);

    // Verify error message
    let error: serde_json::Value = response.json().await.unwrap();
    let error_message = error["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Invalid transaction status"));
    assert!(error_message.contains("Mismatch"));
}

#[actix_web::test]
async fn force_reconcile_unbalanced_transaction_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test account
    let account_id1 =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "test_rule_unbalanced",
        account_id1.clone(),
        account_id2.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let unbalanced_transaction = create_test_transaction(
        &client,
        &token,
        &account_id1,
        &account_id2,
        &rule_id,
        &merchant_id,
        &profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 25.0 }),
    )
    .await;
    assert_eq!(
        unbalanced_transaction.status,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
    );

    // Try to force reconcile the unbalanced transaction - should fail
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Trying to force reconcile unbalanced transaction".to_string()),
    };

    let response = client
        .put(&format!(
            "/exception_management/transactions/{}/force_reconcile",
            unbalanced_transaction.id
        ))
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;

    // Should fail with 400 Bad Request
    assert_eq!(response.status(), 400);

    // Verify error message mentions unbalanced transaction
    let error: serde_json::Value = response.json().await.unwrap();
    let error_message = error["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Invalid transaction status"));
}

#[actix_web::test]
async fn force_reconcile_transaction_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to force reconcile a non-existent transaction
    let force_reconcile_request = ForceReconcileRequest {
        reason: Some("Test force reconcile non-existent transaction".to_string()),
    };

    let response = client
        .put("/exception_management/transactions/txn_nonexistent/force_reconcile")
        .with_auth(&token)
        .json(&force_reconcile_request)
        .send()
        .await;

    assert_eq!(response.status(), 404);
}
#[rstest]
#[case::no_aging_config(
    AgingConfigApiRequest::NoAging,
    0,
    ApiTransactionStatus::Expected,
    ApiTransactionFilterStatus::Expected
)]
#[case::within_threshold(
    AgingConfigApiRequest::WithThreshold {
        threshold: ThresholdApiRequest::WeekDays { value: 5 },
    },
    2,
    ApiTransactionStatus::Expected,
    ApiTransactionFilterStatus::Expected
)]
#[case::exceeds_threshold(
    AgingConfigApiRequest::WithThreshold {
        threshold: ThresholdApiRequest::WeekDays { value: 5 },
    },
    10,
    ApiTransactionStatus::Missing,
    ApiTransactionFilterStatus::Missing
)]
#[actix_web::test]
async fn test_transaction_aging_with_week_days(
    #[case] aging_config: AgingConfigApiRequest,
    #[case] days_back: i64,
    #[case] expected_api_status: ApiTransactionStatus,
    #[case] expected_filter_status: ApiTransactionFilterStatus,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) = client
        .bootstrap("Test Merchant", "Aging Test Profile")
        .await;

    let source_account =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
    let target_account =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        &format!("aging_test_{}", uuid::Uuid::new_v4()),
        source_account.clone(),
        target_account.clone(),
        aging_config.clone(),
    )
    .await;

    // Calculate effective_at by going back the specified number of calendar days from now
    // This ensures the test works with the real system time used in production code
    let effective_at = if days_back > 0 {
        common_utils::date_time::now() - time::Duration::days(days_back)
    } else {
        common_utils::date_time::now()
    };

    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: source_account.to_string(),
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
                account_id: target_account.to_string(),
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
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let created_transaction: GetTransactionResponse = response.json().await.unwrap();
    assert_eq!(created_transaction.status, expected_api_status);

    let filter_response = client
        .get(&format!("/transactions?status={}", expected_filter_status))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(filter_response.status(), 200);
    let filter_transactions: TransactionListResponse = filter_response.json().await.unwrap();

    assert_eq!(filter_transactions.len(), 1);
    assert_eq!(filter_transactions[0].id, created_transaction.id);
    assert_eq!(filter_transactions[0].status, expected_api_status);

    let v2_filter_transactions = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2 {
                status: Some(vec![expected_filter_status]),
                ..Default::default()
            },
            TransactionListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(v2_filter_transactions.items.len(), 1);
    assert_eq!(v2_filter_transactions.items[0].id, created_transaction.id);
    assert_eq!(v2_filter_transactions.items[0].status, expected_api_status);

    let get_response = client
        .get(&format!("/transactions/{}", created_transaction.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(get_response.status(), 200);
    let get_transaction: GetTransactionResponse = get_response.json().await.unwrap();
    assert_eq!(
        get_transaction.status, expected_api_status,
        "GET endpoint: Status mismatch for aging_config={:?}",
        aging_config
    );
}

#[actix_web::test]
async fn create_transaction_populates_exposed_metadata_and_search_entry_keys() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id =
        create_test_account(&client, &token, "Debit Account", BalanceDirection::Debit).await;
    let credit_account_id =
        create_test_account(&client, &token, "Credit Account", BalanceDirection::Credit).await;

    let string_field = || FieldType::String {
        validation_rules: vec![],
        transformation_rules: vec![],
    };

    let transformation = create_transformation_config(
        &profile_id,
        &debit_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![
            (
                "metadata.psp_ref".parse().unwrap(),
                false,
                string_field(),
                Some(MetadataExposeConfig {
                    is_searchable: true,
                }),
            ),
            (
                "metadata.merchant_ref".parse().unwrap(),
                false,
                string_field(),
                Some(MetadataExposeConfig {
                    is_searchable: false,
                }),
            ),
            (
                "metadata.internal_token".parse().unwrap(),
                false,
                string_field(),
                None,
            ),
        ],
    )
    .await;

    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "search_entry_key_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let transaction_request = TransactionRequest {
        entries: vec![
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: debit_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({
                    "psp_ref": "PSP_DEBIT_1",
                    "merchant_ref": "MERCHANT_DEBIT_1",
                    "internal_token": "tok_debit_secret",
                    "raw_only": "not-in-schema",
                })),
                staging_entry_id: None,
                transformation_id: Some(transformation.transformation_id.clone()),
                order_id: "order_debit_1".to_string(),
            }),
            TransactionEntry::Direct(DirectTransactionEntry {
                account_id: credit_account_id.clone(),
                entry_type: BalanceDirection::Credit,
                amount: FloatMajorUnit::new(100.0),
                currency: Currency::USD,
                effective_at: common_utils::date_time::now(),
                metadata: Secret::new(json!({
                    "psp_ref": "PSP_CREDIT_1",
                    "merchant_ref": "MERCHANT_CREDIT_1",
                    "internal_token": "tok_credit_secret",
                    "raw_only": "not-in-schema",
                })),
                staging_entry_id: None,
                transformation_id: Some(transformation.transformation_id.clone()),
                order_id: "order_credit_1".to_string(),
            }),
        ],
        rule_id,
    };

    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let transaction: GetTransactionResponse = response.json().await.unwrap();
    assert_eq!(transaction.entry_id.len(), 2);

    for entry_id in &transaction.entry_id {
        let entry_response = client
            .get(&format!("/entries/{entry_id}"))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(entry_response.status(), 200);
        let entry: GetEntryResponse = entry_response.json().await.unwrap();

        assert_eq!(
            entry.transformation_id.as_deref(),
            Some(transformation.transformation_id.as_str())
        );
        assert!(
            entry.exposed_metadata.get("psp_ref").is_some(),
            "exposed+searchable key psp_ref must be in exposed_metadata"
        );
        assert!(
            entry.exposed_metadata.get("merchant_ref").is_some(),
            "exposed-but-not-searchable key merchant_ref must still be in exposed_metadata"
        );
        assert!(
            entry.exposed_metadata.get("internal_token").is_none(),
            "non-exposed schema field internal_token must be projected out"
        );
        assert!(
            entry.exposed_metadata.get("raw_only").is_none(),
            "undeclared key raw_only must be projected out"
        );
    }

    let search_keys_response = client
        .post("/search_entry_keys")
        .with_auth(&token)
        .json(&SearchEntryKeyListRequest {
            entry_ids: transaction.entry_id.clone(),
        })
        .send()
        .await;
    assert_eq!(search_keys_response.status(), 200);
    let search_keys: Vec<SearchEntryKeyResponse> = search_keys_response.json().await.unwrap();

    assert_eq!(
        search_keys.len(),
        2,
        "only the searchable key psp_ref should produce a row per entry"
    );

    for key in &search_keys {
        assert!(transaction.entry_id.contains(&key.entry_id));
        assert_eq!(
            key.metadata_key, "psp_ref",
            "only searchable keys should be indexed"
        );
        assert!(key.metadata_value.is_some());
    }

    let mut psp_values: Vec<String> = search_keys
        .iter()
        .filter_map(|key| key.metadata_value.clone())
        .collect();
    psp_values.sort();
    assert_eq!(
        psp_values,
        vec!["PSP_CREDIT_1".to_string(), "PSP_DEBIT_1".to_string()]
    );

    let empty_response = client
        .post("/search_entry_keys")
        .with_auth(&token)
        .json(&SearchEntryKeyListRequest { entry_ids: vec![] })
        .send()
        .await;
    assert_eq!(empty_response.status(), 200);
    let empty_keys: Vec<SearchEntryKeyResponse> = empty_response.json().await.unwrap();
    assert!(empty_keys.is_empty());
}

#[actix_web::test]
async fn test_list_transactions_v2_caps_every_transaction_on_a_full_page() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (token, debit_account, credit_account, rule_id) =
        bootstrap_for_entry_cap(&client, "v2_entry_full_page").await;

    for index in 0..12 {
        create_transaction_with_entry_counts(
            &client,
            &token,
            &debit_account,
            &credit_account,
            &rule_id,
            &format!("v2_full_page_order_{index}"),
            common_utils::date_time::now(),
            4,
            4,
        )
        .await;
    }

    let page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2::default(),
            TransactionListSortField::Id { id: None },
            Some(12),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(page.items.len(), 12);
    for item in &page.items {
        assert_eq!(item.entries.len(), 5);
        assert!(item.has_more_entries);
    }
}

/// Leg 1 (rule A) has an Expected placeholder on the link account created *before* the real
/// entry. Linking must skip the placeholder — its metadata is a projection of the source, not a
/// real value — and resolve the link through the real entry to the leg 2 transaction.
#[actix_web::test]
async fn test_linked_transaction_skips_expected_placeholder_entry() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let leg1_debit =
        create_test_account(&client, &token, "Leg1 Debit", BalanceDirection::Debit).await;
    let leg1_credit =
        create_test_account(&client, &token, "Leg1 Credit", BalanceDirection::Credit).await;
    let leg2_debit =
        create_test_account(&client, &token, "Leg2 Debit", BalanceDirection::Debit).await;
    let leg2_credit =
        create_test_account(&client, &token, "Leg2 Credit", BalanceDirection::Credit).await;

    let rule_a = create_test_recon_rule(
        &client,
        &token,
        "link_leg_1",
        leg1_debit.clone(),
        leg1_credit.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;
    let rule_b = create_test_recon_rule(
        &client,
        &token,
        "link_leg_2",
        leg2_debit.clone(),
        leg2_credit.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let link_value = "settlement_ref_42";
    let link_key = "metadata.settlement_ref";

    let update_response = client
        .put(&format!("/recon_rules/{rule_a}"))
        .with_auth(&token)
        .json(&ReconRuleUpdateRequest {
            rule_name: None,
            rule_description: None,
            link_with_rule: Some(LinkWithRuleUpdate::Set {
                data: LinkWithRule::V1(LinkWithRuleV1 {
                    target_data: vec![LinkWithRuleTargetData {
                        rule_id: rule_b.clone(),
                        account_id: leg2_debit.clone(),
                        key: EntryField::Metadata("settlement_ref".to_string()),
                    }],
                    current_rule_account_id: leg1_credit.clone(),
                    current_rule_field: EntryField::Metadata("settlement_ref".to_string()),
                }),
            }),
            aging_config: None,
            priority: None,
            is_active: None,
            strategy: None,
        })
        .send()
        .await;
    assert_eq!(update_response.status(), 200);

    let leg2_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&TransactionRequest {
            entries: vec![
                TransactionEntry::Expected(ExpectedTransactionEntry {
                    account_id: leg2_debit.clone(),
                    entry_type: BalanceDirection::Debit,
                    amount: FloatMajorUnit::new(100.0),
                    currency: Currency::USD,
                    effective_at: common_utils::date_time::now(),
                    metadata: Secret::new(json!({})),
                    search_expectation: SearchExpectation {
                        rule_id: rule_b.clone(),
                        source_value: link_value.to_string(),
                        target_key: link_key.parse().unwrap(),
                    },
                    order_id: "leg2_order".to_string(),
                }),
                TransactionEntry::Direct(DirectTransactionEntry {
                    account_id: leg2_credit.clone(),
                    entry_type: BalanceDirection::Credit,
                    amount: FloatMajorUnit::new(100.0),
                    currency: Currency::USD,
                    effective_at: common_utils::date_time::now(),
                    metadata: Secret::new(json!({})),
                    staging_entry_id: None,
                    transformation_id: None,
                    order_id: "leg2_order".to_string(),
                }),
            ],
            rule_id: rule_b.clone(),
        })
        .send()
        .await;
    assert_eq!(leg2_response.status(), 201);
    let leg2: GetTransactionResponse = leg2_response.json().await.unwrap();

    let leg1_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&TransactionRequest {
            entries: vec![
                TransactionEntry::Direct(DirectTransactionEntry {
                    account_id: leg1_debit.clone(),
                    entry_type: BalanceDirection::Debit,
                    amount: FloatMajorUnit::new(100.0),
                    currency: Currency::USD,
                    effective_at: common_utils::date_time::now(),
                    metadata: Secret::new(json!({})),
                    staging_entry_id: None,
                    transformation_id: None,
                    order_id: "leg1_order".to_string(),
                }),
                TransactionEntry::Expected(ExpectedTransactionEntry {
                    account_id: leg1_credit.clone(),
                    entry_type: BalanceDirection::Credit,
                    amount: FloatMajorUnit::new(40.0),
                    currency: Currency::USD,
                    effective_at: common_utils::date_time::now(),
                    metadata: Secret::new(json!({"settlement_ref": "placeholder_not_real"})),
                    search_expectation: SearchExpectation {
                        rule_id: rule_a.clone(),
                        source_value: "unrelated".to_string(),
                        target_key: "metadata.other".parse().unwrap(),
                    },
                    order_id: "leg1_order".to_string(),
                }),
                TransactionEntry::Direct(DirectTransactionEntry {
                    account_id: leg1_credit.clone(),
                    entry_type: BalanceDirection::Credit,
                    amount: FloatMajorUnit::new(60.0),
                    currency: Currency::USD,
                    effective_at: common_utils::date_time::now(),
                    metadata: Secret::new(json!({"settlement_ref": link_value})),
                    staging_entry_id: None,
                    transformation_id: None,
                    order_id: "leg1_order".to_string(),
                }),
            ],
            rule_id: rule_a.clone(),
        })
        .send()
        .await;
    assert_eq!(leg1_response.status(), 201);
    let leg1: GetTransactionResponse = leg1_response.json().await.unwrap();

    let page = list_transactions_v2(
        &client,
        &token,
        list_transactions_v2_request(
            ApiTransactionListFiltersV2 {
                rule_id: Some(rule_a.clone()),
                ..Default::default()
            },
            TransactionListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, leg1.id);

    let linked = page.items[0]
        .linked_transaction
        .as_ref()
        .expect("leg 1 must link to leg 2 through the real (non-expected) entry");
    assert_eq!(linked.id, leg2.id);
}
