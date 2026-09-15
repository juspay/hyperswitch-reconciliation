use std::collections::{BinaryHeap, HashMap};

use api_models::{
    entry::GetEntryResponse,
    recon_engine::ReconTriggerResponse,
    search_expectation::SearchExpectationResponse,
    search_group::SearchGroupResponse,
    staging_entry::{GetStagingEntryResponse, StagingEntryCreateRequestV2},
};
use common_enums::{EntryStatus, SearchGroupStatus, StagingEntryProcessingMode};
use masking::ExposeInterface;

use crate::{
    api::{
        recon_engine_test::types::{
            ReconRuleCustomConfig, ReconRuleEngine, ReconRuleEngineExt, ReconTestEntry,
        },
        recon_rule_test::create_test_recon_rule,
        transformation_test::create_transformation_config,
    },
    common::TestApp,
};

pub mod one_to_many_test;
pub mod one_to_one_test;
pub mod types;

pub async fn run_recon_engine_test(
    rule_type: ReconRuleEngine,
    rule_custom_config: ReconRuleCustomConfig,
    test_entries: Vec<ReconTestEntry>,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;

    let _recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(1),
        rule_custom_config.populate_account_ids(
            source_accounts
                .iter()
                .chain(target_accounts.iter())
                .map(|a| (a.account_name.clone(), a.account_id.clone()))
                .collect(),
        ),
        source_accounts
            .iter()
            .map(|acc| acc.account_id.clone())
            .collect(),
        target_accounts
            .iter()
            .map(|acc| acc.account_id.clone())
            .collect(),
    )
    .await;

    let account_name_to_id: HashMap<String, String> = source_accounts
        .iter()
        .chain(target_accounts.iter())
        .map(|acc| (acc.account_name.clone(), acc.account_id.clone()))
        .collect();

    let all_metadata_fields: Vec<_> = test_entries
        .iter()
        .flat_map(|e| e.staging_entry_data.get_metadata_fields())
        .collect();

    // Create Transformations for source accounts
    let mut transformations_map = HashMap::new();
    for account in source_accounts.iter() {
        let transformation_config = create_transformation_config(
            &profile_id,
            &account.account_id,
            None,
            &merchant_id,
            &token,
            &client,
            "metadata.transaction_id".parse().unwrap(),
            StagingEntryProcessingMode::Transaction,
            all_metadata_fields.clone(),
        )
        .await;
        transformations_map.insert(account.account_id.clone(), transformation_config);
    }

    // Create Transformations for target accounts
    for account in target_accounts.iter() {
        let transformation_config = create_transformation_config(
            &profile_id,
            &account.account_id,
            None,
            &merchant_id,
            &token,
            &client,
            "metadata.transaction_id".parse().unwrap(),
            StagingEntryProcessingMode::Confirmation,
            all_metadata_fields.clone(),
        )
        .await;
        transformations_map.insert(account.account_id.clone(), transformation_config);
    }

    // Create Staging Entries
    for entry in test_entries.iter() {
        let account_id = match entry.staging_entry_data.processing_mode {
            StagingEntryProcessingMode::Transaction => {
                &source_accounts
                    .iter()
                    .find(|acc| acc.account_name == entry.account_name)
                    .unwrap()
                    .account_id
            }
            StagingEntryProcessingMode::Confirmation => {
                &target_accounts
                    .iter()
                    .find(|acc| acc.account_name == entry.account_name)
                    .unwrap()
                    .account_id
            }
        };
        let transformation_config = transformations_map.get(account_id).unwrap();
        let staging_entry_create_request = StagingEntryCreateRequestV2 {
            account_id: account_id.clone(),
            transformation_id: transformation_config.transformation_id.clone(),
            metadata_schema_id: transformation_config.metadata_schema_id.clone(),
            raw_data: entry.staging_entry_data.get_raw_data(),
            transformation_history_id: None,
        };
        let create_response = client
            .post("/staging_entries/v2")
            .with_auth(&token)
            .json(&staging_entry_create_request)
            .send()
            .await;
        if create_response.status() != 201 {
            panic!(
                "staging entry creation failed: {:?}",
                create_response.text().await
            );
        }
        assert_eq!(create_response.status(), 201);
        let created_staging_entry: GetStagingEntryResponse =
            client.expect_success(create_response).await;

        let process_response = client
            .post(&format!(
                "/recon_engine/{id}/process",
                id = created_staging_entry.id
            ))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(process_response.status(), 200);
        let processed_staging_entry: ReconTriggerResponse =
            client.expect_success(process_response).await;

        let transaction = match processed_staging_entry {
            ReconTriggerResponse::Success(transaction) => *transaction,
            ReconTriggerResponse::Failure(error) => {
                panic!("Failed to process staging entry: {}", error);
            }
        };

        let entries_response = client
            .get(&format!(
                "/transactions/{transaction_id}/entries",
                transaction_id = transaction.transaction_id
            ))
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(entries_response.status(), 200);
        let entries: Vec<GetEntryResponse> = client.expect_success(entries_response).await;
        let latest_entries = entries
            .into_iter()
            .filter(|entry| entry.status != EntryStatus::Archived)
            .collect::<Vec<GetEntryResponse>>();

        let staging_entries_list = client
            .get("/staging_entries")
            .with_auth(&token)
            .send()
            .await;
        assert_eq!(staging_entries_list.status(), 200);
        let staging_entries: Vec<GetStagingEntryResponse> =
            client.expect_success(staging_entries_list).await;

        let current_staging_entry = staging_entries
            .into_iter()
            .filter(|entry| entry.staging_entry_id == created_staging_entry.staging_entry_id)
            .max_by_key(|entry| entry.version)
            .unwrap();

        let expected_result = &entry.expected_result;

        // Assertions
        assert_eq!(
            current_staging_entry.status,
            expected_result.expected_staging_entry_status
        );

        assert_eq!(
            transaction.status,
            expected_result.expected_transaction_status
        );
        if let Some(expected_version) = expected_result.expected_transaction_version {
            assert_eq!(transaction.version, expected_version);
        }
        if let Some((expected_credit, expected_debit)) =
            expected_result.expected_transaction_amounts
        {
            assert_eq!(transaction.credit_amount.value, expected_credit);
            assert_eq!(transaction.debit_amount.value, expected_debit);
        }
        assert_eq!(
            latest_entries.len(),
            expected_result.expected_entries_data.len()
        );
        expected_result
            .expected_entries_data
            .iter()
            .for_each(|expected_entry| {
                if !latest_entries.iter().any(|entry| {
                    entry.status == expected_entry.status
                        && entry.amount.value == expected_entry.amount
                        && entry.entry_type == expected_entry.entry_type
                }) {
                    panic!(
                        "Entry Not Found.\n Expected: {:#?}\n Latest_Entries: {:#?}",
                        expected_entry, latest_entries
                    );
                }
            });

        if let Some(expected_search_group_status) = expected_result.expected_search_group_status {
            let search_group_response = client
                .get(&format!(
                    "/search_groups/primary_transaction/{}",
                    transaction.id
                ))
                .with_auth(&token)
                .send()
                .await;

            match expected_search_group_status {
                SearchGroupStatus::Active => {
                    assert_eq!(
                        search_group_response.status(),
                        200,
                        "Expected search group to exist for transaction {}",
                        transaction.id
                    );
                    let search_group: SearchGroupResponse =
                        client.expect_success(search_group_response).await;
                    assert_eq!(
                        search_group.status,
                        SearchGroupStatus::Active,
                        "Search group status mismatch for transaction {}",
                        transaction.id
                    );
                }
                SearchGroupStatus::Archived => {
                    assert_eq!(
                        search_group_response.status(),
                        404,
                        "Expected no active search group to exist for transaction {}",
                        transaction.id
                    );
                }
            }
        }

        if let Some(expected_expectations) = expected_result.expected_search_expectations.as_ref() {
            let response = client
                .get(&format!(
                    "/search_expectations/primary_transaction/{}",
                    transaction.id
                ))
                .with_auth(&token)
                .send()
                .await;

            assert_eq!(
                response.status(),
                200,
                "Unexpected status from search_expectations endpoint for transaction {}",
                transaction.id
            );

            let actual: Vec<(String, String)> = client
                .expect_success::<Vec<SearchExpectationResponse>>(response)
                .await
                .into_iter()
                .map(|se| (se.target_account_id, se.hashed_source_value.expose()))
                .collect::<BinaryHeap<_>>()
                .into_sorted_vec();

            let expected: Vec<(String, String)> = expected_expectations
                .iter()
                .map(|exp| {
                    let account_id = account_name_to_id
                        .get(&exp.account_name)
                        .unwrap_or_else(|| {
                            panic!(
                                "Test fixture error: account_name {:?} not found in created accounts {:?}",
                                exp.account_name,
                                account_name_to_id.keys().collect::<Vec<_>>()
                            )
                        })
                        .clone();
                    let hashed = recon::utils::recon_engine::hash_string(&exp.source_value).expose();
                    (account_id, hashed)
                })
                .collect::<BinaryHeap<_>>()
                .into_sorted_vec();

            assert_eq!(
                actual, expected,
                "Search expectations mismatch for transaction {}",
                transaction.id
            );
        }
    }
}
