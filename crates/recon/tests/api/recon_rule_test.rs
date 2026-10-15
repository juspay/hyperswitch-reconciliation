use crate::{
    api::{
        account_test::create_test_account,
        recon_engine_test::types::{ReconRuleCustomConfig, ReconRuleEngine, ReconRuleEngineExt},
        transformation_test::create_transformation_config,
    },
    common::{TestApp, TestClient},
};
use api_models::recon_rule::{
    aging_config::AgingConfigApiRequest, OneToOneManySingle as OneToOneManySingleApi,
    OneToOneManySingleSource as OneToOneManySingleSourceApi,
    OneToOneManySingleTarget as OneToOneManySingleTargetApi,
};
use api_models::{
    recon_engine::ReconTriggerResponse,
    recon_rule::{
        GetReconRuleResponse, OneToManySingleSingle as OneToManySingleSingleApi,
        OneToManySingleSingleSource as OneToManySingleSingleSourceApi,
        OneToManySingleSingleTarget as OneToManySingleSingleTargetApi,
        OneToManySingleSingleTargetType as OneToManySingleSingleTargetTypeApi,
        OneToManyStrategy as OneToManyStrategyApi, OneToOneManyMany as OneToOneManyManyApi,
        OneToOneManyManySource as OneToOneManyManySourceApi,
        OneToOneManyManyTarget as OneToOneManyManyTargetApi,
        OneToOneSingleMany as OneToOneSingleManyApi,
        OneToOneSingleManySource as OneToOneSingleManySourceApi,
        OneToOneSingleManyTarget as OneToOneSingleManyTargetApi,
        OneToOneSingleSingle as OneToOneSingleSingleApi,
        OneToOneSingleSingleSource as OneToOneSingleSingleSourceApi,
        OneToOneSingleSingleTarget as OneToOneSingleSingleTargetApi,
        OneToOneStrategy as OneToOneStrategyApi, ReconRuleCreateRequest, ReconRuleUpdateRequest,
        ReconStrategy as ReconStrategyApi,
    },
    staging_entry::{GetStagingEntryResponse, StagingEntryCreateRequest},
    AccountCreateRequest, GetAccountResponse,
};
use common_enums::{BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::{
    recon_rule::{
        entry_field::EntryField,
        grouping_field::{GroupingField, GroupingFieldV1, SingleGroupingField},
        match_rules::{MatchOperator, MatchRuleV1, MatchRules, MatchRulesV1},
        search_identifier::{
            SearchIdentifier, SearchIdentifierKey, SearchIdentifierV2, SingleSearchIdentifier,
        },
        strategies::one_to_many_strategy::PercentageSplitValue,
        trigger::{
            archived::TriggerV1, BasicComparisonOperator, Trigger, TriggerCondition, TriggerLogic,
            TriggerOperator, TriggerOperatorV1, TriggerV2,
        },
    },
    units::FloatMajorUnit,
};
use masking::Secret;
use pretty_assertions::assert_eq;
use rstest::rstest;

fn create_recon_rule_request(
    rule_type: ReconRuleEngine,
    rule_name: Option<String>,
    rule_description: Option<String>,
    priority: Option<u8>,
    rule_custom_config: ReconRuleCustomConfig,
    source_account_ids: Vec<String>,
    target_account_ids: Vec<String>,
) -> ReconRuleCreateRequest {
    let strategy = match rule_type {
        ReconRuleEngine::OneToOneSingleSingleV1(_) => {
            // OneToOne SingleSingle requires exactly one source and one target
            let source_account_id = match source_account_ids.as_slice() {
                [source_account_id] => source_account_id,
                _ => panic!(
                    "OneToOneSingleSingle requires exactly 1 source account, got {}",
                    source_account_ids.len()
                ),
            };

            let target_account_id = match target_account_ids.as_slice() {
                [target_account_id] => target_account_id,
                _ => panic!(
                    "OneToOneSingleSingle requires exactly 1 target account, got {}",
                    target_account_ids.len()
                ),
            };

            ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: rule_custom_config
                        .trigger
                        .get(source_account_id)
                        .unwrap_or(&valid_trigger())
                        .clone(),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id.clone(),
                    tolerance_config: rule_custom_config
                        .tolerance_config
                        .get(target_account_id)
                        .unwrap_or(&None)
                        .clone(),
                },
                match_rules: rule_custom_config
                    .match_rules
                    .get(target_account_id)
                    .unwrap_or(&valid_match_rules())
                    .clone(),
                search_identifier: rule_custom_config
                    .search_identifier
                    .get(target_account_id)
                    .unwrap_or(&valid_search_identifier())
                    .clone(),
            }))
        }
        ReconRuleEngine::OneToOneSingleManyV1(_) => {
            // OneToOne SingleMany also requires exactly one source and one target
            let source_account_id = match source_account_ids.as_slice() {
                [source_account_id] => source_account_id,
                _ => panic!(
                    "OneToOneSingleMany requires exactly 1 source account, got {}",
                    source_account_ids.len()
                ),
            };

            let target_account_id = match target_account_ids.as_slice() {
                [target_account_id] => target_account_id,
                _ => panic!(
                    "OneToOneSingleMany requires exactly 1 target account, got {}",
                    target_account_ids.len()
                ),
            };

            ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleMany(OneToOneSingleManyApi {
                source_account: OneToOneSingleManySourceApi {
                    account_id: source_account_id.clone(),
                    trigger: rule_custom_config
                        .trigger
                        .get(source_account_id)
                        .unwrap_or(&valid_trigger())
                        .clone(),
                },
                target_account: OneToOneSingleManyTargetApi {
                    account_id: target_account_id.clone(),
                },
                match_rules: rule_custom_config
                    .match_rules
                    .get(target_account_id)
                    .unwrap_or(&valid_match_rules())
                    .clone(),
                search_identifier: rule_custom_config
                    .search_identifier
                    .get(target_account_id)
                    .unwrap_or(&valid_search_identifier())
                    .clone(),
            }))
        }
        ReconRuleEngine::OneToOneManySingleV1(_) => {
            let source_account_id = match source_account_ids.as_slice() {
                [source_account_id] => source_account_id,
                _ => panic!(
                    "OneToOneManySingle requires exactly 1 source account, got {}",
                    source_account_ids.len()
                ),
            };

            let target_account_id = match target_account_ids.as_slice() {
                [target_account_id] => target_account_id,
                _ => panic!(
                    "OneToOneManySingle requires exactly 1 target account, got {}",
                    target_account_ids.len()
                ),
            };

            ReconStrategyApi::OneToOne(OneToOneStrategyApi::ManySingle(OneToOneManySingleApi {
                source_account: OneToOneManySingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: rule_custom_config
                        .trigger
                        .get(source_account_id)
                        .unwrap_or(&valid_currency_trigger())
                        .clone(),
                    grouping_field: rule_custom_config
                        .grouping_field
                        .get(source_account_id)
                        .unwrap_or(&valid_batch_id_grouping_field())
                        .clone(),
                },
                target_account: OneToOneManySingleTargetApi {
                    account_id: target_account_id.to_string(),
                },
                match_rules: rule_custom_config
                    .match_rules
                    .get(target_account_id)
                    .unwrap_or(&MatchRules::V1(MatchRulesV1 { rules: vec![] }))
                    .clone(),
                search_identifier: rule_custom_config
                    .search_identifier
                    .get(target_account_id)
                    .unwrap_or(&SearchIdentifier::V2(SearchIdentifierV2::Single(
                        SingleSearchIdentifier {
                            key: SearchIdentifierKey {
                                source_field: "metadata.batch_id".parse().unwrap(), // search identifier will be grouping field
                                target_field: "metadata.transaction_id".parse().unwrap(),
                            },
                        },
                    )))
                    .clone(),
                source_accumulation_mode: rule_custom_config.source_accumulation_mode,
            }))
        }
        ReconRuleEngine::OneToOneManyManyV1(_) => {
            let source_account_id = match source_account_ids.as_slice() {
                [source_account_id] => source_account_id,
                _ => panic!(
                    "OneToOneManyMany requires exactly 1 source account, got {}",
                    source_account_ids.len()
                ),
            };

            let target_account_id = match target_account_ids.as_slice() {
                [target_account_id] => target_account_id,
                _ => panic!(
                    "OneToOneManyMany requires exactly 1 target account, got {}",
                    target_account_ids.len()
                ),
            };

            ReconStrategyApi::OneToOne(OneToOneStrategyApi::ManyMany(OneToOneManyManyApi {
                source_account: OneToOneManyManySourceApi {
                    account_id: source_account_id.clone(),
                    trigger: rule_custom_config
                        .trigger
                        .get(source_account_id)
                        .unwrap_or(&valid_currency_trigger())
                        .clone(),
                    grouping_field: rule_custom_config
                        .grouping_field
                        .get(source_account_id)
                        .unwrap_or(&valid_batch_id_grouping_field())
                        .clone(),
                },
                target_account: OneToOneManyManyTargetApi {
                    account_id: target_account_id.to_string(),
                },
                match_rules: rule_custom_config
                    .match_rules
                    .get(target_account_id)
                    .unwrap_or(&MatchRules::V1(MatchRulesV1 { rules: vec![] }))
                    .clone(),
                search_identifier: rule_custom_config
                    .search_identifier
                    .get(target_account_id)
                    .unwrap_or(&SearchIdentifier::V2(SearchIdentifierV2::Single(
                        SingleSearchIdentifier {
                            key: SearchIdentifierKey {
                                source_field: "metadata.batch_id".parse().unwrap(), // search identifier will be grouping field
                                target_field: "metadata.reference_id".parse().unwrap(),
                            },
                        },
                    )))
                    .clone(),
            }))
        }
        ReconRuleEngine::OneToManySingleSingleV1((_, ref target_account_configs)) => {
            // OneToMany SingleSingle requires exactly one source and one target
            let source_account_id = match source_account_ids.as_slice() {
                [source_account_id] => source_account_id,
                _ => panic!(
                    "OneToManySingleSingle requires exactly 1 source account, got {}",
                    source_account_ids.len()
                ),
            };
            assert_ne!(
                target_account_ids.len(),
                0,
                "OneToManySingleSingle requires at least 1 target account, got {}",
                target_account_ids.len()
            );

            ReconStrategyApi::OneToMany(OneToManyStrategyApi::SingleSingle(
                OneToManySingleSingleApi {
                    source_account: OneToManySingleSingleSourceApi {
                        account_id: source_account_id.clone(),
                        trigger: rule_custom_config
                            .trigger
                            .get(source_account_id)
                            .unwrap_or(&valid_trigger())
                            .clone(),
                    },
                    target_accounts: OneToManySingleSingleTargetTypeApi::Percentage {
                        targets: target_account_ids
                            .iter()
                            .enumerate()
                            .map(|(index, target_account_id)| {
                                (
                                    OneToManySingleSingleTargetApi {
                                        account_id: target_account_id.clone(),
                                        match_rules: rule_custom_config
                                            .match_rules
                                            .get(target_account_id)
                                            .unwrap_or(&valid_match_rules())
                                            .clone(),
                                        search_identifier: rule_custom_config
                                            .search_identifier
                                            .get(target_account_id)
                                            .unwrap_or(&valid_search_identifier())
                                            .clone(),
                                    },
                                    PercentageSplitValue {
                                        value: target_account_configs.get(index).unwrap().1,
                                    },
                                )
                            })
                            .collect(),
                    },
                },
            ))
        }
    };

    ReconRuleCreateRequest {
        rule_name: rule_name.unwrap_or_else(|| format!("Valid {:?} Rule", rule_type)),
        rule_description: rule_description.unwrap_or("Valid recon rule description".to_string()),
        priority: priority.unwrap_or(1),
        aging_config: AgingConfigApiRequest::NoAging,
        strategy,
    }
}

pub async fn create_test_recon_rule(
    client: &TestClient,
    token: &str,
    rule_type: ReconRuleEngine,
    rule_name: Option<String>,
    rule_description: Option<String>,
    priority: Option<u8>,
    rule_custom_config: ReconRuleCustomConfig,
    source_account_ids: Vec<String>,
    target_account_ids: Vec<String>,
) -> GetReconRuleResponse {
    let request = create_recon_rule_request(
        rule_type,
        rule_name,
        rule_description,
        priority,
        rule_custom_config,
        source_account_ids,
        target_account_ids,
    );
    let response = client
        .post("/recon_rules/v2")
        .with_auth(token)
        .json(&request)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let body: GetReconRuleResponse = response.json().await.unwrap();
    body
}

fn valid_trigger() -> Trigger {
    Trigger::V1(TriggerV1 {
        field: "metadata.transaction_id".parse().unwrap(),
        operator: TriggerOperator::V1(TriggerOperatorV1 {
            value: BasicComparisonOperator::Equals,
        }),
        value: "test_value".to_string(),
    })
}

fn valid_trigger_v2_all() -> Trigger {
    Trigger::V2(TriggerV2 {
        logic: TriggerLogic::All,
        conditions: vec![
            TriggerCondition {
                field: "metadata.transaction_id".parse().unwrap(),
                operator: TriggerOperator::V1(TriggerOperatorV1 {
                    value: BasicComparisonOperator::Equals,
                }),
                value: "test_value".to_string(),
            },
            TriggerCondition {
                field: "currency".parse().unwrap(),
                operator: TriggerOperator::V1(TriggerOperatorV1 {
                    value: BasicComparisonOperator::Equals,
                }),
                value: "USD".to_string(),
            },
        ],
    })
}

fn valid_trigger_v2_any() -> Trigger {
    Trigger::V2(TriggerV2 {
        logic: TriggerLogic::Any,
        conditions: vec![
            TriggerCondition {
                field: "metadata.channel".parse().unwrap(),
                operator: TriggerOperator::V1(TriggerOperatorV1 {
                    value: BasicComparisonOperator::Equals,
                }),
                value: "card".to_string(),
            },
            TriggerCondition {
                field: "currency".parse().unwrap(),
                operator: TriggerOperator::V1(TriggerOperatorV1 {
                    value: BasicComparisonOperator::Equals,
                }),
                value: "USD".to_string(),
            },
        ],
    })
}

fn valid_match_rules() -> MatchRules {
    MatchRules::V1(MatchRulesV1 {
        rules: vec![MatchRuleV1 {
            source_field: "metadata.match_rules_field_1".parse().unwrap(),
            target_field: "metadata.match_rules_field_2".parse().unwrap(),
            operator: MatchOperator::Equals,
            label: None,
        }],
    })
}

fn valid_search_identifier() -> SearchIdentifier {
    SearchIdentifier::V2(SearchIdentifierV2::Single(SingleSearchIdentifier {
        key: SearchIdentifierKey {
            source_field: "order_id".parse().unwrap(),
            target_field: "metadata.reference_id".parse().unwrap(),
        },
    }))
}

fn valid_currency_trigger() -> Trigger {
    Trigger::V1(TriggerV1 {
        field: "currency".parse().unwrap(),
        operator: TriggerOperator::V1(TriggerOperatorV1 {
            value: BasicComparisonOperator::Equals,
        }),
        value: "USD".to_string(),
    })
}

fn valid_batch_id_grouping_field() -> GroupingField {
    GroupingField::V1(GroupingFieldV1::Single(SingleGroupingField {
        field: EntryField::Metadata("batch_id".to_string()),
    }))
}

#[actix_web::test]
async fn create_recon_rule_v2_accepts_multi_condition_trigger() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    let request = ReconRuleCreateRequest {
        rule_name: "Composite Trigger Rule".to_string(),
        rule_description: "Uses Trigger::V2".to_string(),
        priority: 10,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id,
                    trigger: valid_trigger_v2_all(),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id,
                    tolerance_config: None,
                },
                match_rules: valid_match_rules(),
                search_identifier: valid_search_identifier(),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let body: GetReconRuleResponse = response.json().await.unwrap();
    assert!(matches!(
        body.strategy,
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(OneToOneSingleSingleApi {
            source_account: OneToOneSingleSingleSourceApi {
                trigger: Trigger::V2(TriggerV2 {
                    logic: TriggerLogic::All,
                    ..
                }),
                ..
            },
            ..
        }))
    ));
}

#[actix_web::test]
async fn create_recon_rule_v2_rejects_empty_composite_trigger_conditions() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    let request = ReconRuleCreateRequest {
        rule_name: "Invalid Composite Trigger Rule".to_string(),
        rule_description: "Should fail validation".to_string(),
        priority: 10,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id,
                    trigger: Trigger::V2(TriggerV2 {
                        logic: TriggerLogic::All,
                        conditions: vec![],
                    }),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id,
                    tolerance_config: None,
                },
                match_rules: valid_match_rules(),
                search_identifier: valid_search_identifier(),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 400);
}

#[actix_web::test]
async fn process_transaction_uses_v2_all_trigger_when_all_conditions_match() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "OMS", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "PSP", BalanceDirection::Credit).await;

    let rule_request = ReconRuleCreateRequest {
        rule_name: "Composite ALL Rule".to_string(),
        rule_description: "Matches only when both conditions are true".to_string(),
        priority: 10,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: valid_trigger_v2_all(),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id.clone(),
                    tolerance_config: None,
                },
                match_rules: valid_match_rules(),
                search_identifier: valid_search_identifier(),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&rule_request)
        .send()
        .await;
    assert_eq!(response.status(), 201);

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

    let staging_entry_request = StagingEntryCreateRequest {
        account_id: source_account_id,
        entry_type: BalanceDirection::Debit,
        processing_mode: StagingEntryProcessingMode::Transaction,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(serde_json::json!({
            "transaction_id": "test_value",
            "match_rules_field_1": "match_value"
        })),
        transformation_id: transformation.transformation_id,
        transformation_history_id: None,
        order_id: "order_and".to_string(),
    };

    let staging_entry_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request)
        .send()
        .await;
    assert_eq!(staging_entry_response.status(), 201);
    let staging_entry: GetStagingEntryResponse = staging_entry_response.json().await.unwrap();

    let process_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(process_response.status(), 200);

    let recon_trigger_response: ReconTriggerResponse = process_response.json().await.unwrap();
    assert!(matches!(
        recon_trigger_response,
        ReconTriggerResponse::Success(_)
    ));
}

#[actix_web::test]
async fn process_transaction_uses_v2_any_trigger_when_any_condition_matches() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "OMS", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "PSP", BalanceDirection::Credit).await;

    let rule_request = ReconRuleCreateRequest {
        rule_name: "Composite ANY Rule".to_string(),
        rule_description: "Matches when any configured condition is true".to_string(),
        priority: 10,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: valid_trigger_v2_any(),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id.clone(),
                    tolerance_config: None,
                },
                match_rules: valid_match_rules(),
                search_identifier: valid_search_identifier(),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&rule_request)
        .send()
        .await;
    assert_eq!(response.status(), 201);

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

    let staging_entry_request = StagingEntryCreateRequest {
        account_id: source_account_id,
        entry_type: BalanceDirection::Debit,
        processing_mode: StagingEntryProcessingMode::Transaction,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(serde_json::json!({
            "transaction_id": "test_value",
            "match_rules_field_1": "match_value"
        })),
        transformation_id: transformation.transformation_id,
        transformation_history_id: None,
        order_id: "order_or".to_string(),
    };

    let staging_entry_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request)
        .send()
        .await;
    assert_eq!(staging_entry_response.status(), 201);
    let staging_entry: GetStagingEntryResponse = staging_entry_response.json().await.unwrap();

    let process_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(process_response.status(), 200);

    let recon_trigger_response: ReconTriggerResponse = process_response.json().await.unwrap();
    assert!(matches!(
        recon_trigger_response,
        ReconTriggerResponse::Success(_)
    ));
}

#[actix_web::test]
async fn process_transaction_prefers_highest_priority_among_v1_and_v2_matches() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "OMS", BalanceDirection::Debit).await;
    let low_priority_target_account_id = create_test_account(
        &client,
        &token,
        "Low Priority PSP",
        BalanceDirection::Credit,
    )
    .await;
    let high_priority_target_account_id = create_test_account(
        &client,
        &token,
        "High Priority PSP",
        BalanceDirection::Credit,
    )
    .await;

    let low_priority_request = ReconRuleCreateRequest {
        rule_name: "Low Priority V1 Rule".to_string(),
        rule_description: "Legacy rule".to_string(),
        priority: 5,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: valid_trigger(),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: low_priority_target_account_id,
                    tolerance_config: None,
                },
                match_rules: valid_match_rules(),
                search_identifier: valid_search_identifier(),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let high_priority_request = ReconRuleCreateRequest {
        rule_name: "High Priority V2 Rule".to_string(),
        rule_description: "Composite rule".to_string(),
        priority: 10,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: valid_trigger_v2_all(),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: high_priority_target_account_id,
                    tolerance_config: None,
                },
                match_rules: valid_match_rules(),
                search_identifier: valid_search_identifier(),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let low_priority_response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&low_priority_request)
        .send()
        .await;
    assert_eq!(low_priority_response.status(), 201);

    let high_priority_response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&high_priority_request)
        .send()
        .await;
    assert_eq!(high_priority_response.status(), 201);
    let high_priority_rule: GetReconRuleResponse = high_priority_response.json().await.unwrap();

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

    let staging_entry_request = StagingEntryCreateRequest {
        account_id: source_account_id,
        entry_type: BalanceDirection::Debit,
        processing_mode: StagingEntryProcessingMode::Transaction,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(serde_json::json!({
            "transaction_id": "test_value",
            "match_rules_field_1": "match_value"
        })),
        transformation_id: transformation.transformation_id,
        transformation_history_id: None,
        order_id: "order_priority".to_string(),
    };

    let staging_entry_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request)
        .send()
        .await;
    assert_eq!(staging_entry_response.status(), 201);
    let staging_entry: GetStagingEntryResponse = staging_entry_response.json().await.unwrap();

    let process_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(process_response.status(), 200);

    let recon_trigger_response: ReconTriggerResponse = process_response.json().await.unwrap();
    let transaction = match recon_trigger_response {
        ReconTriggerResponse::Success(transaction) => *transaction,
        ReconTriggerResponse::Failure(error) => panic!("Expected success, got failure: {error}"),
    };

    assert_eq!(transaction.rule_id, high_priority_rule.rule_id);
}

#[actix_web::test]
async fn create_recon_rule_valid_one_to_one_case() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));

    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;

    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    // Create the recon rule request
    let request = create_recon_rule_request(
        rule_type,
        Some("One To One Single Single Rule".to_string()),
        Some("One To One Single Single Rule description".to_string()),
        Some(100),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    );

    // Make the request
    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);

    let body: GetReconRuleResponse = response.json().await.unwrap();

    // Validate basic fields
    assert!(body.rule_id.starts_with("rule_"));
    assert_eq!(body.rule_name, "One To One Single Single Rule");
    assert_eq!(
        body.rule_description,
        "One To One Single Single Rule description"
    );
    assert_eq!(body.priority, 100);
    assert_eq!(body.is_active, true);
    assert_eq!(body.profile_id, profile_id);

    if let ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
        OneToOneSingleSingleApi {
            source_account:
                OneToOneSingleSingleSourceApi {
                    account_id: source_account_id,
                    trigger: trigger_response,
                },
            target_account:
                OneToOneSingleSingleTargetApi {
                    account_id: target_account_id,
                    tolerance_config: _,
                },
            match_rules: match_rules_response,
            search_identifier: search_identifier_response,
        },
    )) = body.strategy
    {
        assert_eq!(source_account_id, source_account_id);
        assert_eq!(target_account_id, target_account_id);
        if let Trigger::V1(TriggerV1 {
            field,
            operator:
                TriggerOperator::V1(TriggerOperatorV1 {
                    value: BasicComparisonOperator::Equals,
                }),
            value: trigger_value,
        }) = trigger_response
        {
            assert_eq!(field, "metadata.transaction_id".parse().unwrap());
            assert_eq!(trigger_value, "test_value");
        } else {
            panic!("Expected Trigger::V2 but got Trigger::V1");
        }

        match match_rules_response {
            MatchRules::V1(MatchRulesV1 { rules }) => {
                assert_eq!(rules.len(), 1);
                assert_eq!(
                    rules[0].source_field,
                    EntryField::Metadata("match_rules_field_1".to_string())
                );
                assert_eq!(
                    rules[0].target_field,
                    EntryField::Metadata("match_rules_field_2".to_string())
                );
            }
        }

        match search_identifier_response {
            SearchIdentifier::V1(_) => {
                panic!("Expected SearchIdentifier::V2, got V1");
            }
            SearchIdentifier::V2(SearchIdentifierV2::Single(SingleSearchIdentifier { key })) => {
                assert_eq!(key.source_field, "order_id".parse().unwrap());
                assert_eq!(key.target_field, "metadata.reference_id".parse().unwrap());
            }
            SearchIdentifier::V2(SearchIdentifierV2::Composite { .. }) => {
                panic!("Expected SearchIdentifierV2::Single, got Composite");
            }
        }
    } else {
        panic!("Wrong Recon Rule Type");
    }
}

#[rstest]
// Invalid cases - empty rule name
#[case("", "Valid description", 100, false, false, false, false, false, 400)]
// Invalid cases - empty rule description
#[case("Valid Rule", "", 100, false, false, false, false, false, 400)]
// Invalid cases - same source and target account (one-to-one specific)
#[case(
    "Valid Rule",
    "Valid description",
    100,
    true,
    false,
    false,
    false,
    false,
    400
)]
// Invalid cases - non-existent source account
#[case(
    "Valid Rule",
    "Valid description",
    100,
    false,
    true,
    false,
    false,
    false,
    404
)]
// Invalid cases - non-existent target account
#[case(
    "Valid Rule",
    "Valid description",
    100,
    false,
    false,
    true,
    false,
    false,
    404
)]
// Invalid cases - both accounts non-existent
#[case(
    "Valid Rule",
    "Valid description",
    100,
    false,
    false,
    false,
    true,
    false,
    404
)]
// Invalid cases - currency mismatch
#[case(
    "Valid Rule",
    "Valid description",
    100,
    false,
    false,
    false,
    false,
    true,
    400
)]
#[actix_web::test]
async fn create_recon_rule_one_to_one_invalid_cases(
    #[case] rule_name: &str,
    #[case] rule_description: &str,
    #[case] priority: u8,
    #[case] same_source_target: bool,
    #[case] non_existent_source: bool,
    #[case] non_existent_target: bool,
    #[case] both_non_existent: bool,
    #[case] currency_mismatch: bool,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let (source_account_id, target_account_id) = if both_non_existent {
        (
            "non_existent_source_id".to_string(),
            "non_existent_target_id".to_string(),
        )
    } else if non_existent_source {
        // Create a valid target account
        let target_account_id =
            create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;
        ("non_existent_source_id".to_string(), target_account_id)
    } else if non_existent_target {
        // Create a valid source account
        let source_account_id =
            create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
        (source_account_id, "non_existent_target_id".to_string())
    } else {
        // Create valid account
        let source_account_id =
            create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;

        if same_source_target {
            // Use the same account for both source and target
            (source_account_id.clone(), source_account_id)
        } else {
            // Create a different target account
            let account_request = AccountCreateRequest {
                account_name: "Target Account".to_string(),
                account_type: BalanceDirection::Credit,
                currency: match currency_mismatch {
                    true => Currency::EUR,
                    false => Currency::USD,
                },
                initial_balance: FloatMajorUnit::new(1000.0),
            };
            let response = client
                .post("/accounts")
                .with_auth(&token)
                .json(&account_request)
                .send()
                .await;

            assert_eq!(response.status(), 201);
            let target_account: GetAccountResponse = response.json().await.unwrap();
            (source_account_id, target_account.account_id)
        }
    };

    // Create the recon rule request
    let request = create_recon_rule_request(
        ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
        Some(rule_name.to_string()),
        Some(rule_description.to_string()),
        Some(priority),
        ReconRuleCustomConfig::default(),
        vec![source_account_id],
        vec![target_account_id],
    );

    // Make the request
    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);
}

#[actix_web::test]
async fn get_recon_rule_by_id_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap and create a rule
    let (_merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));

    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;

    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    let created_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(150),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    // Test get by ID
    let get_response = client
        .get(&format!("/recon_rules/v2/{}", created_rule.rule_id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(get_response.status(), 200);

    let retrieved_rule: GetReconRuleResponse = get_response.json().await.unwrap();
    assert_eq!(retrieved_rule.rule_id, created_rule.rule_id);
    assert_eq!(retrieved_rule.rule_name, "Test Rule");
    assert_eq!(retrieved_rule.profile_id, profile_id);
    assert_eq!(retrieved_rule.created_at, created_rule.created_at);
    assert_eq!(
        retrieved_rule.last_modified_at,
        created_rule.last_modified_at
    );
}

#[actix_web::test]
async fn get_recon_rule_by_id_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client
        .get("/recon_rules/v2/rule_nonexistent123")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 404);
}

#[actix_web::test]
async fn get_recon_rule_by_id_cross_profile_access() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Create two different profiles
    let (_merchant_id1, _profile_id1, token1) = client.bootstrap("Merchant 1", "Profile 1").await;
    let (_merchant_id2, _profile_id2, token2) = client.bootstrap("Merchant 2", "Profile 2").await;

    // Create accounts and rule for profile 1
    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));

    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token1).await;

    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    let created_rule = create_test_recon_rule(
        &client,
        &token1,
        rule_type,
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(150),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    // Try to access profile 1's rule using profile 2's token
    let get_response = client
        .get(&format!("/recon_rules/v2/{}", created_rule.rule_id))
        .with_auth(&token2)
        .send()
        .await;

    // Should return 404 for security reasons (rule exists but belongs to different profile)
    assert_eq!(get_response.status(), 404);
}

#[actix_web::test]
async fn list_recon_rules_by_profile_id_multiple_rules() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create accounts
    let mut account_ids = Vec::new();
    for i in 0..4 {
        account_ids.push(
            create_test_account(
                &client,
                &token,
                &format!("Account {i}"),
                BalanceDirection::Debit,
            )
            .await,
        );
    }

    let created_rule1 = create_test_recon_rule(
        &client,
        &token,
        ReconRuleEngine::OneToOneSingleSingleV1((
            "Source Account 1".to_string(),
            "Target Account 1".to_string(),
        )),
        Some("Test Rule 1".to_string()),
        Some("Test description 1".to_string()),
        Some(100),
        ReconRuleCustomConfig::default(),
        vec![account_ids[0].clone()],
        vec![account_ids[1].clone()],
    )
    .await;
    let created_rule2 = create_test_recon_rule(
        &client,
        &token,
        ReconRuleEngine::OneToOneSingleSingleV1((
            "Source Account 2".to_string(),
            "Target Account 2".to_string(),
        )),
        Some("Test Rule 2".to_string()),
        Some("Test description 2".to_string()),
        Some(200),
        ReconRuleCustomConfig::default(),
        vec![account_ids[2].clone()],
        vec![account_ids[3].clone()],
    )
    .await;

    // Test list endpoint
    let list_response = client.get("/recon_rules/v2").with_auth(&token).send().await;
    assert_eq!(list_response.status(), 200);

    let rules: Vec<GetReconRuleResponse> = list_response.json().await.unwrap();
    assert_eq!(rules.len(), 2);

    let rule_ids: Vec<String> = rules.iter().map(|r| r.rule_id.clone()).collect();
    assert!(rule_ids.contains(&created_rule1.rule_id));
    assert!(rule_ids.contains(&created_rule2.rule_id));

    // Verify profile isolation
    for rule in &rules {
        assert_eq!(rule.profile_id, profile_id);
    }
}

#[actix_web::test]
async fn get_recon_rule_v2_by_id_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap and create a rule
    let (_merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));

    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;

    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    let created_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(150),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    // Test get by ID
    let get_response = client
        .get(&format!("/recon_rules/v2/{}", created_rule.rule_id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(get_response.status(), 200);

    let retrieved_rule: GetReconRuleResponse = get_response.json().await.unwrap();

    // Validate strategy structure
    match &retrieved_rule.strategy {
        ReconStrategyApi::OneToOne(one_to_one) => {
            match one_to_one {
                api_models::recon_rule::OneToOneStrategy::SingleSingle(strategy) => {
                    // Validate source
                    assert_eq!(
                        strategy.source_account.account_id,
                        source_account_id.clone()
                    );
                    // Validate target
                    assert_eq!(
                        strategy.target_account.account_id,
                        target_account_id.clone()
                    );
                }
                _ => panic!("One To One Single Single was created"),
            }
        }
        ReconStrategyApi::OneToMany(_) => panic!("Expected type was One To One"),
    }
    assert_eq!(retrieved_rule.rule_id, created_rule.rule_id);
    assert_eq!(retrieved_rule.rule_name, "Test Rule");
    assert_eq!(retrieved_rule.profile_id, profile_id);
    assert_eq!(retrieved_rule.created_at, created_rule.created_at);
    assert_eq!(
        retrieved_rule.last_modified_at,
        created_rule.last_modified_at
    );
}

#[actix_web::test]
async fn update_recon_rule_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap and create a rule
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));

    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;

    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(150),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    let update_request = ReconRuleUpdateRequest {
        rule_name: Some("Updated Rule Name".to_string()),
        rule_description: Some("Updated Rule Description".to_string()),
        aging_config: None,
        priority: None,
        is_active: None,
        strategy: None,
    };

    let update_response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), 200);

    let updated_rule: GetReconRuleResponse = update_response.json().await.unwrap();
    assert_eq!(updated_rule.rule_id, recon_rule.rule_id);
    assert_eq!(updated_rule.rule_name, "Updated Rule Name");
    assert_eq!(updated_rule.rule_description, "Updated Rule Description");
}

#[actix_web::test]
async fn update_recon_rule_wrong_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap and create a rule
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;
    let (_merchant_id, _profile_id, token2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));

    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;

    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(150),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    let update_request = ReconRuleUpdateRequest {
        rule_name: Some("Updated Rule Name".to_string()),
        rule_description: Some("Updated Rule Description".to_string()),
        aging_config: None,
        priority: None,
        is_active: None,
        strategy: None,
    };

    let update_response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token2)
        .json(&update_request)
        .send()
        .await;

    // Rule with profile_id 2 does not exist
    assert_eq!(update_response.status(), 404);
}

#[actix_web::test]
async fn update_recon_rule_priority_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));
    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;
    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Priority Rule".to_string()),
        Some("Priority test".to_string()),
        Some(3),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;
    assert_eq!(recon_rule.priority, 3);

    let update_request = ReconRuleUpdateRequest {
        rule_name: None,
        rule_description: None,
        aging_config: None,
        priority: Some(7),
        is_active: None,
        strategy: None,
    };

    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let updated_rule: GetReconRuleResponse = response.json().await.unwrap();
    assert_eq!(updated_rule.priority, 7);
    assert_eq!(updated_rule.rule_name, "Priority Rule");
    assert_eq!(updated_rule.is_active, true);
}

#[actix_web::test]
async fn update_recon_rule_is_active_toggle() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));
    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;
    let source_account_id = &source_accounts.first().unwrap().account_id;
    let target_account_id = &target_accounts.first().unwrap().account_id;

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Toggle Rule".to_string()),
        Some("Toggle".to_string()),
        Some(1),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;
    assert_eq!(recon_rule.is_active, true);

    let deactivate = ReconRuleUpdateRequest {
        rule_name: None,
        rule_description: None,
        aging_config: None,
        priority: None,
        is_active: Some(false),
        strategy: None,
    };
    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&deactivate)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let updated: GetReconRuleResponse = response.json().await.unwrap();
    assert_eq!(updated.is_active, false);

    let reactivate = ReconRuleUpdateRequest {
        rule_name: None,
        rule_description: None,
        aging_config: None,
        priority: None,
        is_active: Some(true),
        strategy: None,
    };
    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&reactivate)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let updated: GetReconRuleResponse = response.json().await.unwrap();
    assert_eq!(updated.is_active, true);
}

#[actix_web::test]
async fn update_recon_rule_strategy_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));
    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;
    let source_account_id = source_accounts.first().unwrap().account_id.clone();
    let target_account_id = target_accounts.first().unwrap().account_id.clone();

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Strategy Rule".to_string()),
        Some("Strategy update test".to_string()),
        Some(1),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    let new_strategy =
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(OneToOneSingleSingleApi {
            source_account: OneToOneSingleSingleSourceApi {
                account_id: source_account_id,
                trigger: valid_trigger_v2_all(),
            },
            target_account: OneToOneSingleSingleTargetApi {
                account_id: target_account_id,
                tolerance_config: None,
            },
            match_rules: valid_match_rules(),
            search_identifier: valid_search_identifier(),
        }));
    let update_request = ReconRuleUpdateRequest {
        rule_name: None,
        rule_description: None,
        aging_config: None,
        priority: None,
        is_active: None,
        strategy: Some(new_strategy),
    };

    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let updated: GetReconRuleResponse = response.json().await.unwrap();
    match updated.strategy {
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(s)) => {
            assert!(
                matches!(s.source_account.trigger, Trigger::V2(_)),
                "Expected Trigger::V2 after strategy update, got: {:?}",
                s.source_account.trigger
            );
        }
        other => panic!("Expected OneToOne::SingleSingle, got: {:?}", other),
    }
}

#[actix_web::test]
async fn update_recon_rule_strategy_one_to_many_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToManySingleSingleV1((
        "OneToMany Source".to_string(),
        vec![
            ("Target A".to_string(), 60.0),
            ("Target B".to_string(), 40.0),
        ],
    ));
    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;
    let source_account_id = source_accounts.first().unwrap().account_id.clone();
    let target_account_ids: Vec<String> = target_accounts
        .iter()
        .map(|a| a.account_id.clone())
        .collect();

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("OneToMany Strategy".to_string()),
        Some("OneToMany strategy update test".to_string()),
        Some(1),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        target_account_ids.clone(),
    )
    .await;

    let new_strategy = ReconStrategyApi::OneToMany(OneToManyStrategyApi::SingleSingle(
        OneToManySingleSingleApi {
            source_account: OneToManySingleSingleSourceApi {
                account_id: source_account_id,
                trigger: valid_trigger_v2_all(),
            },
            target_accounts: OneToManySingleSingleTargetTypeApi::Percentage {
                targets: target_account_ids
                    .iter()
                    .enumerate()
                    .map(|(index, account_id)| {
                        (
                            OneToManySingleSingleTargetApi {
                                account_id: account_id.clone(),
                                match_rules: valid_match_rules(),
                                search_identifier: valid_search_identifier(),
                            },
                            PercentageSplitValue {
                                value: if index == 0 { 60.0 } else { 40.0 },
                            },
                        )
                    })
                    .collect(),
            },
        },
    ));
    let update_request = ReconRuleUpdateRequest {
        rule_name: None,
        rule_description: None,
        aging_config: None,
        priority: None,
        is_active: None,
        strategy: Some(new_strategy),
    };

    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let updated: GetReconRuleResponse = response.json().await.unwrap();
    match updated.strategy {
        ReconStrategyApi::OneToMany(OneToManyStrategyApi::SingleSingle(s)) => {
            assert!(
                matches!(s.source_account.trigger, Trigger::V2(_)),
                "Expected Trigger::V2 after strategy update"
            );
        }
        other => panic!("Expected OneToMany::SingleSingle, got: {:?}", other),
    }
}

#[actix_web::test]
async fn update_recon_rule_strategy_currency_mismatch() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));
    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;
    let source_account_id = source_accounts.first().unwrap().account_id.clone();
    let target_account_id = target_accounts.first().unwrap().account_id.clone();

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Strategy Rule".to_string()),
        Some("Strategy validation test".to_string()),
        Some(1),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    // Accounts created above are USD, so an EUR target breaks the shared-currency rule
    let eur_target_request = AccountCreateRequest {
        account_name: "Target EUR".to_string(),
        account_type: BalanceDirection::Credit,
        currency: Currency::EUR,
        initial_balance: FloatMajorUnit::new(1000.0),
    };
    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&eur_target_request)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let eur_target_account: GetAccountResponse = response.json().await.unwrap();

    let new_strategy =
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(OneToOneSingleSingleApi {
            source_account: OneToOneSingleSingleSourceApi {
                account_id: source_account_id,
                trigger: valid_trigger_v2_all(),
            },
            target_account: OneToOneSingleSingleTargetApi {
                account_id: eur_target_account.account_id,
                tolerance_config: None,
            },
            match_rules: valid_match_rules(),
            search_identifier: valid_search_identifier(),
        }));
    let update_request = ReconRuleUpdateRequest {
        rule_name: None,
        rule_description: None,
        aging_config: None,
        priority: None,
        is_active: None,
        strategy: Some(new_strategy),
    };

    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;
    assert_eq!(response.status(), 400);

    let response = client
        .get(&format!("/recon_rules/v2/{}", recon_rule.rule_id))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let unchanged: GetReconRuleResponse = response.json().await.unwrap();
    match unchanged.strategy {
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(s)) => {
            assert_eq!(s.target_account.account_id, target_account_id);
        }
        other => panic!("Expected OneToOne::SingleSingle, got: {:?}", other),
    }
}

#[actix_web::test]
async fn update_recon_rule_combined_fields() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));
    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;
    let source_account_id = source_accounts.first().unwrap().account_id.clone();
    let target_account_id = target_accounts.first().unwrap().account_id.clone();

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Original Name".to_string()),
        Some("Original description".to_string()),
        Some(1),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    let new_strategy =
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(OneToOneSingleSingleApi {
            source_account: OneToOneSingleSingleSourceApi {
                account_id: source_account_id,
                trigger: valid_trigger_v2_all(),
            },
            target_account: OneToOneSingleSingleTargetApi {
                account_id: target_account_id,
                tolerance_config: None,
            },
            match_rules: valid_match_rules(),
            search_identifier: valid_search_identifier(),
        }));
    let update_request = ReconRuleUpdateRequest {
        rule_name: Some("Renamed Rule".to_string()),
        rule_description: Some("Renamed description".to_string()),
        aging_config: None,
        priority: Some(9),
        is_active: Some(false),
        strategy: Some(new_strategy),
    };

    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let updated: GetReconRuleResponse = response.json().await.unwrap();
    assert_eq!(updated.rule_name, "Renamed Rule");
    assert_eq!(updated.rule_description, "Renamed description");
    assert_eq!(updated.priority, 9);
    assert_eq!(updated.is_active, false);
    match updated.strategy {
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(s)) => {
            assert!(
                matches!(s.source_account.trigger, Trigger::V2(_)),
                "Expected Trigger::V2 after combined update"
            );
        }
        other => panic!("Expected OneToOne::SingleSingle, got: {:?}", other),
    }
}

#[actix_web::test]
async fn update_recon_rule_priority_wrong_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;
    let (_merchant_id, _profile_id, token2) = client
        .bootstrap("Test Merchant Other", "Test Profile Other")
        .await;

    let rule_type = ReconRuleEngine::OneToOneSingleSingleV1((
        "Source Account".to_string(),
        "Target Account".to_string(),
    ));
    let (source_accounts, target_accounts) = rule_type.create_accounts(&client, &token).await;
    let source_account_id = source_accounts.first().unwrap().account_id.clone();
    let target_account_id = target_accounts.first().unwrap().account_id.clone();

    let recon_rule = create_test_recon_rule(
        &client,
        &token,
        rule_type,
        Some("Cross-profile".to_string()),
        Some("Should be 404 for other profile".to_string()),
        Some(1),
        ReconRuleCustomConfig::default(),
        vec![source_account_id],
        vec![target_account_id],
    )
    .await;

    let update_request = ReconRuleUpdateRequest {
        rule_name: None,
        rule_description: None,
        aging_config: None,
        priority: Some(5),
        is_active: None,
        strategy: None,
    };

    let response = client
        .put(&format!("/recon_rules/{}", recon_rule.rule_id))
        .with_auth(&token2)
        .json(&update_request)
        .send()
        .await;
    assert_eq!(response.status(), 404);
}

#[actix_web::test]
async fn create_recon_rule_currency_mismatch() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create source account (USD)
    let source_account_req = AccountCreateRequest {
        account_name: "Source USD".to_string(),
        account_type: BalanceDirection::Debit,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(1000.0),
    };
    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&source_account_req)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let source_account: GetAccountResponse = response.json().await.unwrap();

    // Create target account (EUR)
    let target_account_req = AccountCreateRequest {
        account_name: "Target EUR".to_string(),
        account_type: BalanceDirection::Credit,
        currency: Currency::EUR,
        initial_balance: FloatMajorUnit::new(1000.0),
    };
    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&target_account_req)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let target_account: GetAccountResponse = response.json().await.unwrap();

    // Try to create recon rule
    let rule_request = create_recon_rule_request(
        ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(150),
        ReconRuleCustomConfig::default(),
        vec![source_account.account_id],
        vec![target_account.account_id],
    );

    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&rule_request)
        .send()
        .await;

    assert_eq!(response.status(), 400);
}

// ==================== V2 API Tests ====================

fn valid_one_to_one_recon_rule_request_v2(
    rule_name: &str,
    rule_description: &str,
    priority: u8,
    source_account_id: String,
    target_account_id: String,
) -> ReconRuleCreateRequest {
    let strategy =
        ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(OneToOneSingleSingleApi {
            search_identifier: valid_search_identifier(),
            match_rules: valid_match_rules(),
            source_account: OneToOneSingleSingleSourceApi {
                account_id: source_account_id,
                trigger: valid_trigger(),
            },
            target_account: OneToOneSingleSingleTargetApi {
                account_id: target_account_id,
                tolerance_config: None,
            },
        }));

    ReconRuleCreateRequest {
        rule_name: rule_name.to_string(),
        rule_description: rule_description.to_string(),
        priority,
        aging_config: AgingConfigApiRequest::NoAging,
        strategy,
    }
}

#[actix_web::test]
async fn create_recon_rule_v2_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;

    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    // Create the recon rule request using v2 API
    let request = valid_one_to_one_recon_rule_request_v2(
        "V2 Test Rule",
        "V2 test rule description",
        100,
        source_account_id.clone(),
        target_account_id.clone(),
    );

    // Make the request to v2 endpoint
    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);

    let body: GetReconRuleResponse = response.json().await.unwrap();

    // Validate basic fields
    assert!(body.rule_id.starts_with("rule_"));
    assert_eq!(body.rule_name, "V2 Test Rule");
    assert_eq!(body.rule_description, "V2 test rule description");
    assert_eq!(body.priority, 100);
    assert_eq!(body.is_active, true);
    assert_eq!(body.profile_id, profile_id);

    // Validate strategy structure
    match &body.strategy {
        ReconStrategyApi::OneToOne(one_to_one) => match one_to_one {
            OneToOneStrategyApi::SingleSingle(strategy) => {
                // Validate source
                assert_eq!(strategy.source_account.account_id, source_account_id);
                match &strategy.source_account.trigger {
                    Trigger::V1(field_trigger) => {
                        assert_eq!(
                            field_trigger.field,
                            "metadata.transaction_id".parse().unwrap()
                        );
                        assert_eq!(field_trigger.value, "test_value");
                    }
                    Trigger::V2(_) => panic!("Expected Trigger::V1 in this legacy round-trip test"),
                }
                // Validate target
                assert_eq!(strategy.target_account.account_id, target_account_id);
                // Validate match rules
                match &strategy.match_rules {
                    MatchRules::V1(match_rules_v1) => {
                        assert_eq!(match_rules_v1.rules.len(), 1);
                        assert_eq!(
                            match_rules_v1.rules[0].source_field,
                            "metadata.match_rules_field_1".parse().unwrap()
                        );
                        assert_eq!(
                            match_rules_v1.rules[0].target_field,
                            "metadata.match_rules_field_2".parse().unwrap()
                        );
                    }
                }
                // Validate search identifier
                match &strategy.search_identifier {
                    SearchIdentifier::V1(_) => {
                        panic!("Expected SearchIdentifier::V2, got V1");
                    }
                    SearchIdentifier::V2(SearchIdentifierV2::Single(SingleSearchIdentifier {
                        key,
                    })) => {
                        assert_eq!(key.source_field, "order_id".parse().unwrap());
                        assert_eq!(key.target_field, "metadata.reference_id".parse().unwrap());
                    }
                    SearchIdentifier::V2(SearchIdentifierV2::Composite { .. }) => {
                        panic!("Expected SearchIdentifierV2::Single, got Composite");
                    }
                }
            }
            _ => panic!("Unexpected strategy type"),
        },
        ReconStrategyApi::OneToMany(_) => panic!("Unexpected strategy type"),
    }
}

#[actix_web::test]
async fn create_recon_rule_v2_currency_mismatch() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create source account (USD)
    let source_account_req = AccountCreateRequest {
        account_name: "Source USD".to_string(),
        account_type: BalanceDirection::Debit,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(1000.0),
    };
    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&source_account_req)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let source_account: GetAccountResponse = response.json().await.unwrap();

    // Create target account (EUR)
    let target_account_req = AccountCreateRequest {
        account_name: "Target EUR".to_string(),
        account_type: BalanceDirection::Credit,
        currency: Currency::EUR,
        initial_balance: FloatMajorUnit::new(1000.0),
    };
    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&target_account_req)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let target_account: GetAccountResponse = response.json().await.unwrap();

    // Try to create recon rule
    let rule_request = create_recon_rule_request(
        ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
        Some("Test Rule".to_string()),
        Some("Test description".to_string()),
        Some(150),
        ReconRuleCustomConfig::default(),
        vec![source_account.account_id],
        vec![target_account.account_id],
    );

    let response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&rule_request)
        .send()
        .await;

    assert_eq!(response.status(), 400);
}

#[actix_web::test]
async fn multiple_rules_resilient_processing_one_rule_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create source and target accounts
    let source_account_id =
        create_test_account(&client, &token, "Source Account", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    // Create Rule 1 with priority 1 - This will have an invalid search identifier
    // Using a non-existent field that won't be in the staging entry
    let invalid_trigger = Trigger::V1(TriggerV1 {
        field: "metadata.non_existent_field".parse().unwrap(),
        operator: TriggerOperator::V1(TriggerOperatorV1 {
            value: BasicComparisonOperator::Equals,
        }),
        value: "some_value".to_string(),
    });

    let invalid_search_identifier =
        SearchIdentifier::V2(SearchIdentifierV2::Single(SingleSearchIdentifier {
            key: SearchIdentifierKey {
                source_field: "metadata.non_existent_field".parse().unwrap(),
                target_field: "metadata.non_existent_field".parse().unwrap(),
            },
        }));

    let invalid_rule_request = ReconRuleCreateRequest {
        rule_name: "Invalid Rule - Priority 1".to_string(),
        rule_description: "Rule with non-existent field".to_string(),
        priority: 1,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: invalid_trigger,
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id.clone(),
                    tolerance_config: None,
                },
                match_rules: valid_match_rules(),
                search_identifier: invalid_search_identifier,
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let invalid_rule_response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&invalid_rule_request)
        .send()
        .await;

    assert_eq!(invalid_rule_response.status(), 201);
    let invalid_rule: GetReconRuleResponse = invalid_rule_response.json().await.unwrap();

    // Create Rule 2 with priority 2 - This will be valid
    let valid_rule_response = create_test_recon_rule(
        &client,
        &token,
        ReconRuleEngine::OneToOneSingleSingleV1((
            "Source Account".to_string(),
            "Target Account".to_string(),
        )),
        Some("Valid Rule - Priority 2".to_string()),
        Some("Valid rule that should work".to_string()),
        Some(2),
        ReconRuleCustomConfig::default(),
        vec![source_account_id.clone()],
        vec![target_account_id.clone()],
    )
    .await;

    // Verify both rules were created
    let list_response = client.get("/recon_rules/v2").with_auth(&token).send().await;
    assert_eq!(list_response.status(), 200);
    let rules: Vec<GetReconRuleResponse> = list_response.json().await.unwrap();
    assert_eq!(rules.len(), 2, "Should have 2 recon rules created");

    // Verify priorities
    let invalid_rule_in_list = rules
        .iter()
        .find(|r| r.rule_id == invalid_rule.rule_id)
        .unwrap();
    assert_eq!(invalid_rule_in_list.priority, 1);

    let valid_rule_in_list = rules
        .iter()
        .find(|r| r.rule_id == valid_rule_response.rule_id)
        .unwrap();
    assert_eq!(valid_rule_in_list.priority, 2);

    let transformation_1 = create_transformation_config(
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

    let staging_entry_request = StagingEntryCreateRequest {
        account_id: source_account_id.to_string(),
        entry_type: BalanceDirection::Debit,
        processing_mode: StagingEntryProcessingMode::Transaction,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(
            serde_json::json!({"transaction_id": "test_value", "match_rules_field_1": "match_value"}),
        ),
        transformation_id: transformation_1.transformation_id,
        transformation_history_id: None,
        order_id: "order_test".to_string(),
    };

    let staging_entry_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request)
        .send()
        .await;

    assert_eq!(staging_entry_response.status(), 201);
    let staging_entry: GetStagingEntryResponse = staging_entry_response.json().await.unwrap();

    let process_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(process_response.status(), 200);
    let recon_trigger_response: ReconTriggerResponse = process_response.json().await.unwrap();
    assert!(matches!(
        recon_trigger_response,
        ReconTriggerResponse::Success(_)
    ));

    let transformation_2 = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
        StagingEntryProcessingMode::Confirmation,
        vec![],
    )
    .await;

    let staging_entry_request_confirmation = StagingEntryCreateRequest {
        account_id: target_account_id.to_string(),
        entry_type: BalanceDirection::Credit,
        processing_mode: StagingEntryProcessingMode::Confirmation,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(
            serde_json::json!({"transaction_id": "test_value", "match_rules_field_2": "match_value", "reference_id": "order_test"}),
        ),
        transformation_id: transformation_2.transformation_id,
        transformation_history_id: None,
        order_id: "order_test_23".to_string(),
    };

    let staging_entry_response_confirmation = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_request_confirmation)
        .send()
        .await;

    assert_eq!(staging_entry_response_confirmation.status(), 201);
    let staging_entry_confirmation: GetStagingEntryResponse =
        staging_entry_response_confirmation.json().await.unwrap();

    let process_response_confirmation = client
        .post(&format!(
            "/recon_engine/{}/process",
            staging_entry_confirmation.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(process_response_confirmation.status(), 200);
    let recon_trigger_response: ReconTriggerResponse =
        process_response_confirmation.json().await.unwrap();
    assert!(matches!(
        recon_trigger_response,
        ReconTriggerResponse::Success(_)
    ));
}

#[actix_web::test]
async fn test_multiple_search_expectations() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create two source accounts and one target account
    let source_account_id =
        create_test_account(&client, &token, "Source Account 1", BalanceDirection::Debit).await;
    let target_account_id =
        create_test_account(&client, &token, "Target Account", BalanceDirection::Credit).await;

    // Create Rule 1 - Source Account 1 -> Target Account
    let rule_1_request = ReconRuleCreateRequest {
        rule_name: "Rule 1 - Source 1 to Target".to_string(),
        rule_description: "First rule with source account 1".to_string(),
        priority: 1,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: Trigger::V1(TriggerV1 {
                        field: "metadata.source".parse().unwrap(),
                        operator: TriggerOperator::V1(TriggerOperatorV1 {
                            value: BasicComparisonOperator::Equals,
                        }),
                        value: "source_1".to_string(),
                    }),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id.clone(),
                    tolerance_config: None,
                },
                match_rules: MatchRules::V1(MatchRulesV1 { rules: vec![] }),
                search_identifier: SearchIdentifier::V2(SearchIdentifierV2::Single(
                    SingleSearchIdentifier {
                        key: SearchIdentifierKey {
                            source_field: "order_id".parse().unwrap(),
                            target_field: "metadata.reference_id".parse().unwrap(),
                        },
                    },
                )),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let rule_1_response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&rule_1_request)
        .send()
        .await;

    assert_eq!(rule_1_response.status(), 201);
    let _rule_1: GetReconRuleResponse = rule_1_response.json().await.unwrap();

    // Create Rule 2 - Source Account 2 -> Target Account
    let rule_2_request = ReconRuleCreateRequest {
        rule_name: "Rule 2 - Source 2 to Target".to_string(),
        rule_description: "Second rule with source account 2".to_string(),
        priority: 2,
        strategy: ReconStrategyApi::OneToOne(OneToOneStrategyApi::SingleSingle(
            OneToOneSingleSingleApi {
                source_account: OneToOneSingleSingleSourceApi {
                    account_id: source_account_id.clone(),
                    trigger: Trigger::V1(TriggerV1 {
                        field: "metadata.source".parse().unwrap(),
                        operator: TriggerOperator::V1(TriggerOperatorV1 {
                            value: BasicComparisonOperator::Equals,
                        }),
                        value: "source_2".to_string(),
                    }),
                },
                target_account: OneToOneSingleSingleTargetApi {
                    account_id: target_account_id.clone(),
                    tolerance_config: None,
                },
                match_rules: MatchRules::V1(MatchRulesV1 { rules: vec![] }),
                search_identifier: SearchIdentifier::V2(SearchIdentifierV2::Single(
                    SingleSearchIdentifier {
                        key: SearchIdentifierKey {
                            source_field: "order_id".parse().unwrap(),
                            target_field: "metadata.settlement_id".parse().unwrap(),
                        },
                    },
                )),
            },
        )),
        aging_config: AgingConfigApiRequest::NoAging,
    };

    let rule_2_response = client
        .post("/recon_rules/v2")
        .with_auth(&token)
        .json(&rule_2_request)
        .send()
        .await;

    assert_eq!(rule_2_response.status(), 201);
    let _rule_2: GetReconRuleResponse = rule_2_response.json().await.unwrap();

    // Create transformation for source account 1
    let transformation_1 = create_transformation_config(
        &profile_id,
        &source_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.source".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Create staging entry for source account 1
    let staging_entry_1_request = StagingEntryCreateRequest {
        account_id: source_account_id.to_string(),
        entry_type: BalanceDirection::Debit,
        processing_mode: StagingEntryProcessingMode::Transaction,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(serde_json::json!({"source": "source_1"})),
        transformation_id: transformation_1.transformation_id.clone(),
        transformation_history_id: None,
        order_id: "ref_123".to_string(),
    };

    let staging_entry_1_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_1_request)
        .send()
        .await;

    assert_eq!(staging_entry_1_response.status(), 201);
    let staging_entry_1: GetStagingEntryResponse = staging_entry_1_response.json().await.unwrap();

    // Process staging entry 1
    let process_1_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry_1.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(process_1_response.status(), 200);

    // Create staging entry for source account 2
    let staging_entry_2_request = StagingEntryCreateRequest {
        account_id: source_account_id.to_string(),
        entry_type: BalanceDirection::Debit,
        processing_mode: StagingEntryProcessingMode::Transaction,
        amount: FloatMajorUnit::new(50.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(serde_json::json!({"source": "source_2"})),
        transformation_id: transformation_1.transformation_id.clone(),
        transformation_history_id: None,
        order_id: "ref_123".to_string(),
    };

    let staging_entry_2_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_2_request)
        .send()
        .await;

    assert_eq!(staging_entry_2_response.status(), 201);
    let staging_entry_2: GetStagingEntryResponse = staging_entry_2_response.json().await.unwrap();

    // Process staging entry 2
    let process_2_response = client
        .post(&format!("/recon_engine/{}/process", staging_entry_2.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(process_2_response.status(), 200);

    // Create transformation for target account
    let transformation_target = create_transformation_config(
        &profile_id,
        &target_account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.unique_id".parse().unwrap(),
        StagingEntryProcessingMode::Confirmation,
        vec![],
    )
    .await;

    // Create staging entry for target account (should match with rule 1 based on priority)
    let staging_entry_target_request = StagingEntryCreateRequest {
        account_id: target_account_id.to_string(),
        entry_type: BalanceDirection::Credit,
        processing_mode: StagingEntryProcessingMode::Confirmation,
        amount: FloatMajorUnit::new(100.0),
        currency: Currency::USD,
        effective_at: common_utils::date_time::now(),
        metadata: Secret::new(
            serde_json::json!({"reference_id": "ref_123", "unique_id": "unq_1", "settlement_id": "ref_123"}),
        ),
        transformation_id: transformation_target.transformation_id,
        transformation_history_id: None,
        order_id: "order_test_target".to_string(),
    };

    let staging_entry_target_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&staging_entry_target_request)
        .send()
        .await;

    assert_eq!(staging_entry_target_response.status(), 201);
    let staging_entry_target: GetStagingEntryResponse =
        staging_entry_target_response.json().await.unwrap();

    // Process target staging entry
    let process_target_response = client
        .post(&format!(
            "/recon_engine/{}/process",
            staging_entry_target.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(process_target_response.status(), 200);

    let process_response: ReconTriggerResponse = process_target_response.json().await.unwrap();
    assert!(matches!(process_response, ReconTriggerResponse::Failure(_))); // Should fail since we get multiple expectations
}
