use std::collections::HashMap;

use crate::{
    api::{
        account_test::create_test_account,
        exception_management_test::setup_test_environment,
        staging_entry_test::staging_entry_request,
        transaction_test::{
            create_test_transaction, create_transaction_for_filtering, DataMismatchType,
        },
        transformation_test::create_transformation_config,
    },
    common::TestClient,
};
use api_models::{
    account::{AccountCreateRequest, GetAccountResponse},
    entry::{EntryData, GetEntryResponse},
    metadata_schema::{FieldType, MetadataExposeConfig},
    search_entry_key::{SearchEntryKeyListRequest, SearchEntryKeyResponse},
    staging_entry::{
        ApiStagingEntryCoarseStatus, GetStagingEntryResponse, NeedsManualReviewData,
        StagingEntryData,
    },
    transaction::{
        ApiTransactionStatus, DirectTransactionEntry, EntryOp, EntryOverrides,
        ExpectedTransactionEntry, GetTransactionResponse, ManualReconciliationV2Request, NewEntry,
        SearchExpectation, StagingEntryOverrides, TransactionEntry, TransactionRequest,
    },
};
use common_enums::{BalanceDirection, Currency, EntryStatus, StagingEntryProcessingMode};
use common_types::{
    recon_rule::entry_field::EntryField,
    transaction::status::{TransactionAmountMismatchData, TransactionMatchedData},
    units::FloatMajorUnit,
};
use common_utils::{date_time, id_type::MerchantId};
use masking::{ExposeInterface, Secret};
use pretty_assertions::{assert_eq, assert_ne};
use serde_json::json;

// ================================================================================================
// HELPER FUNCTIONS
// ================================================================================================

async fn reconcile_v2(
    client: &TestClient,
    token: &str,
    transaction_primary_id: &str,
    entry_ops: Vec<EntryOp>,
) -> reqwest::Response {
    client
        .post(&format!(
            "/exception_management/transactions/{transaction_primary_id}/manual_reconciliation/v2"
        ))
        .with_auth(token)
        .json(&ManualReconciliationV2Request {
            entry_ops,
            reason: Some("manual reconciliation v2 test".to_string()),
        })
        .send()
        .await
}

async fn reconcile_v2_ok(
    client: &TestClient,
    token: &str,
    transaction_primary_id: &str,
    entry_ops: Vec<EntryOp>,
) -> GetTransactionResponse {
    let response = reconcile_v2(client, token, transaction_primary_id, entry_ops).await;

    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// The named version's own entries, keyed by entry id.
///
/// Reads `transaction.entry_id` rather than `GET /transactions/{id}/entries`, which also returns
/// the archived entries of every superseded version.
async fn entries_of(
    client: &TestClient,
    token: &str,
    transaction: &GetTransactionResponse,
) -> HashMap<String, GetEntryResponse> {
    let mut entries = HashMap::with_capacity(transaction.entry_id.len());

    for entry_id in &transaction.entry_id {
        let response = client
            .get(&format!("/entries/{entry_id}"))
            .with_auth(token)
            .send()
            .await;

        assert_eq!(response.status(), 200);
        entries.insert(entry_id.clone(), response.json().await.unwrap());
    }

    entries
}

fn entry_on_account<'a>(
    entries: &'a HashMap<String, GetEntryResponse>,
    account_id: &str,
) -> &'a GetEntryResponse {
    let mut matching = entries
        .values()
        .filter(|entry| entry.account_id == account_id);

    let entry = matching
        .next()
        .unwrap_or_else(|| panic!("no entry on account {account_id}"));
    assert!(
        matching.next().is_none(),
        "expected exactly one entry on account {account_id}"
    );

    entry
}

/// Entry order within a version is undefined, so entries on a shared account are picked by amount.
fn entry_with_amount<'a>(
    entries: &'a HashMap<String, GetEntryResponse>,
    account_id: &str,
    amount: f64,
) -> &'a GetEntryResponse {
    entries
        .values()
        .find(|entry| {
            entry.account_id == account_id && entry.amount.value == FloatMajorUnit::new(amount)
        })
        .unwrap_or_else(|| panic!("no entry on account {account_id} with amount {amount}"))
}

fn plaintext_metadata(entry: &GetEntryResponse) -> serde_json::Value {
    entry.metadata.get_inner().clone().expose()
}

async fn search_keys_of(
    client: &TestClient,
    token: &str,
    entry_id: &str,
) -> Vec<SearchEntryKeyResponse> {
    let response = client
        .post("/search_entry_keys")
        .with_auth(token)
        .json(&SearchEntryKeyListRequest {
            entry_ids: vec![entry_id.to_string()],
        })
        .send()
        .await;

    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// Sorted `(metadata_key, metadata_value)` pairs, so one entry's keys compare across versions.
async fn search_key_pairs(
    client: &TestClient,
    token: &str,
    entry_id: &str,
) -> Vec<(String, Option<String>)> {
    let mut pairs: Vec<(String, Option<String>)> = search_keys_of(client, token, entry_id)
        .await
        .into_iter()
        .map(|key| (key.metadata_key, key.metadata_value))
        .collect();
    pairs.sort();

    pairs
}

/// Assertions are status-code only, so pair every rejection with proof that the version it named
/// was left alone — i.e. the request was refused before the write rather than half-applied.
async fn assert_unchanged(
    client: &TestClient,
    token: &str,
    transaction: &GetTransactionResponse,
    context: &str,
) {
    let response = client
        .get(&format!("/transactions/{}", transaction.id))
        .with_auth(token)
        .send()
        .await;

    assert_eq!(response.status(), 200, "{context}");
    let current: GetTransactionResponse = response.json().await.unwrap();

    assert_eq!(current.version, transaction.version, "{context}");
    assert_eq!(current.status, transaction.status, "{context}");
    assert_eq!(current.entry_id, transaction.entry_id, "{context}");
}

/// The base fixture: source account Credit 100 against target account Debit 90, status
/// `UnderAmountMismatch`. Both entries come from staging entries, so both carry a
/// `staging_entry_id` and a `transformation_id` — the lineage v1 drops and v2 must preserve.
async fn mismatched_transaction(
    client: &TestClient,
    token: &str,
    merchant_id: &MerchantId,
    profile_id: &str,
    source_account_id: &str,
    target_account_id: &str,
    rule_id: &str,
) -> GetTransactionResponse {
    let transaction = create_test_transaction(
        client,
        token,
        source_account_id,
        target_account_id,
        rule_id,
        merchant_id,
        profile_id,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch),
        Some(DataMismatchType::Amount { variance: 10.0 }),
    )
    .await;

    assert_eq!(
        transaction.status,
        ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
    );

    transaction
}

/// Metadata fields covering all three exposure states: searchable, exposed-only, not exposed.
fn exposing_metadata_fields() -> Vec<(EntryField, bool, FieldType, Option<MetadataExposeConfig>)> {
    let string_field = || FieldType::String {
        validation_rules: vec![],
        transformation_rules: vec![],
    };

    vec![
        (
            "metadata.reference_id".parse().unwrap(),
            false,
            string_field(),
            Some(MetadataExposeConfig {
                is_searchable: true,
            }),
        ),
        (
            "metadata.payment_method".parse().unwrap(),
            false,
            string_field(),
            Some(MetadataExposeConfig {
                is_searchable: false,
            }),
        ),
        (
            "metadata.merchant_name".parse().unwrap(),
            false,
            string_field(),
            None,
        ),
    ]
}

async fn create_staging_entry(
    client: &TestClient,
    token: &str,
    account_id: &str,
    transformation_id: &str,
    entry_type: BalanceDirection,
    amount: f64,
    currency: Currency,
    metadata: serde_json::Value,
) -> GetStagingEntryResponse {
    let mut request = staging_entry_request(
        account_id,
        entry_type,
        StagingEntryProcessingMode::Confirmation,
        amount,
        currency,
        transformation_id.to_string(),
    );
    request.metadata = Secret::new(metadata);

    let response = client
        .post("/staging_entries")
        .with_auth(token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    response.json().await.unwrap()
}

async fn latest_staging_entry(
    client: &TestClient,
    token: &str,
    staging_entry_id: &str,
) -> GetStagingEntryResponse {
    let response = client.get("/staging_entries").with_auth(token).send().await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetStagingEntryResponse> = response.json().await.unwrap();

    entries
        .into_iter()
        .filter(|entry| entry.staging_entry_id == staging_entry_id)
        .max_by_key(|entry| entry.version)
        .unwrap_or_else(|| panic!("staging entry {staging_entry_id} not found"))
}

/// A staging entry processed into `NeedsManualReview(NoExpectationEntryFound)`, which is one of the
/// statuses `is_linkable()` accepts and therefore what the staging-entry ops need.
async fn link_ready_staging_entry(
    client: &TestClient,
    token: &str,
    account_id: &str,
    transformation_id: &str,
    entry_type: BalanceDirection,
    amount: f64,
    currency: Currency,
    reference_id: &str,
) -> GetStagingEntryResponse {
    let created = create_staging_entry(
        client,
        token,
        account_id,
        transformation_id,
        entry_type,
        amount,
        currency,
        json!({ "reference_id": reference_id }),
    )
    .await;

    let response = client
        .post(&format!("/recon_engine/{}/process", created.id))
        .with_auth(token)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let latest = latest_staging_entry(client, token, &created.staging_entry_id).await;
    assert!(
        matches!(
            latest.data,
            StagingEntryData::NeedsManualReview(NeedsManualReviewData::NoExpectationEntryFound)
        ),
        "staging entry {} must be linkable, got {:?}",
        latest.id,
        latest.data
    );

    latest
}

/// `create_test_account` always mints USD accounts; the currency checks need one that is not.
async fn create_account_with_currency(
    client: &TestClient,
    token: &str,
    name: &str,
    currency: Currency,
) -> String {
    let response = client
        .post("/accounts")
        .with_auth(token)
        .json(&AccountCreateRequest {
            account_name: name.to_string(),
            account_type: BalanceDirection::Debit,
            currency,
            initial_balance: FloatMajorUnit::new(1000.0),
        })
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let account: GetAccountResponse = response.json().await.unwrap();

    account.account_id
}

// ================================================================================================
// ENTRY OPERATIONS
// ================================================================================================

/// The headline difference from v1: entries the request never mentions are reproduced field for
/// field, lineage included, instead of being rebuilt from whatever the client happened to send.
#[actix_web::test]
async fn v2_carries_untouched_entries_forward_with_lineage_intact() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let before = entries_of(&client, &token, &transaction).await;
    let source_before = entry_on_account(&before, &source_account_id);
    let target_before = entry_on_account(&before, &target_account_id);

    assert!(
        source_before.staging_entry_id.is_some() && source_before.transformation_id.is_some(),
        "fixture precondition: the untouched entry must start with lineage to preserve"
    );

    let reconciled = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::UpdateEntry {
            entry_id: target_before.entry_id.clone(),
            overrides: EntryOverrides {
                amount: Some(FloatMajorUnit::new(100.0)),
                ..Default::default()
            },
        }],
    )
    .await;

    assert_eq!(reconciled.version, transaction.version + 1);
    assert_eq!(reconciled.entry_id.len(), 2);
    assert_eq!(
        reconciled.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
    );
    assert_eq!(reconciled.credit_amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(reconciled.debit_amount.value, FloatMajorUnit::new(100.0));

    for entry_id in &reconciled.entry_id {
        assert!(
            !transaction.entry_id.contains(entry_id),
            "a new version writes new entry rows"
        );
    }

    let after = entries_of(&client, &token, &reconciled).await;
    let source_after = entry_on_account(&after, &source_account_id);
    let target_after = entry_on_account(&after, &target_account_id);

    // Balanced with no Expected entry left, so every entry in the new version is rewritten to
    // Matched -- the carried-forward one included.
    for entry in after.values() {
        assert_eq!(
            entry.status,
            EntryStatus::Matched,
            "entry {}",
            entry.entry_id
        );
        assert!(
            matches!(entry.data, EntryData::Matched),
            "entry {} data: {:?}",
            entry.entry_id,
            entry.data
        );
    }

    assert_eq!(source_after.account_id, source_before.account_id);
    assert_eq!(source_after.entry_type, source_before.entry_type);
    assert_eq!(source_after.amount.value, source_before.amount.value);
    assert_eq!(source_after.order_id, source_before.order_id);
    assert_eq!(source_after.effective_at, source_before.effective_at);
    assert_eq!(
        plaintext_metadata(source_after),
        plaintext_metadata(source_before)
    );
    assert_eq!(
        source_after.exposed_metadata,
        source_before.exposed_metadata
    );
    assert_eq!(
        source_after.staging_entry_id, source_before.staging_entry_id,
        "dropping primary_staging_entry_id is what strands a search_duplicate row as Archived"
    );
    assert_eq!(
        source_after.transformation_id,
        source_before.transformation_id
    );

    // The edited entry takes the override and keeps everything the override did not name.
    assert_eq!(target_after.amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(target_after.order_id, target_before.order_id);
    assert_eq!(
        target_after.staging_entry_id,
        target_before.staging_entry_id
    );
    assert_eq!(
        target_after.transformation_id,
        target_before.transformation_id
    );
}

/// `exposed_metadata` and the `search_entry_key` rows derived from it are recomputed exactly when
/// the metadata or its schema changes, and copied otherwise. Three chained calls, one per row of
/// that rule: each leaves the transaction unbalanced, so it stays reconcilable for the next.
#[actix_web::test]
async fn v2_metadata_and_search_keys_fresh_vs_carried() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    // The configs the fixture built declare no metadata fields, so every entry starts with an
    // empty projection and no search keys. This one actually exposes something.
    let exposing_config = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        exposing_metadata_fields(),
    )
    .await;

    let before = entries_of(&client, &token, &transaction).await;
    let source_before = entry_on_account(&before, &source_account_id);
    let target_before = entry_on_account(&before, &target_account_id);
    let original_metadata = plaintext_metadata(target_before);

    assert_eq!(target_before.exposed_metadata, json!({}));
    for key in ["reference_id", "payment_method", "merchant_name"] {
        assert!(
            original_metadata.get(key).is_some(),
            "fixture precondition: target metadata should carry {key}, got {original_metadata}"
        );
    }

    // ===== The op supplies transformation_id only: ciphertext copied, projection redone =====

    let reprojected = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::UpdateEntry {
            entry_id: target_before.entry_id.clone(),
            overrides: EntryOverrides {
                transformation_id: Some(exposing_config.transformation_id.clone()),
                ..Default::default()
            },
        }],
    )
    .await;

    assert_eq!(
        reprojected.status,
        ApiTransactionStatus::PartiallyReconciled
    );

    let entries = entries_of(&client, &token, &reprojected).await;
    let target = entry_on_account(&entries, &target_account_id);
    let source = entry_on_account(&entries, &source_account_id);

    assert_eq!(
        plaintext_metadata(target),
        original_metadata,
        "the metadata is decrypted only to re-project; the ciphertext is stored back unchanged"
    );
    assert_eq!(
        target.exposed_metadata,
        json!({
            "reference_id": original_metadata["reference_id"].clone(),
            "payment_method": original_metadata["payment_method"].clone(),
        }),
        "merchant_name is declared but not exposed, so it is projected out"
    );
    assert_eq!(
        target.transformation_id.as_deref(),
        Some(exposing_config.transformation_id.as_str())
    );

    let keys = search_keys_of(&client, &token, &target.entry_id).await;
    assert_eq!(keys.len(), 1, "only the searchable key produces a row");
    assert_eq!(keys[0].metadata_key, "reference_id");
    assert_eq!(
        keys[0].metadata_value.as_deref(),
        original_metadata["reference_id"].as_str()
    );
    assert_eq!(
        keys[0].primary_transaction_id, reprojected.id,
        "search keys are minted against the version PK, not the logical transaction id"
    );
    assert_ne!(reprojected.id, reprojected.transaction_id);

    // The entry no op mentioned keeps its empty projection and its absent keys.
    assert_eq!(source.exposed_metadata, source_before.exposed_metadata);
    assert!(search_keys_of(&client, &token, &source.entry_id)
        .await
        .is_empty());

    // ===== The op supplies metadata: re-encrypted, re-projected, keys re-derived =====

    let fresh_metadata = json!({
        "reference_id": "REF-AFTER-EDIT",
        "payment_method": "upi",
        "merchant_name": "Rewritten Store",
    });

    let rewritten = reconcile_v2_ok(
        &client,
        &token,
        &reprojected.id,
        vec![EntryOp::UpdateEntry {
            entry_id: target.entry_id.clone(),
            overrides: EntryOverrides {
                metadata: Some(Secret::new(fresh_metadata.clone())),
                ..Default::default()
            },
        }],
    )
    .await;

    let entries = entries_of(&client, &token, &rewritten).await;
    let target = entry_on_account(&entries, &target_account_id);

    assert_eq!(plaintext_metadata(target), fresh_metadata);
    assert_eq!(
        target.exposed_metadata,
        json!({ "reference_id": "REF-AFTER-EDIT", "payment_method": "upi" })
    );
    assert_eq!(
        target.transformation_id.as_deref(),
        Some(exposing_config.transformation_id.as_str()),
        "an override that names no transformation_id carries the entry's current one forward"
    );

    let keys = search_keys_of(&client, &token, &target.entry_id).await;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].metadata_value.as_deref(), Some("REF-AFTER-EDIT"));
    assert_eq!(keys[0].primary_transaction_id, rewritten.id);

    let rewritten_projection = target.exposed_metadata.clone();
    let rewritten_keys = search_key_pairs(&client, &token, &target.entry_id).await;

    // ===== The op supplies neither: both are copied =====

    let carried = reconcile_v2_ok(
        &client,
        &token,
        &rewritten.id,
        vec![EntryOp::UpdateEntry {
            entry_id: target.entry_id.clone(),
            overrides: EntryOverrides {
                order_id: Some("manually-reassigned-order".to_string()),
                ..Default::default()
            },
        }],
    )
    .await;

    let entries = entries_of(&client, &token, &carried).await;
    let target = entry_on_account(&entries, &target_account_id);

    assert_eq!(target.order_id, "manually-reassigned-order");
    assert_eq!(plaintext_metadata(target), fresh_metadata);
    assert_eq!(
        target.exposed_metadata, rewritten_projection,
        "neither metadata nor transformation_id changed, so the projection is copied"
    );
    assert_eq!(
        search_key_pairs(&client, &token, &target.entry_id).await,
        rewritten_keys,
        "and so are the keys derived from it"
    );
    assert_eq!(
        search_keys_of(&client, &token, &target.entry_id).await[0].primary_transaction_id,
        carried.id,
        "though they are re-emitted against the new version"
    );
}

/// `mark_received` moves an `Expected` entry to `Pending`, and refuses anything else.
#[actix_web::test]
async fn v2_mark_received_transitions_expected_entry() {
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

    let response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&TransactionRequest {
            entries: vec![
                TransactionEntry::Direct(DirectTransactionEntry {
                    account_id: source_account_id.clone(),
                    entry_type: BalanceDirection::Credit,
                    amount: FloatMajorUnit::new(100.0),
                    currency: Currency::USD,
                    effective_at: date_time::now(),
                    metadata: Secret::new(json!({"description": "settled leg"})),
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
                    metadata: Secret::new(json!({"description": "awaited leg"})),
                    search_expectation: SearchExpectation {
                        rule_id: rule_id.clone(),
                        source_value: "order_awaiting_confirmation".to_string(),
                        target_key: "metadata.reference_id".parse().unwrap(),
                    },
                    order_id: "order_test".to_string(),
                }),
            ],
            rule_id: rule_id.clone(),
        })
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let transaction: GetTransactionResponse = response.json().await.unwrap();
    assert_eq!(transaction.status, ApiTransactionStatus::Expected);

    let entries = entries_of(&client, &token, &transaction).await;
    let expected_entry = entry_on_account(&entries, &target_account_id);
    let settled_entry = entry_on_account(&entries, &source_account_id);
    assert_eq!(expected_entry.status, EntryStatus::Expected);
    assert_eq!(settled_entry.status, EntryStatus::Pending);

    // The target must currently be Expected, so the already-settled leg is refused.
    let rejected = reconcile_v2(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::MarkReceived {
            entry_id: settled_entry.entry_id.clone(),
            overrides: EntryOverrides::default(),
        }],
    )
    .await;

    assert_eq!(rejected.status(), 400);
    assert_unchanged(
        &client,
        &token,
        &transaction,
        "mark_received on a Pending entry",
    )
    .await;

    // The awaited leg arrives. No Expected entry survives and the version is balanced, so it
    // settles as Matched(Manual).
    let received = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::MarkReceived {
            entry_id: expected_entry.entry_id.clone(),
            overrides: EntryOverrides::default(),
        }],
    )
    .await;

    assert_eq!(
        received.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
    );

    let entries = entries_of(&client, &token, &received).await;
    let settled = entry_on_account(&entries, &target_account_id);
    assert_eq!(settled.status, EntryStatus::Matched);
    assert!(matches!(settled.data, EntryData::Matched));
}

/// `create_entry` with `kind: direct` adds a Pending entry projected through the config it names.
#[actix_web::test]
async fn v2_create_entry_direct_balances_transaction() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let exposing_config = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        exposing_metadata_fields(),
    )
    .await;

    let reconciled = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::CreateEntry {
            entry: NewEntry::Direct {
                account_id: target_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(10.0),
                effective_at: date_time::now(),
                metadata: Secret::new(json!({
                    "reference_id": "CREATED-REF",
                    "payment_method": "netbanking",
                    "merchant_name": "Hidden Store",
                    "not_in_schema": "dropped",
                })),
                order_id: "created-order-id".to_string(),
                transformation_id: Some(exposing_config.transformation_id.clone()),
            },
        }],
    )
    .await;

    assert_eq!(reconciled.entry_id.len(), 3);
    assert_eq!(
        reconciled.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
    );
    assert_eq!(reconciled.credit_amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(reconciled.debit_amount.value, FloatMajorUnit::new(100.0));

    let entries = entries_of(&client, &token, &reconciled).await;
    let created = entry_with_amount(&entries, &target_account_id, 10.0);

    assert_eq!(created.entry_type, BalanceDirection::Debit);
    assert_eq!(created.order_id, "created-order-id");
    assert_eq!(
        created.staging_entry_id, None,
        "a directly created entry has no staging lineage"
    );
    assert_eq!(
        created.transformation_id.as_deref(),
        Some(exposing_config.transformation_id.as_str())
    );
    assert_eq!(
        created.exposed_metadata,
        json!({ "reference_id": "CREATED-REF", "payment_method": "netbanking" })
    );

    let keys = search_keys_of(&client, &token, &created.entry_id).await;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].metadata_key, "reference_id");
    assert_eq!(keys[0].metadata_value.as_deref(), Some("CREATED-REF"));
    assert_eq!(keys[0].primary_transaction_id, reconciled.id);
}

/// `create_entry` with `kind: expected` keeps the transaction open even when the amounts balance,
/// and projects through `NO_EXPOSED_KEYS` because an Expected entry carries no config.
#[actix_web::test]
async fn v2_create_entry_expected_forces_partial_reconciliation() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let source_before = {
        let entries = entries_of(&client, &token, &transaction).await;
        entry_on_account(&entries, &source_account_id).status
    };

    let reconciled = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::CreateEntry {
            entry: NewEntry::Expected {
                account_id: target_account_id.clone(),
                entry_type: BalanceDirection::Debit,
                amount: FloatMajorUnit::new(10.0),
                effective_at: date_time::now(),
                metadata: Secret::new(json!({ "reference_id": "AWAITED-REF" })),
                order_id: "awaited-order-id".to_string(),
            },
        }],
    )
    .await;

    assert_eq!(reconciled.credit_amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(reconciled.debit_amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(
        reconciled.status,
        ApiTransactionStatus::PartiallyReconciled,
        "a surviving Expected entry outranks a balanced amount"
    );

    let entries = entries_of(&client, &token, &reconciled).await;
    let created = entry_with_amount(&entries, &target_account_id, 10.0);

    assert_eq!(created.status, EntryStatus::Expected);
    assert!(matches!(created.data, EntryData::Expected));
    assert_eq!(created.transformation_id, None);
    assert_eq!(
        created.exposed_metadata,
        json!({}),
        "no config means the NO_EXPOSED_KEYS projection"
    );
    assert!(search_keys_of(&client, &token, &created.entry_id)
        .await
        .is_empty());

    // Nothing is rewritten to Matched, so the carried entry keeps the status it had.
    assert_eq!(
        entry_on_account(&entries, &source_account_id).status,
        source_before
    );
}

/// `create_with_staging_entry` pulls an unreconciled staging entry in as a new entry and marks it
/// `Processed`. Amount, account and config come from the staging row and cannot be overridden.
#[actix_web::test]
async fn v2_create_with_staging_entry_links_and_processes_it() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let linkable_config = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        exposing_metadata_fields(),
    )
    .await;

    let staging = link_ready_staging_entry(
        &client,
        &token,
        &target_account_id,
        &linkable_config.transformation_id,
        BalanceDirection::Debit,
        10.0,
        Currency::USD,
        "LINKED-REF",
    )
    .await;

    let override_effective_at = date_time::now() - time::Duration::days(3);

    let reconciled = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::CreateWithStagingEntry {
            staging_entry_id: staging.id.clone(),
            overrides: StagingEntryOverrides {
                effective_at: Some(override_effective_at),
                order_id: Some("linked-order-id".to_string()),
                metadata: None,
            },
        }],
    )
    .await;

    assert_eq!(reconciled.entry_id.len(), 3);
    assert_eq!(
        reconciled.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
    );

    let entries = entries_of(&client, &token, &reconciled).await;
    let linked = entry_with_amount(&entries, &target_account_id, 10.0);

    // Taken from the staging row.
    assert_eq!(linked.entry_type, staging.entry_type);
    assert_eq!(linked.amount.value, staging.amount.value);
    assert_eq!(
        linked.transformation_id.as_deref(),
        Some(staging.transformation_id.as_str())
    );
    assert_eq!(
        linked.exposed_metadata, staging.exposed_metadata,
        "the staging entry's own projection is reused rather than recomputed"
    );

    // Taken from the overrides.
    assert_eq!(linked.order_id, "linked-order-id");
    assert_eq!(linked.effective_at.date(), override_effective_at.date());
    assert_ne!(linked.effective_at, staging.effective_at);

    assert!(
        linked
            .staging_entry_id
            .as_ref()
            .is_some_and(|id| id != &staging.id),
        "lineage points at the newly inserted Processed version, not the one that was linked"
    );

    let keys = search_keys_of(&client, &token, &linked.entry_id).await;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].metadata_value.as_deref(), Some("LINKED-REF"));
    assert_eq!(keys[0].primary_transaction_id, reconciled.id);

    let processed = latest_staging_entry(&client, &token, &staging.staging_entry_id).await;
    assert_eq!(processed.status, ApiStagingEntryCoarseStatus::Processed);
}

/// `replace_with_staging_entry` drops the named entry and adds the staging-sourced one in the same
/// operation, so the entry count does not move.
#[actix_web::test]
async fn v2_replace_with_staging_entry_swaps_the_entry() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let linkable_config = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        exposing_metadata_fields(),
    )
    .await;

    let staging = link_ready_staging_entry(
        &client,
        &token,
        &target_account_id,
        &linkable_config.transformation_id,
        BalanceDirection::Debit,
        100.0,
        Currency::USD,
        "REPLACEMENT-REF",
    )
    .await;

    let replaced_entry_id = {
        let entries = entries_of(&client, &token, &transaction).await;
        entry_on_account(&entries, &target_account_id)
            .entry_id
            .clone()
    };

    let reconciled = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::ReplaceWithStagingEntry {
            entry_id: replaced_entry_id.clone(),
            staging_entry_id: staging.id.clone(),
            overrides: StagingEntryOverrides::default(),
        }],
    )
    .await;

    assert_eq!(
        reconciled.entry_id.len(),
        2,
        "one entry out, one entry in: the net count delta is zero"
    );
    assert!(!reconciled.entry_id.contains(&replaced_entry_id));
    assert_eq!(
        reconciled.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
    );

    let entries = entries_of(&client, &token, &reconciled).await;
    let replacement = entry_on_account(&entries, &target_account_id);

    assert_eq!(replacement.amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(replacement.order_id, staging.order_id);
    assert_eq!(replacement.exposed_metadata, staging.exposed_metadata);
    assert!(replacement.staging_entry_id.is_some());

    let processed = latest_staging_entry(&client, &token, &staging.staging_entry_id).await;
    assert_eq!(processed.status, ApiStagingEntryCoarseStatus::Processed);
}

// ================================================================================================
// REJECTIONS
// ================================================================================================

/// `EntryOpsVecExt::validate_payload` and `validate_with_old_entries` run before anything is written.
#[actix_web::test]
async fn v2_rejects_malformed_op_payloads() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let target_entry_id = {
        let entries = entries_of(&client, &token, &transaction).await;
        entry_on_account(&entries, &target_account_id)
            .entry_id
            .clone()
    };

    // Entry ids are scoped to the version being edited, so one from elsewhere is simply not found.
    let other_transaction = create_transaction_for_filtering(
        &client,
        &token,
        &target_account_id,
        &source_account_id,
        &rule_id,
    )
    .await;
    let foreign_entry_id = other_transaction.entry_id[0].clone();

    let update = |entry_id: &str, overrides: EntryOverrides| EntryOp::UpdateEntry {
        entry_id: entry_id.to_string(),
        overrides,
    };
    let create_direct = |amount: f64| EntryOp::CreateEntry {
        entry: NewEntry::Direct {
            account_id: target_account_id.clone(),
            entry_type: BalanceDirection::Debit,
            amount: FloatMajorUnit::new(amount),
            effective_at: date_time::now(),
            metadata: Secret::new(json!({})),
            order_id: "order_test".to_string(),
            transformation_id: None,
        },
    };

    let cases: Vec<(&str, Vec<EntryOp>, u16)> = vec![
        ("no operations at all", vec![], 400),
        (
            "the same entry targeted twice",
            vec![
                update(&target_entry_id, EntryOverrides::default()),
                EntryOp::RemoveEntry {
                    entry_id: target_entry_id.clone(),
                },
            ],
            400,
        ),
        (
            // replace_with_staging_entry contributes both of its ids to the duplicate check.
            "one staging entry claimed by two ops",
            vec![
                EntryOp::ReplaceWithStagingEntry {
                    entry_id: target_entry_id.clone(),
                    staging_entry_id: "stag_ent_contested".to_string(),
                    overrides: StagingEntryOverrides::default(),
                },
                EntryOp::CreateWithStagingEntry {
                    staging_entry_id: "stag_ent_contested".to_string(),
                    overrides: StagingEntryOverrides::default(),
                },
            ],
            400,
        ),
        (
            "a created entry with a zero amount",
            vec![create_direct(0.0)],
            400,
        ),
        (
            "a created entry with a negative amount",
            vec![create_direct(-5.0)],
            400,
        ),
        (
            "an override with a negative amount",
            vec![update(
                &target_entry_id,
                EntryOverrides {
                    amount: Some(FloatMajorUnit::new(-5.0)),
                    ..Default::default()
                },
            )],
            400,
        ),
        (
            "an entry id that does not exist",
            vec![update("entry_does_not_exist", EntryOverrides::default())],
            404,
        ),
        (
            "an entry id from a different transaction",
            vec![update(&foreign_entry_id, EntryOverrides::default())],
            404,
        ),
    ];

    for (description, entry_ops, expected_status) in cases {
        let response = reconcile_v2(&client, &token, &transaction.id, entry_ops).await;

        assert_eq!(response.status(), expected_status, "{description}");
        assert_unchanged(&client, &token, &transaction, description).await;
    }
}

/// Profile scoping, the accepted status list, and the rule-account coverage floor that stops
/// `remove_entry` from emptying a transaction.
#[actix_web::test]
async fn v2_enforces_transaction_and_rule_invariants() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let target_entry_id = {
        let entries = entries_of(&client, &token, &transaction).await;
        entry_on_account(&entries, &target_account_id)
            .entry_id
            .clone()
    };
    let no_op = vec![EntryOp::UpdateEntry {
        entry_id: target_entry_id.clone(),
        overrides: EntryOverrides::default(),
    }];

    let missing = reconcile_v2(&client, &token, "txn_not_found", no_op).await;
    assert_eq!(missing.status(), 404);

    // Another profile cannot reach this transaction at all -- not even to learn it exists.
    let no_op = vec![EntryOp::UpdateEntry {
        entry_id: target_entry_id.clone(),
        overrides: EntryOverrides::default(),
    }];
    let (_, _, other_token) = client.bootstrap("Other Merchant", "Other Profile").await;
    let cross_profile = reconcile_v2(&client, &other_token, &transaction.id, no_op).await;
    assert_eq!(cross_profile.status(), 404);
    assert_unchanged(&client, &token, &transaction, "cross-profile access").await;

    // A balanced direct transaction is created Matched(Auto), which is outside the nine statuses
    // MANUALLY_RECONCILABLE_STATUSES allows.
    let already_matched = create_transaction_for_filtering(
        &client,
        &token,
        &target_account_id,
        &source_account_id,
        &rule_id,
    )
    .await;
    assert_eq!(
        already_matched.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Auto)
    );

    let wrong_status = reconcile_v2(
        &client,
        &token,
        &already_matched.id,
        vec![EntryOp::UpdateEntry {
            entry_id: already_matched.entry_id[0].clone(),
            overrides: EntryOverrides::default(),
        }],
    )
    .await;
    assert_eq!(wrong_status.status(), 400);
    assert_unchanged(
        &client,
        &token,
        &already_matched,
        "a transaction that is already Matched",
    )
    .await;

    // The rule covers both accounts, so the target account cannot be left without an entry.
    let uncovered = reconcile_v2(
        &client,
        &token,
        &transaction.id,
        vec![EntryOp::RemoveEntry {
            entry_id: target_entry_id.clone(),
        }],
    )
    .await;
    assert_eq!(uncovered.status(), 400);
    assert_unchanged(
        &client,
        &token,
        &transaction,
        "removing the last entry on a rule account",
    )
    .await;

    // Removing it alongside a replacement on the same account keeps coverage, so it goes through.
    let reconciled = reconcile_v2_ok(
        &client,
        &token,
        &transaction.id,
        vec![
            EntryOp::RemoveEntry {
                entry_id: target_entry_id.clone(),
            },
            EntryOp::CreateEntry {
                entry: NewEntry::Direct {
                    account_id: target_account_id.clone(),
                    entry_type: BalanceDirection::Debit,
                    amount: FloatMajorUnit::new(100.0),
                    effective_at: date_time::now(),
                    metadata: Secret::new(json!({ "reference_id": "REBUILT-REF" })),
                    order_id: "rebuilt-order-id".to_string(),
                    transformation_id: None,
                },
            },
        ],
    )
    .await;

    assert_eq!(reconciled.entry_id.len(), 2);
    assert!(!reconciled.entry_id.contains(&target_entry_id));
    assert_eq!(
        reconciled.status,
        ApiTransactionStatus::Matched(TransactionMatchedData::Manual)
    );

    let entries = entries_of(&client, &token, &reconciled).await;
    let rebuilt = entry_on_account(&entries, &target_account_id);
    assert_eq!(rebuilt.amount.value, FloatMajorUnit::new(100.0));
    assert_eq!(rebuilt.order_id, "rebuilt-order-id");
}

/// Accounts named by an op must exist in the profile and match the transaction's currency, and a
/// transformation config must belong to the account its entry ends up on.
#[actix_web::test]
async fn v2_validates_accounts_and_transformation_configs() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let target_entry_id = {
        let entries = entries_of(&client, &token, &transaction).await;
        entry_on_account(&entries, &target_account_id)
            .entry_id
            .clone()
    };

    let euro_account_id =
        create_account_with_currency(&client, &token, "Euro Account", Currency::EUR).await;

    // A config bound to the source account, applied to an entry on the target account.
    let source_config = create_transformation_config(
        &profile_id,
        &source_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        exposing_metadata_fields(),
    )
    .await;

    let create_on = |account_id: &str| EntryOp::CreateEntry {
        entry: NewEntry::Direct {
            account_id: account_id.to_string(),
            entry_type: BalanceDirection::Debit,
            amount: FloatMajorUnit::new(10.0),
            effective_at: date_time::now(),
            metadata: Secret::new(json!({})),
            order_id: "order_test".to_string(),
            transformation_id: None,
        },
    };

    let cases: Vec<(&str, Vec<EntryOp>, u16)> = vec![
        (
            "a created entry on an account that does not exist",
            vec![create_on("acc_not_found")],
            404,
        ),
        (
            "a created entry on an account in another currency",
            vec![create_on(&euro_account_id)],
            400,
        ),
        (
            "an override naming a transformation config that does not exist",
            vec![EntryOp::UpdateEntry {
                entry_id: target_entry_id.clone(),
                overrides: EntryOverrides {
                    transformation_id: Some("tran_conf_not_found".to_string()),
                    ..Default::default()
                },
            }],
            404,
        ),
        (
            "a transformation config bound to a different account",
            vec![EntryOp::UpdateEntry {
                entry_id: target_entry_id.clone(),
                overrides: EntryOverrides {
                    transformation_id: Some(source_config.transformation_id.clone()),
                    ..Default::default()
                },
            }],
            400,
        ),
    ];

    for (description, entry_ops, expected_status) in cases {
        let response = reconcile_v2(&client, &token, &transaction.id, entry_ops).await;

        assert_eq!(response.status(), expected_status, "{description}");
        assert_unchanged(&client, &token, &transaction, description).await;
    }
}

/// A staging entry can only be pulled in while it is unreconciled and in the same currency, and a
/// rejected link must leave it untouched.
#[actix_web::test]
async fn v2_validates_staging_entry_links() {
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

    let transaction = mismatched_transaction(
        &client,
        &token,
        &merchant_id,
        &profile_id,
        &source_account_id,
        &target_account_id,
        &rule_id,
    )
    .await;

    let linkable_config = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        exposing_metadata_fields(),
    )
    .await;

    // Never processed, so still Pending -- and Pending is not linkable.
    let pending = create_staging_entry(
        &client,
        &token,
        &target_account_id,
        &linkable_config.transformation_id,
        BalanceDirection::Debit,
        10.0,
        Currency::USD,
        json!({ "reference_id": "PENDING-REF" }),
    )
    .await;

    // Processed, but into MissingUniqueField, which is one of the statuses is_linkable() refuses.
    let missing_unique = create_staging_entry(
        &client,
        &token,
        &target_account_id,
        &linkable_config.transformation_id,
        BalanceDirection::Debit,
        10.0,
        Currency::USD,
        json!({ "some_other_field": "no unique field here" }),
    )
    .await;
    let process = client
        .post(&format!("/recon_engine/{}/process", missing_unique.id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(process.status(), 200);
    let missing_unique =
        latest_staging_entry(&client, &token, &missing_unique.staging_entry_id).await;
    assert!(
        matches!(
            missing_unique.data,
            StagingEntryData::NeedsManualReview(NeedsManualReviewData::MissingUniqueField { .. })
        ),
        "expected MissingUniqueField, got {:?}",
        missing_unique.data
    );

    // Linkable, but denominated in a currency the transaction does not use.
    let euro_holder_account_id =
        create_test_account(&client, &token, "Euro Holder", BalanceDirection::Debit).await;
    let euro_config = create_transformation_config(
        &profile_id,
        &euro_holder_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.reference_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        exposing_metadata_fields(),
    )
    .await;
    let euro_staging = link_ready_staging_entry(
        &client,
        &token,
        &euro_holder_account_id,
        &euro_config.transformation_id,
        BalanceDirection::Debit,
        10.0,
        Currency::EUR,
        "EURO-REF",
    )
    .await;

    let cases: Vec<(&str, Option<&GetStagingEntryResponse>, String, u16)> = vec![
        (
            "a staging entry that does not exist",
            None,
            "stag_ent_not_found".to_string(),
            404,
        ),
        (
            "a staging entry that is still Pending",
            Some(&pending),
            pending.id.clone(),
            400,
        ),
        (
            "a staging entry parked in MissingUniqueField",
            Some(&missing_unique),
            missing_unique.id.clone(),
            400,
        ),
        (
            "a linkable staging entry in another currency",
            Some(&euro_staging),
            euro_staging.id.clone(),
            400,
        ),
    ];

    for (description, staging_entry, staging_entry_id, expected_status) in cases {
        let response = reconcile_v2(
            &client,
            &token,
            &transaction.id,
            vec![EntryOp::CreateWithStagingEntry {
                staging_entry_id,
                overrides: StagingEntryOverrides::default(),
            }],
        )
        .await;

        assert_eq!(response.status(), expected_status, "{description}");
        assert_unchanged(&client, &token, &transaction, description).await;

        if let Some(staging_entry) = staging_entry {
            let latest =
                latest_staging_entry(&client, &token, &staging_entry.staging_entry_id).await;
            assert_ne!(
                latest.status,
                ApiStagingEntryCoarseStatus::Processed,
                "{description}: a refused link must not consume the staging entry"
            );
        }
    }
}
