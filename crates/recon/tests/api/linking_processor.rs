use api_models::{
    bulk_operations::{
        BulkOperationResultItem, BulkResultStatus, TransactionBulkAction,
        TransactionBulkOperationRequest, TransactionBulkSelection,
    },
    entry::{GetEntryResponse, LinkedEntryResponse},
    link_config::{
        link_matching_key::{LinkIdentifierKey, SingleLinkIdentifier},
        link_trigger::{LinkTriggerCondition, LinkTriggerGroup, LinkTriggerOperator},
        GetLinkConfigResponse, LinkConfigCreateRequest, LinkSearchIdentifier, LinkTrigger,
    },
    linking_processor::LinkingTriggerResponse,
    transaction::{
        DirectTransactionEntry, ExpectedTransactionEntry, GetTransactionResponse,
        SearchExpectation, TransactionEntry, TransactionRequest,
    },
};
use common_enums::{
    BalanceDirection, Currency, EntryLinkSide, EntryLinkStatus, EntryStatus,
    StagingEntryProcessingMode,
};
use common_types::{recon_rule::entry_field::EntryField, units::FloatMajorUnit};
use common_utils::id_type::MerchantId;
use masking::Secret;
use rstest::rstest;
use serde_json::{json, Value};

use crate::{
    api::{
        account_test::create_test_account,
        recon_engine_test::types::{ReconRuleCustomConfig, ReconRuleEngine},
        recon_rule_test::create_test_recon_rule,
        transformation_test::create_transformation_config,
    },
    common::{TestApp, TestClient},
};

struct LinkingSetup {
    config: LinkConfigCreateRequest,
    balancing_account_id: String,
    transaction_transformation_id: String,
    confirmation_transformation_id: String,
}

async fn setup_linking(
    client: &TestClient,
    token: &str,
    merchant_id: &MerchantId,
    profile_id: &str,
) -> LinkingSetup {
    let account_id = create_test_account(client, token, "PSP", BalanceDirection::Debit).await;
    let balancing_account_id =
        create_test_account(client, token, "Bank", BalanceDirection::Credit).await;
    let mut rules = Vec::new();
    for name in ["Transaction rule", "Confirmation rule"] {
        let rule = create_test_recon_rule(
            client,
            token,
            ReconRuleEngine::OneToOneSingleSingleV1(("PSP".to_owned(), "Bank".to_owned())),
            Some(name.to_owned()),
            None,
            None,
            ReconRuleCustomConfig::default(),
            vec![account_id.clone()],
            vec![balancing_account_id.clone()],
        )
        .await;
        rules.push(rule.rule_id);
    }
    let mut transformations = Vec::new();
    for mode in [
        StagingEntryProcessingMode::Transaction,
        StagingEntryProcessingMode::Confirmation,
    ] {
        let transformation = create_transformation_config(
            profile_id,
            &account_id,
            None,
            merchant_id,
            token,
            client,
            EntryField::OrderId,
            mode,
            vec![],
        )
        .await;
        transformations.push(transformation.transformation_id);
    }
    let trigger = LinkTrigger {
        any: vec![LinkTriggerGroup {
            all: vec![LinkTriggerCondition {
                field: EntryField::Currency,
                operator: LinkTriggerOperator::Equals,
                value: "USD".to_owned(),
            }],
        }],
    };
    LinkingSetup {
        config: LinkConfigCreateRequest {
            account_id,
            transaction_rule_id: rules[0].clone(),
            confirmation_rule_id: rules[1].clone(),
            transaction_trigger: trigger.clone(),
            confirmation_trigger: trigger,
            link_search_identifier: LinkSearchIdentifier::Single(SingleLinkIdentifier {
                key: LinkIdentifierKey {
                    transaction_field: EntryField::OrderId,
                    confirmation_field: EntryField::Metadata("reference".to_owned()),
                },
            }),
            priority: 10,
        },
        balancing_account_id,
        transaction_transformation_id: transformations[0].clone(),
        confirmation_transformation_id: transformations[1].clone(),
    }
}

async fn create_config(
    client: &TestClient,
    token: &str,
    request: &LinkConfigCreateRequest,
) -> String {
    let response = client
        .post("/link_configs")
        .with_auth(token)
        .json(request)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let config: GetLinkConfigResponse = client.expect_success(response).await;
    config.link_config_id
}

async fn update_config(client: &TestClient, token: &str, id: &str, update: Value) {
    let response = client
        .put(&format!("/link_configs/{id}"))
        .with_auth(token)
        .json(&update)
        .send()
        .await;
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
}

fn transaction_request(
    setup: &LinkingSetup,
    side: EntryLinkSide,
    value: &str,
) -> TransactionRequest {
    let (rule_id, transformation_id) = match side {
        EntryLinkSide::Transaction => (
            &setup.config.transaction_rule_id,
            &setup.transaction_transformation_id,
        ),
        EntryLinkSide::Confirmation => (
            &setup.config.confirmation_rule_id,
            &setup.confirmation_transformation_id,
        ),
    };
    let direct = DirectTransactionEntry {
        account_id: setup.config.account_id.clone(),
        entry_type: BalanceDirection::Debit,
        amount: FloatMajorUnit::new(10.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"reference": value})),
        staging_entry_id: None,
        transformation_id: Some(transformation_id.clone()),
        order_id: value.to_owned(),
    };
    let balancing = DirectTransactionEntry {
        account_id: setup.balancing_account_id.clone(),
        entry_type: BalanceDirection::Credit,
        transformation_id: None,
        ..direct.clone()
    };
    TransactionRequest {
        rule_id: rule_id.clone(),
        entries: vec![
            TransactionEntry::Direct(direct),
            TransactionEntry::Direct(balancing),
        ],
    }
}

async fn create_entries(
    client: &TestClient,
    token: &str,
    request: &TransactionRequest,
    posted: bool,
) -> Vec<GetEntryResponse> {
    let response = client
        .post("/transactions")
        .with_auth(token)
        .json(request)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let transaction: GetTransactionResponse = client.expect_success(response).await;
    if posted {
        let response = client
            .post("/transactions/bulk_operations")
            .with_auth(token)
            .json(&TransactionBulkOperationRequest {
                action: TransactionBulkAction::ManualPost {
                    reason: Some("Linking test".to_owned()),
                },
                selection: TransactionBulkSelection::Ids {
                    ids: vec![transaction.id.clone()],
                },
            })
            .send()
            .await;
        assert_eq!(response.status(), 200);
        let results: Vec<BulkOperationResultItem> = client.expect_success(response).await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].status,
            BulkResultStatus::Success {
                logical_id: transaction.transaction_id.clone()
            }
        );
    }
    let response = client
        .get(&format!(
            "/transactions/{}/entries",
            transaction.transaction_id
        ))
        .with_auth(token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = client.expect_success(response).await;
    let current: Vec<_> = entries
        .into_iter()
        .filter(|entry| entry.status != EntryStatus::Archived)
        .collect();
    assert_eq!(current.len(), request.entries.len());
    if posted {
        assert!(current
            .iter()
            .all(|entry| entry.status == EntryStatus::Posted));
    }
    current
}

async fn create_entry(
    client: &TestClient,
    token: &str,
    setup: &LinkingSetup,
    side: EntryLinkSide,
    posted: bool,
    value: &str,
) -> GetEntryResponse {
    create_entries(
        client,
        token,
        &transaction_request(setup, side, value),
        posted,
    )
    .await
    .into_iter()
    .find(|entry| entry.account_id == setup.config.account_id)
    .unwrap()
}

async fn get_entry(client: &TestClient, token: &str, entry_id: &str) -> GetEntryResponse {
    let response = client
        .get(&format!("/entries/{entry_id}"))
        .with_auth(token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    client.expect_success(response).await
}

async fn linked_entries(
    client: &TestClient,
    token: &str,
    entry_id: &str,
) -> Vec<LinkedEntryResponse> {
    let response = client
        .get(&format!("/entries/{entry_id}/linked_entries"))
        .with_auth(token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    client.expect_success(response).await
}

async fn process_entry(
    client: &TestClient,
    token: &str,
    entry: &GetEntryResponse,
) -> LinkingTriggerResponse {
    let response = client
        .post(&format!("/linking_processor/{}/process", entry.entry_id))
        .with_admin_key()
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let result = client.expect_success(response).await;
    // Linking must preserve the reconciliation fields exposed by the entry API.
    let after = get_entry(client, token, &entry.entry_id).await;
    assert_eq!(
        serde_json::to_value(after).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
    result
}

async fn process_success(client: &TestClient, token: &str, entry: &GetEntryResponse) {
    let result = process_entry(client, token, entry).await;
    assert!(
        matches!(result, LinkingTriggerResponse::Success),
        "{result:?}"
    );
}

async fn process_failure(client: &TestClient, token: &str, entry: &GetEntryResponse, reason: &str) {
    match process_entry(client, token, entry).await {
        LinkingTriggerResponse::Failure(actual) => assert_eq!(actual, reason),
        LinkingTriggerResponse::Success => panic!("Expected linking failure: {reason}"),
    }
    assert!(linked_entries(client, token, &entry.entry_id)
        .await
        .is_empty());
}

async fn assert_links(
    client: &TestClient,
    token: &str,
    entry: &GetEntryResponse,
    counterparts: &[&GetEntryResponse],
    config_id: &str,
    status: EntryLinkStatus,
) -> Vec<LinkedEntryResponse> {
    let links = linked_entries(client, token, &entry.entry_id).await;
    assert_eq!(links.len(), counterparts.len());
    for counterpart in counterparts {
        let matching: Vec<_> = links
            .iter()
            .filter(|link| link.entry.entry_id == counterpart.entry_id)
            .collect();
        assert_eq!(matching.len(), 1);
        let link = matching[0];
        assert!(!link.entry_link_id.is_empty());
        assert_eq!(link.link_config_id, config_id);
        assert_eq!(link.status, status);
        assert_eq!(
            serde_json::to_value(&link.entry).unwrap(),
            serde_json::to_value(counterpart).unwrap()
        );
    }
    links
}

async fn assert_api_error(response: reqwest::Response, status: u16, code: &str) {
    assert_eq!(response.status(), status);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], code, "{body}");
}

fn opposite(side: EntryLinkSide) -> EntryLinkSide {
    match side {
        EntryLinkSide::Transaction => EntryLinkSide::Confirmation,
        EntryLinkSide::Confirmation => EntryLinkSide::Transaction,
    }
}

#[rstest]
#[case(EntryLinkSide::Transaction, false)]
#[case(EntryLinkSide::Confirmation, false)]
#[case(EntryLinkSide::Transaction, true)]
#[case(EntryLinkSide::Confirmation, true)]
#[actix_web::test]
async fn linking_matches_both_arrival_orders(
    #[case] first_side: EntryLinkSide,
    #[case] posted: bool,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    let config = create_config(&client, &token, &setup.config).await;
    let first = create_entry(&client, &token, &setup, first_side, posted, "shared").await;
    let second = create_entry(
        &client,
        &token,
        &setup,
        opposite(first_side),
        posted,
        "shared",
    )
    .await;
    assert!(linked_entries(&client, &token, &first.entry_id)
        .await
        .is_empty());
    process_success(&client, &token, &first).await;
    assert!(linked_entries(&client, &token, &first.entry_id)
        .await
        .is_empty());
    process_success(&client, &token, &second).await;
    let status = if posted {
        EntryLinkStatus::Final
    } else {
        EntryLinkStatus::Provisional
    };
    let forward = assert_links(&client, &token, &first, &[&second], &config, status).await;
    let reverse = assert_links(&client, &token, &second, &[&first], &config, status).await;
    assert_eq!(forward[0].entry_link_id, reverse[0].entry_link_id);
    for request in [
        client
            .post(&format!("/linking_processor/{}/process", first.entry_id))
            .with_admin_key(),
        client
            .post(&format!("/linking_processor/{}/enqueue", first.entry_id))
            .with_auth(&token),
    ] {
        let response = request.send().await;
        assert_api_error(response, 400, "LP_802").await;
    }
    assert_eq!(
        serde_json::to_value(linked_entries(&client, &token, &first.entry_id).await).unwrap(),
        serde_json::to_value(forward).unwrap()
    );
}

#[rstest]
#[case(EntryLinkSide::Transaction)]
#[case(EntryLinkSide::Confirmation)]
#[actix_web::test]
async fn provisional_linking_fans_out_and_reuses_candidates(#[case] side: EntryLinkSide) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    let config = create_config(&client, &token, &setup.config).await;
    let first = create_entry(&client, &token, &setup, side, false, "fan-out").await;
    let second = create_entry(&client, &token, &setup, side, false, "fan-out").await;
    process_success(&client, &token, &first).await;
    process_success(&client, &token, &second).await;
    assert!(linked_entries(&client, &token, &first.entry_id)
        .await
        .is_empty());
    let third = create_entry(&client, &token, &setup, opposite(side), false, "fan-out").await;
    process_success(&client, &token, &third).await;
    let fourth = create_entry(&client, &token, &setup, opposite(side), false, "fan-out").await;
    process_success(&client, &token, &fourth).await;
    for entry in [&first, &second] {
        let links = assert_links(
            &client,
            &token,
            entry,
            &[&third, &fourth],
            &config,
            EntryLinkStatus::Provisional,
        )
        .await;
        assert_eq!(links[0].entry.entry_id, third.entry_id);
        assert_eq!(links[1].entry.entry_id, fourth.entry_id);
    }
    for entry in [&third, &fourth] {
        assert_links(
            &client,
            &token,
            entry,
            &[&first, &second],
            &config,
            EntryLinkStatus::Provisional,
        )
        .await;
    }
}

#[rstest]
#[case(EntryLinkSide::Transaction)]
#[case(EntryLinkSide::Confirmation)]
#[actix_web::test]
async fn multiple_posted_candidates_fail_without_creating_links(#[case] side: EntryLinkSide) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    create_config(&client, &token, &setup.config).await;
    let first = create_entry(&client, &token, &setup, side, true, "ambiguous").await;
    let second = create_entry(&client, &token, &setup, side, true, "ambiguous").await;
    process_success(&client, &token, &first).await;
    process_success(&client, &token, &second).await;
    let incoming = create_entry(&client, &token, &setup, opposite(side), true, "ambiguous").await;
    process_failure(&client, &token, &incoming, "Multiple Posted links found").await;
    for entry in [&first, &second] {
        assert!(linked_entries(&client, &token, &entry.entry_id)
            .await
            .is_empty());
    }
}

#[actix_web::test]
async fn posted_linking_consumes_each_candidate_once() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    let config = create_config(&client, &token, &setup.config).await;
    let first = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Transaction,
        true,
        "final",
    )
    .await;
    let second = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Confirmation,
        true,
        "final",
    )
    .await;
    process_success(&client, &token, &first).await;
    process_success(&client, &token, &second).await;
    let third = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Transaction,
        true,
        "final",
    )
    .await;
    process_success(&client, &token, &third).await;
    assert!(linked_entries(&client, &token, &third.entry_id)
        .await
        .is_empty());
    let fourth = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Confirmation,
        true,
        "final",
    )
    .await;
    process_success(&client, &token, &fourth).await;
    for (entry, counterpart) in [
        (&first, &second),
        (&second, &first),
        (&third, &fourth),
        (&fourth, &third),
    ] {
        assert_links(
            &client,
            &token,
            entry,
            &[counterpart],
            &config,
            EntryLinkStatus::Final,
        )
        .await;
    }
}

#[actix_web::test]
async fn different_values_and_posted_phases_do_not_cross_match() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    create_config(&client, &token, &setup.config).await;
    let mut entries = Vec::new();
    for (side, posted, value) in [
        (EntryLinkSide::Transaction, false, "provisional"),
        (EntryLinkSide::Confirmation, false, "different"),
        (EntryLinkSide::Confirmation, true, "provisional"),
        (EntryLinkSide::Transaction, true, "different"),
    ] {
        let entry = create_entry(&client, &token, &setup, side, posted, value).await;
        process_success(&client, &token, &entry).await;
        entries.push(entry);
    }
    for entry in entries {
        assert!(linked_entries(&client, &token, &entry.entry_id)
            .await
            .is_empty());
    }
}

#[actix_web::test]
async fn failed_linking_can_be_fixed_and_enqueued_once() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    let entry = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Transaction,
        false,
        "retry",
    )
    .await;
    process_failure(
        &client,
        &token,
        &entry,
        "No Link Config found for processing",
    )
    .await;
    let process_path = format!("/linking_processor/{}/process", entry.entry_id);
    assert_api_error(
        client.post(&process_path).with_admin_key().send().await,
        400,
        "LP_802",
    )
    .await;
    let config = create_config(&client, &token, &setup.config).await;
    let enqueue_path = format!("/linking_processor/{}/enqueue", entry.entry_id);
    assert_eq!(
        client
            .post(&enqueue_path)
            .with_auth(&token)
            .send()
            .await
            .status(),
        200
    );
    assert_api_error(
        client.post(&enqueue_path).with_auth(&token).send().await,
        400,
        "LP_802",
    )
    .await;
    process_success(&client, &token, &entry).await;
    let counterpart = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Confirmation,
        false,
        "retry",
    )
    .await;
    process_success(&client, &token, &counterpart).await;
    assert_links(
        &client,
        &token,
        &entry,
        &[&counterpart],
        &config,
        EntryLinkStatus::Provisional,
    )
    .await;
}

#[rstest]
#[case(EntryLinkSide::Transaction, false)]
#[case(EntryLinkSide::Confirmation, false)]
#[case(EntryLinkSide::Transaction, true)]
#[case(EntryLinkSide::Confirmation, true)]
#[actix_web::test]
async fn missing_identifiers_fail_for_single_and_composite_keys(
    #[case] side: EntryLinkSide,
    #[case] composite: bool,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let mut setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    setup.config.link_search_identifier = serde_json::from_value(if composite {
        json!({"matching_key_type": "composite", "delimiter": "pipe", "keys": [
            {"transaction_field": "order_id", "confirmation_field": "metadata.reference"},
            {"transaction_field": "metadata.missing", "confirmation_field": "metadata.missing"}
        ]})
    } else {
        json!({"matching_key_type": "single", "key": {"transaction_field": "metadata.missing", "confirmation_field": "metadata.missing"}})
    }).unwrap();
    create_config(&client, &token, &setup.config).await;
    let entry = create_entry(&client, &token, &setup, side, false, "missing").await;
    process_failure(
        &client,
        &token,
        &entry,
        "Missing search identifier value for field: metadata.missing",
    )
    .await;
}

#[actix_web::test]
async fn entry_without_transformation_records_failure() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    create_config(&client, &token, &setup.config).await;
    let mut request = transaction_request(&setup, EntryLinkSide::Transaction, "manual-entry");
    for entry in &mut request.entries {
        if let TransactionEntry::Direct(entry) = entry {
            entry.transformation_id = None;
        }
    }
    let entries = create_entries(&client, &token, &request, false).await;
    for entry in entries {
        process_failure(&client, &token, &entry, "Missing transformation config id").await;
    }
}

#[rstest]
#[case(EntryLinkSide::Transaction)]
#[case(EntryLinkSide::Confirmation)]
#[actix_web::test]
async fn highest_priority_matching_config_uses_side_specific_triggers(#[case] side: EntryLinkSide) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    create_config(&client, &token, &setup.config).await;
    let mut preferred = setup.config.clone();
    preferred.priority = 20;
    let matching: LinkTrigger = serde_json::from_value(json!({"any": [
        {"all": [{"field": "currency", "operator": "equals", "value": "EUR"}]},
        {"all": [
            {"field": "currency", "operator": "equals", "value": "USD"},
            {"field": "order_id", "operator": "not_equals", "value": "excluded"}
        ]}
    ]}))
    .unwrap();
    match side {
        EntryLinkSide::Transaction => {
            preferred.transaction_trigger = matching.clone();
            preferred.confirmation_trigger.any[0].all[0].value = "EUR".to_owned();
        }
        EntryLinkSide::Confirmation => {
            preferred.confirmation_trigger = matching.clone();
            preferred.transaction_trigger.any[0].all[0].value = "EUR".to_owned();
        }
    }
    let config = create_config(&client, &token, &preferred).await;
    let mut excluded = setup.config.clone();
    excluded.priority = 100;
    let inactive = create_config(&client, &token, &excluded).await;
    update_config(&client, &token, &inactive, json!({"is_active": false})).await;
    excluded.priority = 90;
    excluded.transaction_trigger = serde_json::from_value(json!({"any": [
        {"all": [
            {"field": "currency", "operator": "equals", "value": "USD"},
            {"field": "order_id", "operator": "equals", "value": "excluded"}
        ]},
        {"all": [{"field": "metadata.absent", "operator": "not_equals", "value": "anything"}]}
    ]}))
    .unwrap();
    excluded.confirmation_trigger = excluded.transaction_trigger.clone();
    create_config(&client, &token, &excluded).await;
    let entry = create_entry(&client, &token, &setup, side, false, "eligible").await;
    process_success(&client, &token, &entry).await;
    update_config(
        &client,
        &token,
        &config,
        json!({"transaction_trigger": matching, "confirmation_trigger": matching}),
    )
    .await;
    let counterpart =
        create_entry(&client, &token, &setup, opposite(side), false, "eligible").await;
    process_success(&client, &token, &counterpart).await;
    assert_links(
        &client,
        &token,
        &entry,
        &[&counterpart],
        &config,
        EntryLinkStatus::Provisional,
    )
    .await;
}

#[rstest]
#[case("inactive")]
#[case("wrong_account")]
#[case("wrong_rule")]
#[case("non_matching_trigger")]
#[actix_web::test]
async fn ineligible_config_records_failure(#[case] exclusion: &str) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    let mut request = setup.config.clone();
    match exclusion {
        "wrong_account" => request.account_id = setup.balancing_account_id.clone(),
        "wrong_rule" => std::mem::swap(
            &mut request.transaction_rule_id,
            &mut request.confirmation_rule_id,
        ),
        "non_matching_trigger" => {
            request.transaction_trigger.any[0].all[0].value = "EUR".to_owned();
        }
        _ => {}
    }
    let config = create_config(&client, &token, &request).await;
    if exclusion == "inactive" {
        update_config(&client, &token, &config, json!({"is_active": false})).await;
    }
    let entry = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Transaction,
        false,
        "excluded",
    )
    .await;
    process_failure(
        &client,
        &token,
        &entry,
        "No Link Config found for processing",
    )
    .await;
}

#[actix_web::test]
async fn matching_values_are_isolated_by_config() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    create_config(&client, &token, &setup.config).await;
    let first = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Transaction,
        false,
        "shared",
    )
    .await;
    process_success(&client, &token, &first).await;
    let mut request = setup.config.clone();
    request.priority = 20;
    let config = create_config(&client, &token, &request).await;
    let second = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Confirmation,
        false,
        "shared",
    )
    .await;
    process_success(&client, &token, &second).await;
    assert!(linked_entries(&client, &token, &first.entry_id)
        .await
        .is_empty());
    assert!(linked_entries(&client, &token, &second.entry_id)
        .await
        .is_empty());
    let third = create_entry(
        &client,
        &token,
        &setup,
        EntryLinkSide::Transaction,
        false,
        "shared",
    )
    .await;
    process_success(&client, &token, &third).await;
    assert_links(
        &client,
        &token,
        &second,
        &[&third],
        &config,
        EntryLinkStatus::Provisional,
    )
    .await;
    assert!(linked_entries(&client, &token, &first.entry_id)
        .await
        .is_empty());
}

#[actix_web::test]
async fn expected_entries_cannot_be_processed_or_enqueued() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let setup = setup_linking(&client, &token, &merchant_id, &profile_id).await;
    create_config(&client, &token, &setup.config).await;
    let mut request = transaction_request(&setup, EntryLinkSide::Transaction, "expected");
    let TransactionEntry::Direct(balancing) = request.entries.pop().unwrap() else {
        panic!("Expected direct entry");
    };
    request
        .entries
        .push(TransactionEntry::Expected(ExpectedTransactionEntry {
            account_id: balancing.account_id,
            entry_type: balancing.entry_type,
            amount: balancing.amount,
            currency: balancing.currency,
            effective_at: balancing.effective_at,
            metadata: balancing.metadata,
            order_id: balancing.order_id,
            search_expectation: SearchExpectation {
                rule_id: setup.config.transaction_rule_id.clone(),
                source_value: "expected".to_owned(),
                target_key: "order_id".parse().unwrap(),
            },
        }));
    for entry in create_entries(&client, &token, &request, false).await {
        if entry.status == EntryStatus::Expected {
            for request in [
                client
                    .post(&format!("/linking_processor/{}/process", entry.entry_id))
                    .with_admin_key(),
                client
                    .post(&format!("/linking_processor/{}/enqueue", entry.entry_id))
                    .with_auth(&token),
            ] {
                let response = request.send().await;
                assert_api_error(response, 400, "LP_802").await;
            }
            assert!(linked_entries(&client, &token, &entry.entry_id)
                .await
                .is_empty());
        } else {
            assert_eq!(entry.status, EntryStatus::Pending);
            let response = client
                .post(&format!("/linking_processor/{}/enqueue", entry.entry_id))
                .with_auth(&token)
                .send()
                .await;
            assert_api_error(response, 400, "LP_802").await;
            process_success(&client, &token, &entry).await;
        }
    }
}
