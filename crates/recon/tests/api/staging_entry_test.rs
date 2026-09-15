use crate::{
    api::{
        account_test::create_test_account, ingestion_test::create_ingestion_config,
        transformation_test::create_transformation_config,
    },
    common::TestApp,
};
use api_models::{
    account::{AccountCreateRequest, GetAccountResponse},
    recon_rule::{
        aging_config::AgingConfigApiRequest, OneToOneSingleSingle, OneToOneSingleSingleSource,
        OneToOneSingleSingleTarget, OneToOneStrategy, ReconRuleCreateRequest, ReconStrategy,
    },
    staging_entry::{
        ApiStagingEntryCoarseStatus, ApiStagingEntryListFiltersV2, ApiStagingEntryStatus,
        GetStagingEntryResponse, ListStagingEntriesV2Request, NeedsManualReviewData,
        StagingEntryCreateRequest, StagingEntryData as ApiStagingEntryData, StagingEntryListV2Page,
        StagingEntryUpdateRequest, VoidStagingEntryRequest,
    },
};
use common_enums::{BalanceDirection, Currency, ResolutionActionType, StagingEntryProcessingMode};
use common_types::{
    date_time::TimeRange,
    pagination::{CursorDirection, CursorPagination, SortOrder},
    recon_rule::{
        entry_field::EntryField,
        match_rules::{MatchOperator, MatchRuleV1, MatchRules, MatchRulesV1},
        search_identifier::{SearchIdentifier, SearchIdentifierV1},
        trigger::{
            archived::TriggerV1, BasicComparisonOperator, Trigger, TriggerOperator,
            TriggerOperatorV1,
        },
    },
    staging_entry::{
        status::{DomainStagingEntryStatus, StagingEntryManualReviewData},
        EditedData, EditedDataV1, ManualReviewData, ManualReviewDataV1, PendingData, PendingDataV1,
        ProcessingErrorType, ProcessingErrorTypeV1, StagingEntryData, StagingEntryDataV2,
        StagingEntryListSortField, VoidData, VoidDataV1,
    },
    units::FloatMajorUnit,
};
use common_utils::id_type::MerchantId;
use masking::{ExposeInterface, Secret};
use pretty_assertions::assert_eq;
use recon::utils::staging_entry::DomainStagingEntryStatusExt;
use rstest::rstest;
use serde_json::json;

// Helper function to create a recon rule
/// # Panics
/// If the request fails
pub async fn create_test_recon_rule_for_staging_entry(
    client: &crate::common::TestClient,
    token: &str,
    rule_id: &str,
    source_account_id: String,
    target_account_id: String,
) -> String {
    let request = ReconRuleCreateRequest {
        rule_name: format!("Test Rule {rule_id}"),
        rule_description: format!("Test rule for {rule_id}"),
        priority: 10,
        aging_config: AgingConfigApiRequest::NoAging,
        strategy: ReconStrategy::OneToOne(OneToOneStrategy::SingleSingle(OneToOneSingleSingle {
            source_account: OneToOneSingleSingleSource {
                account_id: source_account_id,
                trigger: Trigger::V1(TriggerV1 {
                    field: "metadata.test".parse().unwrap(),
                    operator: TriggerOperator::V1(TriggerOperatorV1 {
                        value: BasicComparisonOperator::Equals,
                    }),
                    value: "data".to_string(),
                }),
            },
            target_account: OneToOneSingleSingleTarget {
                account_id: target_account_id,
                tolerance_config: None,
            },
            match_rules: MatchRules::V1(MatchRulesV1 {
                rules: vec![MatchRuleV1 {
                    source_field: "amount".parse().unwrap(),
                    target_field: "amount".parse().unwrap(),
                    operator: MatchOperator::Equals,
                    label: None,
                }],
            }),
            search_identifier: SearchIdentifier::V1(SearchIdentifierV1 {
                source_field: "metadata.transaction_id".parse().unwrap(),
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

// Helper function to create a staging entry with a specific status and error type
pub async fn create_test_staging_entry(
    client: &crate::common::TestClient,
    token: &str,
    profile_id: &str,
    account_id: &str,
    merchant_id: &MerchantId,
    target_data: StagingEntryData,
    transformation_id: String,
) -> GetStagingEntryResponse {
    match &target_data {
        StagingEntryData::V2(staging_entry_data_v2) => match staging_entry_data_v2 {
            StagingEntryDataV2::Pending(pending_data) => {
                match pending_data {
                    PendingData::V1(v1_data) => match v1_data {
                        PendingDataV1::New => {
                            // Create a normal pending staging entry
                            let mut request = staging_entry_request(
                                account_id,
                                BalanceDirection::Credit,
                                StagingEntryProcessingMode::Transaction,
                                100.0,
                                Currency::USD,
                                transformation_id,
                            );
                            // Create transformation config for successful processing
                            let transformation = create_transformation_config(
                                profile_id,
                                account_id,
                                None,
                                merchant_id,
                                token,
                                client,
                                "metadata.transaction_id".parse().unwrap(),
                                StagingEntryProcessingMode::Transaction,
                                vec![],
                            )
                            .await;

                            request.transformation_id = transformation.transformation_id;

                            let response = client
                                .post("/staging_entries")
                                .with_auth(token)
                                .json(&request)
                                .send()
                                .await;

                            assert_eq!(response.status(), 201);
                            response.json::<GetStagingEntryResponse>().await.unwrap()
                        }
                        PendingDataV1::Edited(edited_data) => match edited_data {
                            EditedData::V1(v1_data) => {
                                // Create a staging entry that needs manual review first, then update it to get Edited status

                                // First create a NeedsManualReview entry directly
                                let mut request = staging_entry_request(
                                    account_id,
                                    BalanceDirection::Credit,
                                    StagingEntryProcessingMode::Transaction,
                                    100.0,
                                    Currency::USD,
                                    transformation_id,
                                );

                                // Create transformation config for successful processing
                                let transformation = create_transformation_config(
                                    profile_id,
                                    account_id,
                                    None,
                                    merchant_id,
                                    token,
                                    client,
                                    "metadata.transaction_id".parse().unwrap(),
                                    StagingEntryProcessingMode::Transaction,
                                    vec![],
                                )
                                .await;

                                request.transformation_id = transformation.transformation_id;
                                request.metadata = Secret::new(json!({"test": "data"})); // Missing transaction_id field to trigger NoRulesFound

                                let create_response = client
                                    .post("/staging_entries")
                                    .with_auth(token)
                                    .json(&request)
                                    .send()
                                    .await;
                                assert_eq!(create_response.status(), 201);
                                let staging_entry: GetStagingEntryResponse =
                                    create_response.json().await.unwrap();

                                // Process to get NeedsManualReview status
                                // Process to get NeedsManualReview status
                                client
                                    .post(&format!("/recon_engine/{}/process", staging_entry.id))
                                    .with_auth(token)
                                    .send()
                                    .await;

                                // Get the updated staging entry (latest version)
                                let list_response =
                                    client.get("/staging_entries").with_auth(token).send().await;

                                assert_eq!(list_response.status(), 200);
                                let all_entries: Vec<GetStagingEntryResponse> =
                                    list_response.json().await.unwrap();

                                // Find the latest version of our staging entry
                                let mut matching_entries: Vec<_> = all_entries
                                    .into_iter()
                                    .filter(|entry| {
                                        entry.staging_entry_id == staging_entry.staging_entry_id
                                    })
                                    .collect();
                                matching_entries.sort_by_key(|b| std::cmp::Reverse(b.version));

                                let latest_staging_entry =
                                    matching_entries.into_iter().next().unwrap();

                                // Then update it to create an Edited entry
                                let update_request = StagingEntryUpdateRequest {
                                    reason: v1_data.reason.clone(),
                                    account_id: latest_staging_entry.account.account_id.clone(),
                                    entry_type: latest_staging_entry.entry_type,
                                    amount: latest_staging_entry.amount.value,
                                    currency: latest_staging_entry.amount.currency,
                                    effective_at: latest_staging_entry.effective_at,
                                    metadata: Secret::new(
                                        json!({"metadata.transaction_id": "corrected_txn_123", "test": "data"}),
                                    ),
                                    transformation_id: latest_staging_entry
                                        .transformation_id
                                        .clone(),
                                    order_id: latest_staging_entry.order_id.clone(),
                                };

                                let update_response = client
                                    .put(&format!("/staging_entries/{}", latest_staging_entry.id))
                                    .with_auth(token)
                                    .json(&update_request)
                                    .send()
                                    .await;

                                assert_eq!(update_response.status(), 200);
                                update_response
                                    .json::<GetStagingEntryResponse>()
                                    .await
                                    .unwrap()
                            }
                        },
                    },
                }
            }
            StagingEntryDataV2::Processed => {
                // Create staging entry and process it to get Processed status
                // We need to create a recon rule and target account for successful processing

                // Create a target account
                let target_account_id =
                    create_test_account(client, token, "Target Account", BalanceDirection::Credit)
                        .await;

                // Create recon rule between accounts
                create_test_recon_rule_for_staging_entry(
                    client,
                    token,
                    "processed_test_rule",
                    account_id.to_string(),
                    target_account_id,
                )
                .await;

                // Create transformation config for successful processing
                let transformation = create_transformation_config(
                    profile_id,
                    account_id,
                    None,
                    merchant_id,
                    token,
                    client,
                    "metadata.transaction_id".parse().unwrap(),
                    StagingEntryProcessingMode::Transaction,
                    vec![],
                )
                .await;

                let mut request = staging_entry_request(
                    account_id,
                    BalanceDirection::Credit,
                    StagingEntryProcessingMode::Transaction,
                    100.0,
                    Currency::USD,
                    transformation_id,
                );
                request.transformation_id = transformation.transformation_id;
                request.metadata =
                    Secret::new(json!({"transaction_id": "test_txn_123", "test": "data"}));

                let create_response = client
                    .post("/staging_entries")
                    .with_auth(token)
                    .json(&request)
                    .send()
                    .await;

                assert_eq!(create_response.status(), 201);
                let staging_entry: GetStagingEntryResponse = create_response.json().await.unwrap();

                // Process the staging entry
                let _process_response = client
                    .post(&format!("/recon_engine/{}/process", staging_entry.id))
                    .with_auth(token)
                    .send()
                    .await;

                // Get the updated staging entry (latest version)
                let list_response = client.get("/staging_entries").with_auth(token).send().await;

                assert_eq!(list_response.status(), 200);
                let all_entries: Vec<GetStagingEntryResponse> = list_response.json().await.unwrap();

                // Find the latest version of our staging entry
                let mut matching_entries: Vec<_> = all_entries
                    .into_iter()
                    .filter(|entry| entry.staging_entry_id == staging_entry.staging_entry_id)
                    .collect();
                matching_entries.sort_by_key(|b| std::cmp::Reverse(b.version));

                matching_entries.into_iter().next().unwrap()
            }
            StagingEntryDataV2::NeedsManualReview(manual_review_data) => {
                match manual_review_data {
                    ManualReviewData::V1(v1_data) => match &v1_data.error_type {
                        ProcessingErrorType::V1(processing_error_type_v1) => {
                            let request = match processing_error_type_v1 {
                                ProcessingErrorTypeV1::NoRulesFound => {
                                    let mut request = staging_entry_request(
                                        account_id,
                                        BalanceDirection::Credit,
                                        StagingEntryProcessingMode::Transaction,
                                        100.0,
                                        Currency::USD,
                                        transformation_id,
                                    );

                                    // No transformation config - will result in NoRulesFound
                                    // Create transformation config first
                                    let transformation = create_transformation_config(
                                        profile_id,
                                        account_id,
                                        None,
                                        merchant_id,
                                        token,
                                        client,
                                        "metadata.transaction_id".parse().unwrap(),
                                        StagingEntryProcessingMode::Transaction,
                                        vec![],
                                    )
                                    .await;
                                    request.transformation_id = transformation.transformation_id;
                                    request.metadata =
                                        Secret::new(json!({"transaction_id": "duplicate_txn_123"}));

                                    request
                                }
                                ProcessingErrorTypeV1::DuplicateEntry => {
                                    // Create staging entry and process it to get Processed status
                                    // We need to create a recon rule and target account for successful processing

                                    // Create a target account
                                    let target_account_id = create_test_account(
                                        client,
                                        token,
                                        "Target Account",
                                        BalanceDirection::Credit,
                                    )
                                    .await;

                                    // Create recon rule between accounts
                                    create_test_recon_rule_for_staging_entry(
                                        client,
                                        token,
                                        "processed_test_rule",
                                        account_id.to_string(),
                                        target_account_id,
                                    )
                                    .await;

                                    // Create transformation config for successful processing
                                    let transformation = create_transformation_config(
                                        profile_id,
                                        account_id,
                                        None,
                                        merchant_id,
                                        token,
                                        client,
                                        "metadata.transaction_id".parse().unwrap(),
                                        StagingEntryProcessingMode::Transaction,
                                        vec![],
                                    )
                                    .await;
                                    let mut request = staging_entry_request(
                                        account_id,
                                        BalanceDirection::Credit,
                                        StagingEntryProcessingMode::Transaction,
                                        100.0,
                                        Currency::USD,
                                        transformation_id,
                                    );
                                    request.transformation_id = transformation.transformation_id;
                                    request.metadata = Secret::new(
                                        json!({"transaction_id": "test_txn_123", "test": "data"}),
                                    );

                                    let create_response = client
                                        .post("/staging_entries")
                                        .with_auth(token)
                                        .json(&request)
                                        .send()
                                        .await;

                                    assert_eq!(create_response.status(), 201);
                                    let staging_entry: GetStagingEntryResponse =
                                        create_response.json().await.unwrap();

                                    // Process the staging entry
                                    let _process_response = client
                                        .post(&format!(
                                            "/recon_engine/{}/process",
                                            staging_entry.id
                                        ))
                                        .with_auth(token)
                                        .send()
                                        .await;
                                    request
                                }
                                ProcessingErrorTypeV1::MissingUniqueField { missing_field } => {
                                    // Create transformation config with unique field but don't provide it in metadata
                                    let mut request = staging_entry_request(
                                        account_id,
                                        BalanceDirection::Credit,
                                        StagingEntryProcessingMode::Transaction,
                                        100.0,
                                        Currency::USD,
                                        transformation_id,
                                    );

                                    let transformation = create_transformation_config(
                                        profile_id,
                                        account_id,
                                        None,
                                        merchant_id,
                                        token,
                                        client,
                                        missing_field.clone(),
                                        StagingEntryProcessingMode::Transaction,
                                        vec![],
                                    )
                                    .await;
                                    request.transformation_id = transformation.transformation_id;
                                    // Don't provide the required transaction_id field
                                    request.metadata = Secret::new(json!({"other_field": "value"}));

                                    request
                                }
                                ProcessingErrorTypeV1::StagingEntryCurrencyMismatch {
                                    staging_currency,
                                    account_currency,
                                } => {
                                    // Create a target account
                                    let account_request = AccountCreateRequest {
                                        account_name: "Target Account".to_string(),
                                        account_type: BalanceDirection::Debit,
                                        currency: account_currency
                                            .parse::<Currency>()
                                            .unwrap_or(Currency::USD),
                                        initial_balance: FloatMajorUnit::new(1000.0),
                                    };
                                    let response = client
                                        .post("/accounts")
                                        .with_auth(token)
                                        .json(&account_request)
                                        .send()
                                        .await;

                                    assert_eq!(response.status(), 201);
                                    let target_account: GetAccountResponse =
                                        response.json().await.unwrap();

                                    // Create recon rule between accounts
                                    create_test_recon_rule_for_staging_entry(
                                        client,
                                        token,
                                        "processed_test_rule",
                                        account_id.to_string(),
                                        target_account.account_id,
                                    )
                                    .await;

                                    // Create staging entry with currency different from account currency
                                    let mut request = staging_entry_request(
                                        account_id,
                                        BalanceDirection::Credit,
                                        StagingEntryProcessingMode::Transaction,
                                        100.0,
                                        staging_currency
                                            .parse::<Currency>()
                                            .unwrap_or(Currency::AED),
                                        transformation_id,
                                    );

                                    let transformation = create_transformation_config(
                                        profile_id,
                                        account_id,
                                        None,
                                        merchant_id,
                                        token,
                                        client,
                                        "metadata.transaction_id".parse().unwrap(),
                                        StagingEntryProcessingMode::Transaction,
                                        vec![],
                                    )
                                    .await;
                                    request.transformation_id = transformation.transformation_id;
                                    request.metadata = Secret::new(
                                        json!({"transaction_id": "currency_mismatch_txn_123", "test": "data"}),
                                    );

                                    request
                                }
                                _ => panic!("Error Type not implemented yet: {}", ""),
                            };

                            let create_response = client
                                .post("/staging_entries")
                                .with_auth(token)
                                .json(&request)
                                .send()
                                .await;

                            assert_eq!(create_response.status(), 201);
                            let staging_entry: GetStagingEntryResponse =
                                create_response.json().await.unwrap();

                            // Process to get NeedsManualReview status
                            client
                                .post(&format!("/recon_engine/{}/process", staging_entry.id))
                                .with_auth(token)
                                .send()
                                .await;

                            // Get the updated staging entry (latest version)
                            let list_response =
                                client.get("/staging_entries").with_auth(token).send().await;

                            assert_eq!(list_response.status(), 200);
                            let all_entries: Vec<GetStagingEntryResponse> =
                                list_response.json().await.unwrap();

                            // Find the latest version of our staging entry
                            let mut matching_entries: Vec<_> = all_entries
                                .into_iter()
                                .filter(|entry| {
                                    entry.staging_entry_id == staging_entry.staging_entry_id
                                })
                                .collect();
                            matching_entries.sort_by_key(|b| std::cmp::Reverse(b.version));

                            matching_entries.into_iter().next().unwrap()
                        }
                    },
                }
            }

            StagingEntryDataV2::Void(_) | StagingEntryDataV2::Archived => {
                // Create a staging entry and then void it
                // Voiding creates a new version (Void) and archives the old version (Archived)
                let mut request = staging_entry_request(
                    account_id,
                    BalanceDirection::Credit,
                    StagingEntryProcessingMode::Transaction,
                    100.0,
                    Currency::USD,
                    transformation_id,
                );
                // Create transformation config for successful processing
                let transformation = create_transformation_config(
                    profile_id,
                    account_id,
                    None,
                    merchant_id,
                    token,
                    client,
                    "metadata.transaction_id".parse().unwrap(),
                    StagingEntryProcessingMode::Transaction,
                    vec![],
                )
                .await;
                request.transformation_id = transformation.transformation_id;

                let create_response = client
                    .post("/staging_entries")
                    .with_auth(token)
                    .json(&request)
                    .send()
                    .await;

                assert_eq!(create_response.status(), 201);
                let original_entry: GetStagingEntryResponse = create_response.json().await.unwrap();

                // Void the staging entry (this creates new version and archives the old one)
                let void_request = VoidStagingEntryRequest {
                    reason: Some("Test void for status test".to_string()),
                };

                let void_response = client
                    .put(&format!("/staging_entries/{}/void", original_entry.id))
                    .with_auth(token)
                    .json(&void_request)
                    .send()
                    .await;

                assert_eq!(void_response.status(), 200);
                let voided_entry: GetStagingEntryResponse = void_response.json().await.unwrap();
                assert_eq!(voided_entry.status, ApiStagingEntryCoarseStatus::Void);

                if ApiStagingEntryCoarseStatus::Void.eq(&DomainStagingEntryStatus::from(
                    &target_data,
                )
                .to_api_response()
                .coarse())
                {
                    voided_entry
                } else {
                    // For Archived status, fetch the updated original entry from database
                    // The original entry should now be archived after the void operation
                    let get_response = client
                        .get(&format!("/staging_entries/{}", original_entry.id))
                        .with_auth(token)
                        .send()
                        .await;

                    assert_eq!(get_response.status(), 200);
                    let archived_entry: GetStagingEntryResponse =
                        get_response.json().await.unwrap();
                    assert_eq!(archived_entry.status, ApiStagingEntryCoarseStatus::Archived);
                    archived_entry
                }
            }
        },
    }
}

// Helper function to create a staging entry request
pub fn staging_entry_request(
    account_id: &str,
    entry_type: BalanceDirection,
    processing_mode: StagingEntryProcessingMode,
    amount: f64,
    currency: Currency,
    transformation_id: String,
) -> StagingEntryCreateRequest {
    StagingEntryCreateRequest {
        account_id: account_id.to_string(),
        entry_type,
        processing_mode,
        amount: FloatMajorUnit::new(amount),
        currency,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"test": "data"})),
        transformation_id,
        transformation_history_id: None,
        order_id: "order_test".to_string(),
    }
}

pub fn list_staging_entries_v2_request(
    filters: ApiStagingEntryListFiltersV2,
    sort_by: StagingEntryListSortField,
    limit: Option<u16>,
    direction: CursorDirection,
    order: SortOrder,
) -> ListStagingEntriesV2Request {
    ListStagingEntriesV2Request {
        filters,
        cursor_payload: CursorPagination {
            limit,
            direction,
            sort_by,
            order,
        },
    }
}

pub async fn list_staging_entries_v2(
    client: &crate::common::TestClient,
    token: &str,
    request: ListStagingEntriesV2Request,
) -> StagingEntryListV2Page {
    let response = client
        .post("/staging_entries/v2/list")
        .with_auth(token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

fn staging_entry_ids(page: &StagingEntryListV2Page) -> Vec<String> {
    page.items
        .iter()
        .map(|staging_entry| staging_entry.id.clone())
        .collect()
}

async fn create_staging_entry_for_v2_listing(
    profile_id: &str,
    merchant_id: &MerchantId,
    client: &crate::common::TestClient,
    token: &str,
    account_id: &str,
    entry_type: BalanceDirection,
    amount: f64,
    currency: Currency,
    order_id: &str,
    effective_at: time::PrimitiveDateTime,
    transformation_history_id: Option<String>,
) -> GetStagingEntryResponse {
    let transformation_config = create_transformation_config(
        profile_id,
        account_id,
        None,
        merchant_id,
        token,
        client,
        EntryField::OrderId,
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let mut request = staging_entry_request(
        account_id,
        entry_type,
        StagingEntryProcessingMode::Transaction,
        amount,
        currency,
        transformation_config.transformation_id,
    );
    request.order_id = order_id.to_string();
    request.effective_at = effective_at;
    request.transformation_history_id = transformation_history_id;

    let response = client
        .post("/staging_entries")
        .with_auth(token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    response.json().await.unwrap()
}

#[actix_web::test]
async fn test_list_staging_entries_v2_paginates_by_id_cursor() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

    for index in 0..3 {
        create_staging_entry_for_v2_listing(
            &profile_id,
            &merchant_id,
            &client,
            &token,
            &account_id,
            BalanceDirection::Credit,
            100.0 + f64::from(index),
            Currency::USD,
            &format!("v2_id_cursor_order_{index}"),
            common_utils::date_time::now(),
            None,
        )
        .await;
    }

    let first_page = list_staging_entries_v2(
        &client,
        &token,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2::default(),
            StagingEntryListSortField::Id { id: None },
            Some(2),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(first_page.items.len(), 2);
    assert!(first_page.prev_cursor.is_none());
    let first_page_ids = staging_entry_ids(&first_page);
    let next_cursor = first_page
        .next_cursor
        .clone()
        .expect("first page should include a next cursor");

    match &next_cursor {
        StagingEntryListSortField::Id { id: Some(id) } => {
            assert_eq!(Some(id), first_page_ids.last());
        }
        cursor => panic!("expected id cursor, got {cursor:?}"),
    }

    let second_page = list_staging_entries_v2(
        &client,
        &token,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2::default(),
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

    let second_page_ids = staging_entry_ids(&second_page);
    assert!(first_page_ids
        .iter()
        .all(|id| !second_page_ids.contains(id)));

    let previous_page = list_staging_entries_v2(
        &client,
        &token,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2::default(),
            second_page
                .prev_cursor
                .expect("second page should include a previous cursor"),
            Some(2),
            CursorDirection::Previous,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(staging_entry_ids(&previous_page), first_page_ids);
}

#[actix_web::test]
async fn test_list_staging_entries_v2_filters_status_account_order_and_time_range() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;
    let other_account_id =
        create_test_account(&client, &token, "Other Account", BalanceDirection::Credit).await;

    let included_effective_at = common_utils::date_time::now() - time::Duration::hours(2);
    let included_history_id = "v2_filter_history".to_string();
    let included = create_staging_entry_for_v2_listing(
        &profile_id,
        &merchant_id,
        &client,
        &token,
        &account_id,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
        "v2_filter_order",
        included_effective_at,
        Some(included_history_id.clone()),
    )
    .await;

    create_staging_entry_for_v2_listing(
        &profile_id,
        &merchant_id,
        &client,
        &token,
        &account_id,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
        "v2_filter_other_order",
        included_effective_at,
        Some(included_history_id.clone()),
    )
    .await;
    create_staging_entry_for_v2_listing(
        &profile_id,
        &merchant_id,
        &client,
        &token,
        &account_id,
        BalanceDirection::Debit,
        100.0,
        Currency::EUR,
        "v2_filter_order",
        included_effective_at,
        Some(included_history_id.clone()),
    )
    .await;
    create_staging_entry_for_v2_listing(
        &profile_id,
        &merchant_id,
        &client,
        &token,
        &other_account_id,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
        "v2_filter_order",
        included_effective_at,
        Some(included_history_id.clone()),
    )
    .await;

    let page = list_staging_entries_v2(
        &client,
        &token,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2 {
                status: Some(vec![ApiStagingEntryCoarseStatus::Pending]),
                detailed_status: None,
                entry_type: Some(vec![BalanceDirection::Credit]),
                account_ids: Some(vec![account_id]),
                transformation_config_ids: Some(vec![included
                    .transformation_config
                    .transformation_config_id
                    .clone()]),
                transformation_history_id: Some(included_history_id),
                transformation_history_ids: None,
                currency: Some(vec![Currency::USD]),
                time_range: Some(TimeRange {
                    start_time: included_effective_at - time::Duration::hours(1),
                    end_time: Some(included_effective_at + time::Duration::hours(1)),
                }),
                staging_entry_id: Some(included.staging_entry_id.clone()),
                order_id: Some("v2_filter_order".to_string()),
            },
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(staging_entry_ids(&page), vec![included.id]);
    assert_eq!(page.items[0].status, ApiStagingEntryCoarseStatus::Pending);
    assert_eq!(page.items[0].order_id, "v2_filter_order");
    assert_eq!(
        page.items[0].transformation_config.transformation_config_id,
        page.items[0].transformation_id
    );
    assert_eq!(
        page.items[0]
            .transformation_config
            .transformation_config_name,
        "Test Transformation"
    );
}

#[actix_web::test]
async fn test_list_staging_entries_v2_rejects_invalid_filters() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let invalid_time_response = client
        .post("/staging_entries/v2/list")
        .with_auth(&token)
        .json(&list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2 {
                time_range: Some(TimeRange {
                    start_time: common_utils::date_time::now() + time::Duration::hours(1),
                    end_time: Some(common_utils::date_time::now()),
                }),
                ..Default::default()
            },
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(invalid_time_response.status(), 400);

    let archived_status_response = client
        .post("/staging_entries/v2/list")
        .with_auth(&token)
        .json(&list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2 {
                status: Some(vec![ApiStagingEntryCoarseStatus::Archived]),
                ..Default::default()
            },
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(archived_status_response.status(), 400);

    let archived_detailed_status_response = client
        .post("/staging_entries/v2/list")
        .with_auth(&token)
        .json(&list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2 {
                detailed_status: Some(vec![ApiStagingEntryStatus::Archived]),
                ..Default::default()
            },
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(archived_detailed_status_response.status(), 400);
}

#[actix_web::test]
async fn test_list_staging_entries_v2_filters_by_detailed_status() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let mut manual_review_entries = Vec::new();
    for error_type in [
        ProcessingErrorTypeV1::NoRulesFound,
        ProcessingErrorTypeV1::DuplicateEntry,
    ] {
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

        manual_review_entries.push(
            create_test_staging_entry(
                &client,
                &token,
                &profile_id,
                &account_id,
                &merchant_id,
                StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(ManualReviewData::V1(
                    ManualReviewDataV1 {
                        error_type: ProcessingErrorType::V1(error_type),
                    },
                ))),
                transformation_config.transformation_id,
            )
            .await,
        );
    }
    let mut manual_review_entries = manual_review_entries.into_iter();
    let no_rules_found = manual_review_entries
        .find(|entry| {
            matches!(
                entry.data,
                ApiStagingEntryData::NeedsManualReview(NeedsManualReviewData::NoRulesFound)
            )
        })
        .unwrap();
    let duplicate = manual_review_entries.next().unwrap();

    // The coarse `status` field must keep reporting the pre-flattening value, while the reason
    // moves into `detailed_status`.
    assert_eq!(
        duplicate.status,
        ApiStagingEntryCoarseStatus::NeedsManualReview
    );
    assert_eq!(
        duplicate.detailed_status,
        ApiStagingEntryStatus::NeedsManualReview(Some(
            StagingEntryManualReviewData::DuplicateEntry
        ))
    );

    // The legacy fields stay bare strings and the new ones are nested objects, so the `status` /
    // `sub_status` keys inside `detailed_status` cannot collide with the top-level `status`.
    let raw_entry: serde_json::Value = client
        .get(&format!("/staging_entries/{}", duplicate.id))
        .with_auth(&token)
        .send()
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(raw_entry["status"], json!("needs_manual_review"));
    assert_eq!(raw_entry["discarded_status"], json!(null));
    assert_eq!(
        raw_entry["detailed_status"],
        json!({ "status": "needs_manual_review", "sub_status": "duplicate_entry" })
    );
    assert_eq!(raw_entry["detailed_discarded_status"], json!(null));

    let duplicates_only = staging_entry_ids(
        &list_staging_entries_v2(
            &client,
            &token,
            list_staging_entries_v2_request(
                ApiStagingEntryListFiltersV2 {
                    detailed_status: Some(vec![ApiStagingEntryStatus::NeedsManualReview(Some(
                        StagingEntryManualReviewData::DuplicateEntry,
                    ))]),
                    ..Default::default()
                },
                StagingEntryListSortField::Id { id: None },
                Some(50),
                CursorDirection::Next,
                SortOrder::Desc,
            ),
        )
        .await,
    );
    assert!(duplicates_only.contains(&duplicate.id));
    assert!(!duplicates_only.contains(&no_rules_found.id));

    // A parent status with the `sub_status` key omitted entirely - what a caller that has not
    // migrated yet sends - must still match every sub-status.
    let bare_parent_response = client
        .post("/staging_entries/v2/list")
        .with_auth(&token)
        .json(&json!({
            "filters": { "detailed_status": [{ "status": "needs_manual_review" }] },
            "cursor_payload": {
                "limit": 50,
                "direction": "next",
                "order": "desc",
                "sort_by": { "sort_field": "id", "id": null }}}))
        .send()
        .await;

    assert_eq!(bare_parent_response.status(), 200);
    let all_manual_review = staging_entry_ids(&bare_parent_response.json().await.unwrap());
    assert!(all_manual_review.contains(&duplicate.id));
    assert!(all_manual_review.contains(&no_rules_found.id));

    let coarse_page = list_staging_entries_v2(
        &client,
        &token,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2 {
                status: Some(vec![ApiStagingEntryCoarseStatus::NeedsManualReview]),
                ..Default::default()
            },
            StagingEntryListSortField::Id { id: None },
            Some(50),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;
    let coarse_ids = staging_entry_ids(&coarse_page);
    assert!(coarse_ids.contains(&duplicate.id));
    assert!(coarse_ids.contains(&no_rules_found.id));
}

#[actix_web::test]
async fn test_list_staging_entries_v2_excludes_archived_entries() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

    let original_entry = create_staging_entry_for_v2_listing(
        &profile_id,
        &merchant_id,
        &client,
        &token,
        &account_id,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
        "v2_archived_order",
        common_utils::date_time::now(),
        None,
    )
    .await;

    let void_response = client
        .put(&format!("/staging_entries/{}/void", original_entry.id))
        .with_auth(&token)
        .json(&VoidStagingEntryRequest {
            reason: Some("archived v2 list exclusion test".to_string()),
        })
        .send()
        .await;

    assert_eq!(void_response.status(), 200);
    let voided_entry: GetStagingEntryResponse = void_response.json().await.unwrap();
    assert_eq!(voided_entry.status, ApiStagingEntryCoarseStatus::Void);

    let archived_response = client
        .get(&format!("/staging_entries/{}", original_entry.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(archived_response.status(), 200);
    let archived_entry: GetStagingEntryResponse = archived_response.json().await.unwrap();
    assert_eq!(archived_entry.status, ApiStagingEntryCoarseStatus::Archived);

    let page = list_staging_entries_v2(
        &client,
        &token,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2::default(),
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;

    let page_ids = staging_entry_ids(&page);
    assert!(!page_ids.contains(&original_entry.id));
    assert!(page_ids.contains(&voided_entry.id));
}

#[actix_web::test]
async fn test_list_staging_entries_v2_profile_isolation() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id1, profile_id1, token1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let (merchant_id2, profile_id2, token2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    let account_id1 =
        create_test_account(&client, &token1, "Account 1", BalanceDirection::Credit).await;
    let account_id2 =
        create_test_account(&client, &token2, "Account 2", BalanceDirection::Credit).await;

    let entry1 = create_staging_entry_for_v2_listing(
        &profile_id1,
        &merchant_id1,
        &client,
        &token1,
        &account_id1,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
        "v2_profile_order_1",
        common_utils::date_time::now(),
        None,
    )
    .await;
    let entry2 = create_staging_entry_for_v2_listing(
        &profile_id2,
        &merchant_id2,
        &client,
        &token2,
        &account_id2,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
        "v2_profile_order_2",
        common_utils::date_time::now(),
        None,
    )
    .await;

    let page1 = list_staging_entries_v2(
        &client,
        &token1,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2::default(),
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;
    let page1_ids = staging_entry_ids(&page1);
    assert!(page1_ids.contains(&entry1.id));
    assert!(!page1_ids.contains(&entry2.id));

    let page2 = list_staging_entries_v2(
        &client,
        &token2,
        list_staging_entries_v2_request(
            ApiStagingEntryListFiltersV2::default(),
            StagingEntryListSortField::Id { id: None },
            Some(10),
            CursorDirection::Next,
            SortOrder::Desc,
        ),
    )
    .await;
    let page2_ids = staging_entry_ids(&page2);
    assert!(page2_ids.contains(&entry2.id));
    assert!(!page2_ids.contains(&entry1.id));
}

// Test create_staging_entry with different scenarios
#[rstest]
#[case(100.50, 201)] // Valid amount
#[case(-100.0, 400)] // Negative amount
#[case(f64::INFINITY, 400)] // Infinity amount
#[actix_web::test]
async fn test_create_staging_entry(#[case] amount: f64, #[case] expected_status: u16) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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
    let request = staging_entry_request(
        &account_id,
        BalanceDirection::Credit,
        StagingEntryProcessingMode::Transaction,
        amount,
        Currency::USD,
        transformation_config.transformation_id,
    );

    let response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 201 {
        let body: GetStagingEntryResponse = response.json().await.unwrap();
        // Verify the staging_entry_nano_id format
        assert!(body.staging_entry_id.starts_with("stag_ent_main_"));
        assert_eq!(body.account.account_id, account_id);
        assert_eq!(body.entry_type, BalanceDirection::Credit);
        assert_eq!(body.amount.value.0, amount);
        assert_eq!(body.amount.currency, Currency::USD);
        assert_eq!(body.status, ApiStagingEntryCoarseStatus::Pending);
        assert_eq!(
            body.processing_mode,
            StagingEntryProcessingMode::Transaction
        );
        assert!(!body.metadata.get_inner().clone().expose().is_null());
    }
}

// Test get_staging_entry_by_id with valid and invalid scenarios
#[rstest]
#[case(true, 200)] // Valid ID
#[case(false, 404)] // Invalid ID (not found)
#[actix_web::test]
async fn test_get_staging_entry_by_id(#[case] use_valid_id: bool, #[case] expected_status: u16) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let id = if use_valid_id {
        // Create an account first
        let account_id =
            create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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

        // Create a staging entry first
        let request = staging_entry_request(
            &account_id,
            BalanceDirection::Credit,
            StagingEntryProcessingMode::Confirmation,
            100.50,
            Currency::USD,
            transformation_config.transformation_id,
        );
        let create_response = client
            .post("/staging_entries")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;
        let created_entry: GetStagingEntryResponse = create_response.json().await.unwrap();
        created_entry.id
    } else {
        // Use a non-existent ID
        uuid::Uuid::new_v4().to_string()
    };

    let response = client
        .get(&format!("/staging_entries/{id}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 200 {
        let retrieved_entry: GetStagingEntryResponse = response.json().await.unwrap();
        assert_eq!(retrieved_entry.id, id);
        assert!(retrieved_entry
            .staging_entry_id
            .starts_with("stag_ent_main_"));
        assert_eq!(retrieved_entry.entry_type, BalanceDirection::Credit);
        assert_eq!(
            retrieved_entry.processing_mode,
            StagingEntryProcessingMode::Confirmation
        );
        assert_eq!(retrieved_entry.amount.value.0, 100.50);
        assert_eq!(retrieved_entry.amount.currency, Currency::USD);
    }
}

// Test time range validation for staging entries
#[rstest]
#[case(Some(1), None, 200)] // start_time only - valid
#[case(Some(1), Some(2), 200)] // start < end - valid
#[case(Some(2), Some(1), 400)] // start > end - invalid
#[case(Some(1), Some(1), 200)] // start = end - valid
#[case(None, None, 200)] // no time filter - valid
#[actix_web::test]
async fn test_staging_entries_time_range_validation(
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account and staging entry for testing
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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
    let request = staging_entry_request(
        &account_id,
        BalanceDirection::Credit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation_config.transformation_id,
    );
    client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request)
        .send()
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
        .get(&format!("/staging_entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 200 {
        let entries: Vec<GetStagingEntryResponse> = response.json().await.unwrap();
        // For valid requests, we should get some response (exact count depends on time range)
        assert!(entries.len() <= 1); // At most 1 entry we created
    } else {
        // For invalid requests, check error response
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

// Test list staging entries by profile ID with various filter combinations
#[rstest]
#[case(None, None, None, None, 4)] // No filters - returns all 4 entries
#[case(Some("credit"), None, None, None, 2)] // Only credit entries
#[case(Some("debit"), None, None, None, 2)] // Only debit entries
#[case(None, Some("USD"), None, None, 2)] // Only USD entries
#[case(None, Some("EUR"), None, None, 1)] // Only EUR entries
#[case(None, Some("GBP"), None, None, 1)] // Only GBP entries
#[case(None, Some("USD,EUR"), None, None, 3)] // Multiple currencies
#[case(Some("credit"), Some("USD"), None, None, 1)] // Credit + USD
#[case(Some("credit"), Some("EUR"), None, None, 1)] // Debit + EUR
#[case(None, None, Some("pending"), None, 4)] // Status filter (all are pending)
#[case(Some("credit"), Some("USD"), Some("pending"), None, 1)] // All filters combined
#[case(None, None, None, Some(-1), 4)] // Time range - past hour
#[case(None, None, None, Some(1), 0)] // Time range - future (no entries)
#[actix_web::test]
async fn test_list_staging_entries_by_profile_id_with_filters(
    #[case] entry_type: Option<&str>,
    #[case] currency: Option<&str>,
    #[case] status: Option<&str>,
    #[case] time_range: Option<i64>,
    #[case] expected_count: usize,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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

    // Create staging entries with different properties
    let entries = vec![
        (BalanceDirection::Credit, Currency::USD, 100.0),
        (BalanceDirection::Debit, Currency::USD, 50.0),
        (BalanceDirection::Credit, Currency::EUR, 75.0),
        (BalanceDirection::Debit, Currency::GBP, 25.0),
    ];

    for (entry_type, currency, amount) in entries {
        let request = staging_entry_request(
            &account_id,
            entry_type,
            StagingEntryProcessingMode::Transaction,
            amount,
            currency,
            transformation_config.transformation_id.clone(),
        );
        client
            .post("/staging_entries")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;
    }

    // Build query string
    let mut query_params = Vec::new();
    if let Some(et) = entry_type {
        query_params.push(format!("entry_type={et}"));
    }
    if let Some(curr) = currency {
        query_params.push(format!("currency={curr}"));
    }
    if let Some(st) = status {
        query_params.push(format!("status={st}"));
    }
    if let Some(tr) = time_range {
        let format =
            time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z")
                .unwrap();
        let start_time = (common_utils::date_time::now() + time::Duration::hours(tr))
            .format(&format)
            .unwrap();
        query_params.push(format!("start_time={start_time}"));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    // Make request
    let response = client
        .get(&format!("/staging_entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetStagingEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), expected_count);

    // Validate filtered results match expected criteria
    for entry in &entries {
        if let Some(et) = entry_type {
            assert_eq!(entry.entry_type.to_string().to_lowercase(), et);
        }
        if let Some(curr) = currency {
            let currencies: Vec<&str> = curr.split(',').collect();
            assert!(currencies.contains(&entry.amount.currency.to_string().as_str()));
        }
        if let Some(st) = status {
            assert_eq!(entry.status.to_string().to_lowercase(), st);
        }
    }
}

// Test combined filters with time range validation
#[rstest]
#[case(Some("credit"), Some("USD"), Some(1), Some(2), 200, 0)] // Valid time range with filters (future time = no entries)
#[case(Some("debit"), Some("EUR"), Some(2), Some(1), 400, 0)] // Invalid time range with filters
#[case(None, None, Some(-1), None, 200, 4)] // Past time with start only
#[case(Some("credit"), Some("USD,EUR"), Some(-1), Some(1), 200, 2)] // Multiple currencies with time range
#[actix_web::test]
async fn test_staging_entries_combined_filters_with_time_range(
    #[case] entry_type: Option<&str>,
    #[case] currency: Option<&str>,
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_status: u16,
    #[case] expected_count: usize,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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

    // Create staging entries with different properties
    let entries = vec![
        (BalanceDirection::Credit, Currency::USD, 100.0),
        (BalanceDirection::Debit, Currency::USD, 50.0),
        (BalanceDirection::Credit, Currency::EUR, 75.0),
        (BalanceDirection::Debit, Currency::GBP, 25.0),
    ];

    for (entry_type_val, currency_val, amount) in entries {
        let request = staging_entry_request(
            &account_id,
            entry_type_val,
            StagingEntryProcessingMode::Transaction,
            amount,
            currency_val,
            transformation_config.transformation_id.clone(),
        );
        client
            .post("/staging_entries")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;
    }

    // Build query string
    let mut query_params = Vec::new();
    if let Some(et) = entry_type {
        query_params.push(format!("entry_type={et}"));
    }
    if let Some(curr) = currency {
        query_params.push(format!("currency={curr}"));
    }

    // Add time range parameters
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
        .get(&format!("/staging_entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 200 {
        let entries: Vec<GetStagingEntryResponse> = response.json().await.unwrap();
        assert_eq!(entries.len(), expected_count);

        // Validate filtered results match expected criteria
        for entry in &entries {
            if let Some(et) = entry_type {
                assert_eq!(entry.entry_type.to_string().to_lowercase(), et);
            }
            if let Some(curr) = currency {
                let currencies: Vec<&str> = curr.split(',').collect();
                assert!(currencies.contains(&entry.amount.currency.to_string().as_str()));
            }
        }
    } else {
        // For invalid requests, check error response
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

// Test edge cases for time range validation
#[rstest]
#[case(Some(0), Some(0), 200)] // start_time = end_time (same time) - valid
#[case(Some(-24), Some(-1), 200)] // Past time range - valid
#[case(Some(1), Some(24), 200)] // Future time range - valid
#[case(Some(-1), Some(-2), 400)] // start_time > end_time (past) - invalid
#[actix_web::test]
async fn test_staging_entries_time_range_edge_cases(
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account and staging entry for testing
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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
    let request = staging_entry_request(
        &account_id,
        BalanceDirection::Credit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation_config.transformation_id,
    );
    client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request)
        .send()
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

    let query_string = format!("?{}", query_params.join("&"));

    // Make request
    let response = client
        .get(&format!("/staging_entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 400 {
        // For invalid requests, check error response exists
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

// Test profile isolation - ensure profiles only see their own entries
#[actix_web::test]
async fn test_list_staging_entries_profile_isolation() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id1, profile_id1, token1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let (merchant_id2, profile_id2, token2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Create accounts for each profile
    let account_id1 =
        create_test_account(&client, &token1, "Account 1", BalanceDirection::Credit).await;

    let ingestion_config_response1 =
        create_ingestion_config(&account_id1, &profile_id1, &merchant_id1, &client).await;

    let transformation_config1 = create_transformation_config(
        &profile_id1,
        &account_id1,
        Some(&ingestion_config_response1.ingestion_id),
        &merchant_id1,
        &token1,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let account_id2 =
        create_test_account(&client, &token2, "Account 2", BalanceDirection::Debit).await;

    let ingestion_config_response2 =
        create_ingestion_config(&account_id2, &profile_id2, &merchant_id2, &client).await;

    let transformation_config2 = create_transformation_config(
        &profile_id2,
        &account_id2,
        Some(&ingestion_config_response2.ingestion_id),
        &merchant_id2,
        &token2,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Create staging entries for profile 1
    let request1 = staging_entry_request(
        &account_id1,
        BalanceDirection::Credit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation_config1.transformation_id,
    );
    client
        .post("/staging_entries")
        .with_auth(&token1)
        .json(&request1)
        .send()
        .await;

    // Create staging entries for profile 2
    let request2 = staging_entry_request(
        &account_id2,
        BalanceDirection::Debit,
        StagingEntryProcessingMode::Confirmation,
        200.0,
        Currency::EUR,
        transformation_config2.transformation_id,
    );
    client
        .post("/staging_entries")
        .with_auth(&token2)
        .json(&request2)
        .send()
        .await;

    // Profile 1 should only see its own entries
    let response = client
        .get("/staging_entries")
        .with_auth(&token1)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let entries: Vec<GetStagingEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].account.account_id, account_id1);
    assert_eq!(entries[0].amount.value.0, 100.0);

    // Profile 2 should only see its own entries
    let response = client
        .get("/staging_entries")
        .with_auth(&token2)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let entries: Vec<GetStagingEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].account.account_id, account_id2);
    assert_eq!(entries[0].amount.value.0, 200.0);
}

// Test staging entry resolutions endpoint for different status scenarios
#[rstest]
#[case("pending_initial",
    StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(PendingDataV1::New))),
    vec![] // New pending entries have no resolutions
)]
#[case("pending_edited",
    StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(PendingDataV1::Edited(EditedData::V1(EditedDataV1 {
        reason: "Test correction".to_string()
    }))))),
    vec![] // Edited pending entries have no resolutions
)]
#[case("processed",
    StagingEntryData::V2(StagingEntryDataV2::Processed),
    vec![] // Processed entries have no resolutions
)]
#[case("no_rules_found",
    StagingEntryData::V2(
        StagingEntryDataV2::NeedsManualReview(
            ManualReviewData::V1(
                ManualReviewDataV1 {
                    error_type: ProcessingErrorType::V1(
                        ProcessingErrorTypeV1::NoRulesFound
                    )}
            )
        )
    ),
    vec![
        ResolutionActionType::CreateRule,
        ResolutionActionType::LinkStagingEntriesToTransaction, ResolutionActionType::VoidStagingEntry,
        ResolutionActionType::EditStagingEntry,
    ]
)]
#[case("duplicate_entry",
    StagingEntryData::V2(
        StagingEntryDataV2::NeedsManualReview(
            ManualReviewData::V1(
                ManualReviewDataV1 {
                    error_type: ProcessingErrorType::V1(
                        ProcessingErrorTypeV1::DuplicateEntry
                    )}
            )
        )
    ),
    vec![
        ResolutionActionType::VoidStagingEntry,
        ResolutionActionType::ProcessDuplicateStagingEntry, ResolutionActionType::LinkStagingEntriesToTransaction,
        ResolutionActionType::EditStagingEntry,
    ]
)]
#[case("missing_unique_field",
    StagingEntryData::V2(
        StagingEntryDataV2::NeedsManualReview(
            ManualReviewData::V1(
                ManualReviewDataV1 {
                    error_type: ProcessingErrorType::V1(
                        ProcessingErrorTypeV1::MissingUniqueField {
                            missing_field: "metadata.transaction_id".parse().unwrap()}
                    )}
            )
        )
    ),
    vec![
        ResolutionActionType::CreateRule,
        ResolutionActionType::VoidStagingEntry,
        ResolutionActionType::EditStagingEntry,
    ]
)]
#[case("currency_mismatch",
    StagingEntryData::V2(
        StagingEntryDataV2::NeedsManualReview(
            ManualReviewData::V1(
                ManualReviewDataV1 {
                    error_type: ProcessingErrorType::V1(
                        ProcessingErrorTypeV1::StagingEntryCurrencyMismatch {
                            staging_currency: Currency::AED.to_string(),
                            account_currency: Currency::USD.to_string()}
                    )}
            )
        )
    ),
    vec![
        ResolutionActionType::CreateAccount,
        ResolutionActionType::VoidStagingEntry,
        ResolutionActionType::EditStagingEntry,
    ]
)]
#[actix_web::test]
async fn test_get_staging_entry_resolutions(
    #[case] test_name: &str,
    #[case] target_data: StagingEntryData,
    #[case] expected_resolutions: Vec<ResolutionActionType>,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account and staging entry
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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
        target_data.clone(),
        transformation_config.transformation_id,
    )
    .await;

    assert_eq!(
        staging_entry.status,
        DomainStagingEntryStatus::from(&target_data)
            .to_api_response()
            .coarse(),
        "Staging Entry status should be target status"
    );

    // Test get resolutions for the staging entry
    let response = client
        .get(&format!(
            "/exception_management/staging_entries/{}/resolutions",
            staging_entry.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        response.status(),
        200,
        "Failed for test case: {}",
        test_name
    );
    let resolutions: Vec<ResolutionActionType> = response.json().await.unwrap();

    // Verify expected resolutions
    assert_eq!(
        resolutions.len(),
        expected_resolutions.len(),
        "Resolution count mismatch for test case: {}",
        test_name
    );
    for expected_resolution in expected_resolutions {
        assert!(
            resolutions.contains(&expected_resolution),
            "Missing resolution {expected_resolution:?} for test case: {test_name}"
        );
    }
}

// Test staging entry update validation - which statuses allow correction
#[rstest]
#[case(
    StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(PendingDataV1::New))),
    400,
    "InvalidStatus"
)] // Pending not allowed
#[case(
    StagingEntryData::V2(StagingEntryDataV2::Processed),
    400,
    "InvalidStatus"
)] // Processed not allowed
#[case(StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(ManualReviewData::V1(ManualReviewDataV1 { error_type: ProcessingErrorType::V1(ProcessingErrorTypeV1::NoRulesFound) }))), 200, "Should allow correction")] // NoRulesFound allowed
#[case(StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(ManualReviewData::V1(ManualReviewDataV1 { error_type: ProcessingErrorType::V1(ProcessingErrorTypeV1::DuplicateEntry) }))), 200, "Should allow correction")] // DuplicateEntry allowed
#[case(StagingEntryData::V2(StagingEntryDataV2::Void(VoidData::V1(VoidDataV1 { reason: Some("test".to_string()) }))), 400, "InvalidStatus")] // Void not allowed
#[case(
    StagingEntryData::V2(StagingEntryDataV2::Archived),
    400,
    "InvalidStatus"
)] // Archived not allowed
#[actix_web::test]
async fn test_staging_entry_correction_validation(
    #[case] target_data: StagingEntryData,
    #[case] expected_status: u16,
    #[case] description: &str,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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

    // Create staging entry with the target status
    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        target_data,
        transformation_config.transformation_id,
    )
    .await;
    let updated_amount = 150.0;
    // Attempt to update the staging entry
    let update_request = StagingEntryUpdateRequest {
        reason: "Test correction attempt".to_string(),
        account_id: staging_entry.account.account_id.clone(),
        entry_type: staging_entry.entry_type,
        amount: FloatMajorUnit::new(updated_amount), // Changed amount
        currency: staging_entry.amount.currency,
        effective_at: staging_entry.effective_at,
        metadata: staging_entry.metadata.get_inner().clone(),
        transformation_id: staging_entry.transformation_id.clone(),
        order_id: staging_entry.order_id.clone(),
    };

    let response = client
        .put(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(response.status(), expected_status, "{}", description);

    if expected_status == 400 {
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(
            error.get("error").is_some(),
            "Should have error field for 400 responses"
        );
    } else if expected_status == 200 {
        let updated_entry: GetStagingEntryResponse = response.json().await.unwrap();
        // Verify it created a new version with Edited status (shows as Pending)
        assert_eq!(updated_entry.status, ApiStagingEntryCoarseStatus::Pending);
        assert_eq!(updated_entry.amount.value.0, updated_amount);
        assert_ne!(updated_entry.id, staging_entry.id); // New version has different ID
        assert_eq!(
            updated_entry.staging_entry_id,
            staging_entry.staging_entry_id
        ); // Same staging entry ID
        assert_eq!(updated_entry.version, staging_entry.version + 1); // Version incremented
    }
}

// Test updating individual fields
#[actix_web::test]
async fn test_update_staging_entry_amount_field() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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

    // Create a NeedsManualReview staging entry
    let target_data = StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(
        ManualReviewData::V1(ManualReviewDataV1 {
            error_type: ProcessingErrorType::V1(ProcessingErrorTypeV1::NoRulesFound),
        }),
    ));
    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        target_data,
        transformation_config.transformation_id,
    )
    .await;

    let original_amount = staging_entry.amount.value.0;
    let new_amount = 250.75;

    // Update only the amount
    let update_request = StagingEntryUpdateRequest {
        reason: "Correcting amount due to data entry error".to_string(),
        account_id: staging_entry.account.account_id.clone(),
        entry_type: staging_entry.entry_type,
        amount: FloatMajorUnit::new(new_amount),
        currency: staging_entry.amount.currency,
        effective_at: staging_entry.effective_at,
        metadata: staging_entry.metadata.get_inner().clone(),
        transformation_id: staging_entry.transformation_id.clone(),
        order_id: staging_entry.order_id.clone(),
    };

    let response = client
        .put(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let updated_entry: GetStagingEntryResponse = response.json().await.unwrap();

    // Verify amount was updated
    assert_eq!(updated_entry.amount.value.0, new_amount);
    assert_ne!(updated_entry.amount.value.0, original_amount);

    // Verify other fields remained the same
    assert_eq!(updated_entry.entry_type, staging_entry.entry_type);
    assert_eq!(updated_entry.amount.currency, staging_entry.amount.currency);
    assert_eq!(updated_entry.effective_at, staging_entry.effective_at);
    assert_eq!(
        updated_entry.transformation_id,
        staging_entry.transformation_id
    );

    // Verify versioning
    assert_eq!(updated_entry.version, staging_entry.version + 1);
    assert_eq!(
        updated_entry.staging_entry_id,
        staging_entry.staging_entry_id
    );
    assert_ne!(updated_entry.id, staging_entry.id);

    // Verify the original entry is now archived
    let original_response = client
        .get(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(original_response.status(), 200);
    let original_entry: GetStagingEntryResponse = original_response.json().await.unwrap();
    assert_eq!(original_entry.status, ApiStagingEntryCoarseStatus::Archived);
}

// Test updating multiple fields
#[actix_web::test]
async fn test_update_staging_entry_multiple_fields() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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

    // Create a NeedsManualReview staging entry
    let target_data = StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(
        ManualReviewData::V1(ManualReviewDataV1 {
            error_type: ProcessingErrorType::V1(ProcessingErrorTypeV1::MissingUniqueField {
                missing_field: "metadata.transaction_id".parse().unwrap(),
            }),
        }),
    ));
    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        target_data,
        transformation_config.transformation_id,
    )
    .await;

    // Update amount, currency, and metadata
    let new_amount = 500.0;
    let new_currency = Currency::EUR;
    let new_metadata = Secret::new(json!({
        "transaction_id": "corrected_txn_456",
        "description": "Updated transaction",
        "source": "manual_correction"
    }));

    let update_request = StagingEntryUpdateRequest {
        reason: "Correcting multiple fields after manual review".to_string(),
        account_id: staging_entry.account.account_id.clone(),
        entry_type: staging_entry.entry_type,
        amount: FloatMajorUnit::new(new_amount),
        currency: new_currency,
        effective_at: staging_entry.effective_at,
        metadata: new_metadata.clone(),
        transformation_id: staging_entry.transformation_id.clone(),
        order_id: staging_entry.order_id.clone(),
    };

    let response = client
        .put(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let updated_entry: GetStagingEntryResponse = response.json().await.unwrap();

    // Verify all updated fields
    assert_eq!(updated_entry.amount.value.0, new_amount);
    assert_eq!(updated_entry.amount.currency, new_currency);
    assert_eq!(
        updated_entry.metadata.get_inner().clone().expose(),
        new_metadata.expose()
    );

    // Verify unchanged fields
    assert_eq!(updated_entry.entry_type, staging_entry.entry_type);
    assert_eq!(updated_entry.effective_at, staging_entry.effective_at);
    assert_eq!(
        updated_entry.transformation_id,
        staging_entry.transformation_id
    );

    // Verify versioning and status
    assert_eq!(updated_entry.status, ApiStagingEntryCoarseStatus::Pending); // Edited entries show as Pending
    assert_eq!(updated_entry.version, staging_entry.version + 1);
    assert_eq!(
        updated_entry.staging_entry_id,
        staging_entry.staging_entry_id
    );
}

// Test error scenario: Non-existent staging entry ID
#[actix_web::test]
async fn test_update_staging_entry_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let non_existent_id = uuid::Uuid::new_v4().to_string();

    let update_request = StagingEntryUpdateRequest {
        reason: "Test update attempt".to_string(),
        account_id: "test_account".to_string(),
        entry_type: BalanceDirection::Credit,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"test": "data"})),
        transformation_id: "test_transformation".to_string(),
        order_id: "order_test".to_string(),
    };

    let response = client
        .put(&format!("/staging_entries/{non_existent_id}"))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        404,
        "Should return 404 for non-existent staging entry"
    );

    // Verify error response structure
    let error: serde_json::Value = response.json().await.unwrap();
    assert!(
        error.get("error").is_some(),
        "Should have error field in response"
    );
}

// Test error scenario: Profile isolation (different profile trying to access entry)
#[actix_web::test]
async fn test_update_staging_entry_profile_isolation() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id1, profile_id1, token1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let (_, _, token2) = client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Create staging entry with profile 1
    let account_id =
        create_test_account(&client, &token1, "Test Account", BalanceDirection::Credit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id1, &merchant_id1, &client).await;

    let transformation_config = create_transformation_config(
        &profile_id1,
        &account_id,
        Some(&ingestion_config_response.ingestion_id),
        &merchant_id1,
        &token1,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;
    let target_data = StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(
        ManualReviewData::V1(ManualReviewDataV1 {
            error_type: ProcessingErrorType::V1(ProcessingErrorTypeV1::NoRulesFound),
        }),
    ));
    let staging_entry = create_test_staging_entry(
        &client,
        &token1,
        &profile_id1,
        &account_id,
        &merchant_id1,
        target_data,
        transformation_config.transformation_id,
    )
    .await;

    // Try to update with profile 2's token
    let update_request = StagingEntryUpdateRequest {
        reason: "Test update attempt from different profile".to_string(),
        account_id: staging_entry.account.account_id.clone(),
        entry_type: staging_entry.entry_type,
        amount: FloatMajorUnit::new(200.0),
        currency: staging_entry.amount.currency,
        effective_at: staging_entry.effective_at,
        metadata: staging_entry.metadata.get_inner().clone(),
        transformation_id: staging_entry.transformation_id.clone(),
        order_id: staging_entry.order_id.clone(),
    };

    let response = client
        .put(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token2) // Using different profile's token
        .json(&update_request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        404,
        "Should return 404 when accessing entry from different profile"
    );

    // Verify error response structure
    let error: serde_json::Value = response.json().await.unwrap();
    assert!(
        error.get("error").is_some(),
        "Should have error field in response"
    );
}

// Test error scenario: Invalid request data
#[actix_web::test]
async fn test_update_staging_entry_invalid_data() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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
    let target_data = StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(
        ManualReviewData::V1(ManualReviewDataV1 {
            error_type: ProcessingErrorType::V1(ProcessingErrorTypeV1::NoRulesFound),
        }),
    ));
    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        target_data,
        transformation_config.transformation_id,
    )
    .await;

    // Test with invalid amount (negative)
    let invalid_update_request = StagingEntryUpdateRequest {
        reason: "Test with invalid amount".to_string(),
        account_id: staging_entry.account.account_id.clone(),
        entry_type: staging_entry.entry_type,
        amount: FloatMajorUnit::new(-100.0), // Invalid negative amount
        currency: staging_entry.amount.currency,
        effective_at: staging_entry.effective_at,
        metadata: staging_entry.metadata.get_inner().clone(),
        transformation_id: staging_entry.transformation_id.clone(),
        order_id: staging_entry.order_id.clone(),
    };

    let response = client
        .put(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token)
        .json(&invalid_update_request)
        .send()
        .await;

    // Should return 400 for invalid data
    assert!(
        response.status() == 400 || response.status() == 422,
        "Should return 400 or 422 for invalid amount, got: {}",
        response.status()
    );

    // Verify error response structure for any error status
    if !response.status().is_success() {
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(
            error.get("error").is_some(),
            "Should have error field in error response"
        );
    }
}

// Test the complete update workflow
#[actix_web::test]
async fn test_staging_entry_update_workflow() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Credit).await;

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

    // Step 1: Create a staging entry that will become NeedsManualReview
    let target_data = StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(
        ManualReviewData::V1(ManualReviewDataV1 {
            error_type: ProcessingErrorType::V1(ProcessingErrorTypeV1::NoRulesFound),
        }),
    ));
    let staging_entry = create_test_staging_entry(
        &client,
        &token,
        &profile_id,
        &account_id,
        &merchant_id,
        target_data,
        transformation_config.transformation_id,
    )
    .await;

    // Verify NeedsManualReview state
    assert_eq!(
        staging_entry.status,
        ApiStagingEntryCoarseStatus::NeedsManualReview
    );
    assert_eq!(staging_entry.version, 2);

    // Step 2: Update the staging entry
    let update_request = StagingEntryUpdateRequest {
        reason: "Manual correction after review - fixing amount and adding transaction ID"
            .to_string(),
        account_id: staging_entry.account.account_id.clone(),
        entry_type: staging_entry.entry_type,
        amount: FloatMajorUnit::new(999.99),
        currency: staging_entry.amount.currency,
        effective_at: staging_entry.effective_at,
        metadata: Secret::new(json!({
            "transaction_id": "manual_correction_txn_789",
            "corrected_by": "operator_123",
            "correction_reason": "amount_error"
        })),
        transformation_id: staging_entry.transformation_id.clone(),
        order_id: staging_entry.order_id.clone(),
    };

    let update_response = client
        .put(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), 200);
    let updated_entry: GetStagingEntryResponse = update_response.json().await.unwrap();

    // Step 3: Verify the updated entry
    assert_eq!(updated_entry.status, ApiStagingEntryCoarseStatus::Pending); // Edited entries show as Pending
    assert_eq!(updated_entry.amount.value.0, 999.99);
    assert_eq!(updated_entry.version, staging_entry.version + 1); // Version incremented
    assert_ne!(updated_entry.id, staging_entry.id); // New version
    assert_eq!(
        updated_entry.staging_entry_id,
        staging_entry.staging_entry_id
    ); // Same group

    // Verify the metadata was updated
    let updated_metadata = updated_entry.metadata.get_inner().clone().expose();
    assert!(updated_metadata["transaction_id"]
        .as_str()
        .unwrap()
        .contains("manual_correction"));

    // Step 4: Verify the original entry is archived
    let original_response = client
        .get(&format!("/staging_entries/{}", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(original_response.status(), 200);
    let original_entry: GetStagingEntryResponse = original_response.json().await.unwrap();
    assert_eq!(original_entry.status, ApiStagingEntryCoarseStatus::Archived);

    // Step 5: Verify both entries show up in the list
    let list_response = client
        .get("/staging_entries")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(list_response.status(), 200);
    let all_entries: Vec<GetStagingEntryResponse> = list_response.json().await.unwrap();

    let related_entries: Vec<_> = all_entries
        .into_iter()
        .filter(|entry| entry.staging_entry_id == staging_entry.staging_entry_id)
        .collect();

    assert_eq!(related_entries.len(), 3); // (Archived, Archived and Pending)
}
