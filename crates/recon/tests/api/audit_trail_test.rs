use crate::{
    api::{
        account_test::create_test_account, transaction_test::create_test_recon_rule,
        transformation_test::create_transformation_config,
    },
    common::TestApp,
};
use api_models::recon_rule::aging_config::AgingConfigApiRequest;
use api_models::{
    audit_trail::AuditEvent,
    ingestion::{IngestionConfigCreateRequest, IngestionConfigData, IngestionConfigResponse},
    staging_entry::StagingEntryCreateRequest,
    transaction::ApiTransactionStatus,
};
use common_enums::{BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::{transaction::status::TransactionMatchedData, units::FloatMajorUnit};
use common_utils::id_type::MerchantId;
use masking::Secret;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::json;

// ========== Test Helper Functions ==========

async fn create_manual_ingestion_config(
    client: &crate::common::TestClient,
    merchant_id: MerchantId,
    profile_id: &str,
    account_id: &str,
    name: &str,
) -> IngestionConfigResponse {
    let request = IngestionConfigCreateRequest {
        merchant_id,
        profile_id: profile_id.to_string(),
        name: name.to_string(),
        description: None,
        account_id: account_id.to_string(),
        config: IngestionConfigData::Manual,
        is_active: true,
    };

    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    client.expect_success(response).await
}

async fn upload_file(
    client: &crate::common::TestClient,
    token: &str,
    ingestion_id: &str,
    file_content: &[u8],
    file_name: &str,
) {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
         Content-Type: text/csv\r\n\r\n\
         {}\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        file_name,
        std::str::from_utf8(file_content).unwrap()
    );

    let response = client
        .post(&format!("/ingestions/{ingestion_id}/upload"))
        .with_auth(token)
        .header(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .body(&body)
        .send()
        .await;

    assert_eq!(response.status(), 200);
}

async fn create_staging_entry(
    client: &crate::common::TestClient,
    token: &str,
    account_id: &str,
    transformation_id: &str,
    amount: f64,
) {
    let request = StagingEntryCreateRequest {
        account_id: account_id.to_string(),
        entry_type: BalanceDirection::Debit,
        processing_mode: StagingEntryProcessingMode::Transaction,
        amount: FloatMajorUnit::new(amount),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"transaction_id": format!("txn_{}", amount), "test": "data"})),
        transformation_id: transformation_id.to_string(),
        transformation_history_id: None,
        order_id: format!("order_{amount}"),
    };

    let response = client
        .post("/staging_entries")
        .with_auth(token)
        .json(&request)
        .send()
        .await;

    if response.status() != 201 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        panic!("Failed to create staging entry. Status: {status}, Body: {body}");
    }
}

// No helper needed - we'll use the public create_test_transaction from transaction_test

// ========== Basic Functionality Tests ==========

#[actix_web::test]
async fn test_get_audit_trail_empty() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client.get("/audit_trail").with_auth(&token).send().await;

    assert_eq!(response.status(), 200);
    let events: Vec<AuditEvent> = response.json().await.unwrap();
    assert_eq!(events.len(), 0);
}

#[actix_web::test]
async fn test_get_audit_trail_unauthorized() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let response = client.get("/audit_trail").send().await;

    assert_eq!(response.status(), 401);
}

#[actix_web::test]
async fn test_audit_trail_profile_isolation() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id1, profile_id1, token1) = client.bootstrap("Merchant 1", "Profile 1").await;
    let (merchant_id2, profile_id2, token2) = client.bootstrap("Merchant 2", "Profile 2").await;

    // Create account and upload file for profile 1
    let account_id1 =
        create_test_account(&client, &token1, "Account 1", BalanceDirection::Debit).await;
    let config1 = create_manual_ingestion_config(
        &client,
        merchant_id1.clone(),
        &profile_id1,
        &account_id1,
        "Ingestion 1",
    )
    .await;
    upload_file(
        &client,
        &token1,
        &config1.ingestion_id,
        b"test,data\n1,2",
        "test1.csv",
    )
    .await;

    // Create account and upload file for profile 2
    let account_id2 =
        create_test_account(&client, &token2, "Account 2", BalanceDirection::Debit).await;
    let config2 = create_manual_ingestion_config(
        &client,
        merchant_id2.clone(),
        &profile_id2,
        &account_id2,
        "Ingestion 2",
    )
    .await;
    upload_file(
        &client,
        &token2,
        &config2.ingestion_id,
        b"test,data\n3,4",
        "test2.csv",
    )
    .await;

    // Get audit trail for profile 1
    let response1 = client.get("/audit_trail").with_auth(&token1).send().await;
    assert_eq!(response1.status(), 200);
    let events1: Vec<AuditEvent> = response1.json().await.unwrap();

    // Get audit trail for profile 2
    let response2 = client.get("/audit_trail").with_auth(&token2).send().await;
    assert_eq!(response2.status(), 200);
    let events2: Vec<AuditEvent> = response2.json().await.unwrap();

    // Each profile should only see their own events
    assert!(
        !events1.is_empty(),
        "Profile 1 should have at least 1 event"
    );
    assert!(
        !events2.is_empty(),
        "Profile 2 should have at least 1 event"
    );

    // Verify events belong to correct accounts
    for event in &events1 {
        if let AuditEvent::FileUploaded { account, .. } = event {
            assert_eq!(account.account_id, account_id1);
        }
    }

    for event in &events2 {
        if let AuditEvent::FileUploaded { account, .. } = event {
            assert_eq!(account.account_id, account_id2);
        }
    }
}

// ========== Sequence-Based Grouping Tests (using rstest) ==========

/// Expected event in audit trail response
#[derive(Debug, Clone, PartialEq)]
enum ExpectedEvent {
    FileUploaded,
    IngestionsFailed(usize),
    StagingEntriesCreated(usize),
    StagingEntryNeedsManualReview(usize),
    ExpectationsCreated(usize),
    TransactionsMatched(usize),
    TransactionsReconciled(usize),
    TransactionsMismatched(usize),
}

/// Helper to execute a sequence of events matching AuditEvent types
async fn execute_event_sequence(
    client: &crate::common::TestClient,
    token: &str,
    merchant_id: MerchantId,
    profile_id: &str,
    account_id: &str,
    account_id2: &str,
    sequence: &[&str],
) {
    let mut config = None;
    let mut transformation = None;
    let mut rule_id = None;

    for event_type in sequence {
        match *event_type {
            "FileUploaded" => {
                if config.is_none() {
                    config = Some(
                        create_manual_ingestion_config(
                            client,
                            merchant_id.clone(),
                            profile_id,
                            account_id,
                            "Test Ingestion",
                        )
                        .await,
                    );
                }
                let file_name = format!("file_{}.csv", uuid::Uuid::new_v4());
                upload_file(
                    client,
                    token,
                    &config.as_ref().unwrap().ingestion_id,
                    b"test,data\n1,2",
                    &file_name,
                )
                .await;
            }
            "StagingEntriesCreated" => {
                if transformation.is_none() {
                    if config.is_none() {
                        config = Some(
                            create_manual_ingestion_config(
                                client,
                                merchant_id.clone(),
                                profile_id,
                                account_id,
                                "Test Ingestion",
                            )
                            .await,
                        );
                    }
                    transformation = Some(
                        create_transformation_config(
                            profile_id,
                            account_id,
                            Some(&config.as_ref().unwrap().ingestion_id),
                            &merchant_id,
                            token,
                            client,
                            "metadata.transaction_id".parse().unwrap(),
                            StagingEntryProcessingMode::Transaction,
                            vec![],
                        )
                        .await,
                    );
                }
                create_staging_entry(
                    client,
                    token,
                    account_id,
                    &transformation.as_ref().unwrap().transformation_id,
                    100.0,
                )
                .await;
            }
            "ExpectationsCreated" => {
                if rule_id.is_none() {
                    rule_id = Some(
                        create_test_recon_rule(
                            client,
                            token,
                            "test_rule_seq",
                            account_id.to_string(),
                            account_id2.to_string(),
                            AgingConfigApiRequest::NoAging,
                        )
                        .await,
                    );
                }
                let _txn = super::transaction_test::create_test_transaction(
                    client,
                    token,
                    account_id,
                    account_id2,
                    rule_id.as_ref().unwrap(),
                    &merchant_id,
                    profile_id,
                    ApiTransactionStatus::Expected,
                    None,
                )
                .await;
            }
            "TransactionsMatched" => {
                if rule_id.is_none() {
                    rule_id = Some(
                        create_test_recon_rule(
                            client,
                            token,
                            "test_rule_seq",
                            account_id.to_string(),
                            account_id2.to_string(),
                            AgingConfigApiRequest::NoAging,
                        )
                        .await,
                    );
                }
                let _txn = super::transaction_test::create_test_transaction(
                    client,
                    token,
                    account_id,
                    account_id2,
                    rule_id.as_ref().unwrap(),
                    &merchant_id,
                    profile_id,
                    ApiTransactionStatus::Matched(TransactionMatchedData::Auto),
                    None,
                )
                .await;
            }
            "TransactionsMismatched" => {
                if rule_id.is_none() {
                    rule_id = Some(
                        create_test_recon_rule(
                            client,
                            token,
                            "test_rule_seq",
                            account_id.to_string(),
                            account_id2.to_string(),
                            AgingConfigApiRequest::NoAging,
                        )
                        .await,
                    );
                }
                let _txn = super::transaction_test::create_test_transaction(
                    client,
                    token,
                    account_id,
                    account_id2,
                    rule_id.as_ref().unwrap(),
                    &merchant_id,
                    profile_id,
                    ApiTransactionStatus::DataMismatch,
                    Some(super::transaction_test::DataMismatchType::Metadata),
                )
                .await;
            }
            _ => panic!("Invalid event type: {event_type}"),
        }
    }
}

// Test various event sequences and verify exact response structure
#[rstest]
// No grouping - 3 separate FileUploaded events
#[case(
    &["FileUploaded", "FileUploaded", "FileUploaded"],
    &[ExpectedEvent::FileUploaded, ExpectedEvent::FileUploaded, ExpectedEvent::FileUploaded]
)]
// Time window grouping - 3 staging entries → 1 group with count=3
#[case(
    &["StagingEntriesCreated", "StagingEntriesCreated", "StagingEntriesCreated"],
    &[ExpectedEvent::StagingEntriesCreated(3)]
)]
// Time window grouping - 3 transactions → 1 group with count=3
#[case(
    &["TransactionsMatched", "TransactionsMatched", "TransactionsMatched"],
    &[ExpectedEvent::TransactionsMatched(3)]
)]
// Time window grouping - 3 expectations (staging entries are archived, not in audit trail)
#[case(
    &["ExpectationsCreated", "ExpectationsCreated", "ExpectationsCreated"],
    &[ExpectedEvent::ExpectationsCreated(3)]
)]
// Time window grouping - 3 mismatches (creates 6 staging entries: 2 per transaction, grouped by account)
// Each transaction creates 2 staging entries (one for each account), so we get 2 groups of 3
#[case(
    &["TransactionsMismatched", "TransactionsMismatched", "TransactionsMismatched"],
    &[ExpectedEvent::StagingEntriesCreated(3), ExpectedEvent::StagingEntriesCreated(3), ExpectedEvent::TransactionsMismatched(3)]
)]
// Mixed - uploads separate, others group
#[case(
    &["FileUploaded", "FileUploaded", "StagingEntriesCreated", "StagingEntriesCreated", "TransactionsMatched", "TransactionsMatched"],
    &[ExpectedEvent::FileUploaded, ExpectedEvent::FileUploaded, ExpectedEvent::StagingEntriesCreated(2), ExpectedEvent::TransactionsMatched(2)]
)]
// Alternating uploads and staging - after sorting: all uploads first, then staging groups
#[case(
    &["FileUploaded", "StagingEntriesCreated", "FileUploaded", "StagingEntriesCreated", "FileUploaded", "StagingEntriesCreated"],
    &[ExpectedEvent::FileUploaded, ExpectedEvent::FileUploaded, ExpectedEvent::FileUploaded, ExpectedEvent::StagingEntriesCreated(3)]
)]
// Complex - after sorting: FileUploaded, then ExpectationsCreated, TransactionsMatched, StagingEntriesCreated groups, TransactionsMismatched
// 2 explicit staging for one account + 1 from TransactionsMismatched = 3 total for account1
// 1 from TransactionsMismatched for account2 = 1 total for account2
#[case(
    &["FileUploaded", "StagingEntriesCreated", "StagingEntriesCreated", "ExpectationsCreated", "TransactionsMatched", "TransactionsMismatched"],
    &[ExpectedEvent::FileUploaded, ExpectedEvent::ExpectationsCreated(1), ExpectedEvent::TransactionsMatched(1), ExpectedEvent::StagingEntriesCreated(3), ExpectedEvent::StagingEntriesCreated(1), ExpectedEvent::TransactionsMismatched(1)]
)]
// Multiple transaction types - after sorting: ExpectationsCreated, TransactionsMatched, StagingEntriesCreated groups, TransactionsMismatched
// 2 TransactionsMismatched create 4 staging entries (2 per transaction, grouped by account)
// account1: 2 staging entries → StagingEntriesCreated(2)
// account2: 2 staging entries → StagingEntriesCreated(2)
#[case(
    &["ExpectationsCreated", "ExpectationsCreated", "TransactionsMatched", "TransactionsMatched", "TransactionsMismatched", "TransactionsMismatched"],
    &[ExpectedEvent::ExpectationsCreated(2), ExpectedEvent::TransactionsMatched(2), ExpectedEvent::StagingEntriesCreated(2), ExpectedEvent::StagingEntriesCreated(2), ExpectedEvent::TransactionsMismatched(2)]
)]
#[actix_web::test]
async fn test_event_sequence_grouping(
    #[case] sequence: &[&str],
    #[case] expected_events: &[ExpectedEvent],
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    execute_event_sequence(
        &client,
        &token,
        merchant_id,
        &profile_id,
        &account_id,
        &account_id2,
        sequence,
    )
    .await;

    let response = client.get("/audit_trail").with_auth(&token).send().await;
    assert_eq!(response.status(), 200);
    let mut events: Vec<AuditEvent> = response.json().await.unwrap();

    // Sort events by timestamp to ensure chronological order
    events.sort_by_key(|e| match e {
        AuditEvent::FileUploaded { timestamp, .. } => *timestamp,
        AuditEvent::IngestionsFailed { last_failed_at, .. } => *last_failed_at,
        AuditEvent::StagingEntriesCreated { timestamp, .. } => *timestamp,
        AuditEvent::StagingEntryNeedsManualReview { timestamp, .. } => *timestamp,
        AuditEvent::ExpectationsCreated { timestamp, .. } => *timestamp,
        AuditEvent::TransactionsMatched { timestamp, .. } => *timestamp,
        AuditEvent::TransactionsReconciled { timestamp, .. } => *timestamp,
        AuditEvent::TransactionsMismatched { timestamp, .. } => *timestamp,
    });

    // Convert actual events to ExpectedEvent format
    let actual_events: Vec<ExpectedEvent> = events
        .iter()
        .map(|e| match e {
            AuditEvent::FileUploaded { .. } => ExpectedEvent::FileUploaded,
            AuditEvent::IngestionsFailed { count, .. } => ExpectedEvent::IngestionsFailed(*count),
            AuditEvent::StagingEntriesCreated { count, .. } => {
                ExpectedEvent::StagingEntriesCreated(*count)
            }
            AuditEvent::StagingEntryNeedsManualReview { count, .. } => {
                ExpectedEvent::StagingEntryNeedsManualReview(*count)
            }
            AuditEvent::ExpectationsCreated { count, .. } => {
                ExpectedEvent::ExpectationsCreated(*count)
            }
            AuditEvent::TransactionsMatched { count, .. } => {
                ExpectedEvent::TransactionsMatched(*count)
            }
            AuditEvent::TransactionsReconciled { count, .. } => {
                ExpectedEvent::TransactionsReconciled(*count)
            }
            AuditEvent::TransactionsMismatched { count, .. } => {
                ExpectedEvent::TransactionsMismatched(*count)
            }
        })
        .collect();

    assert_eq!(
        actual_events, expected_events,
        "Sequence {:?}: Expected events {:?}, got {:?}",
        sequence, expected_events, actual_events
    );
}

// Test chronological order
#[rstest]
#[case(&["FileUploaded", "StagingEntriesCreated", "TransactionsMatched", "FileUploaded", "StagingEntriesCreated", "TransactionsMatched"])]
#[case(&["StagingEntriesCreated", "StagingEntriesCreated", "StagingEntriesCreated", "FileUploaded", "FileUploaded", "FileUploaded", "TransactionsMatched", "TransactionsMatched", "TransactionsMatched"])]
#[case(&["TransactionsMatched", "FileUploaded", "TransactionsMismatched", "StagingEntriesCreated", "FileUploaded", "ExpectationsCreated"])]
#[actix_web::test]
async fn test_event_sequence_chronological_order(#[case] sequence: &[&str]) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    execute_event_sequence(
        &client,
        &token,
        merchant_id,
        &profile_id,
        &account_id,
        &account_id2,
        sequence,
    )
    .await;

    let response = client.get("/audit_trail").with_auth(&token).send().await;
    assert_eq!(response.status(), 200);
    let events: Vec<AuditEvent> = response.json().await.unwrap();

    let timestamps: Vec<time::PrimitiveDateTime> = events
        .iter()
        .map(|e| match e {
            AuditEvent::FileUploaded { timestamp, .. } => *timestamp,
            AuditEvent::IngestionsFailed { last_failed_at, .. } => *last_failed_at,
            AuditEvent::StagingEntriesCreated { timestamp, .. } => *timestamp,
            AuditEvent::StagingEntryNeedsManualReview { timestamp, .. } => *timestamp,
            AuditEvent::ExpectationsCreated { timestamp, .. } => *timestamp,
            AuditEvent::TransactionsMatched { timestamp, .. } => *timestamp,
            AuditEvent::TransactionsReconciled { timestamp, .. } => *timestamp,
            AuditEvent::TransactionsMismatched { timestamp, .. } => *timestamp,
        })
        .collect();

    for i in 1..timestamps.len() {
        assert!(
            timestamps[i] >= timestamps[i - 1],
            "Sequence {sequence:?}: Events should be in chronological order"
        );
    }
}
