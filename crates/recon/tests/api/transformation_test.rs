use std::path::PathBuf;

use crate::{
    api::{
        account_test::create_test_account,
        ingestion_test::create_ingestion_config,
        metadata_schema_test::{basic_date_time_format, create_basic_schema_request},
    },
    common::TestApp,
};
use api_models::{
    metadata_schema::{
        AmountSchemaConfig, AmountUnitConfigApi, BalanceDirectionSchemaConfig,
        CurrencySchemaConfig, EffectiveAtSchemaConfig, FieldType, FieldsConfig,
        MetadataExposeConfig, MetadataFieldSchemaConfig, MetadataSchemaData,
        MetadataSchemaResponseV2, MinorUnitValidationRule, OrderIdSchemaConfig,
        StringTransformationRule, UniqueConstraint, UniqueConstraintType,
    },
    transformation::{
        CreateTransformationConfigV2Request, CsvParsingConfig, ParsingConfig, RowSkipConfig,
        SkipCondition, SkipConfig, SkipOperator, TransformationConfigResponse,
        TransformationConfigV2, TransformationHistoryResponse, TransformationTriggerRequest,
        UpdateTransformationConfigRequest, ValueSkipConfig,
    },
};
use common_enums::{BalanceDirection, StagingEntryProcessingMode};
use common_types::{
    metadata_schema::datatypes::string::{
        StringTransformationRule as DieselStringTransformationRule,
        StringTransformationRuleV1 as DieselStringTransformationRuleV1,
    },
    recon_rule::entry_field::EntryField,
    transformation::{
        RowSkipConfig as DieselRowSkipConfig, SkipCondition as DieselSkipCondition,
        SkipConfig as DieselSkipConfig, SkipOperator as DieselSkipOperator,
        TransformationConfigData,
    },
};
use common_utils::id_type::MerchantId;
use pretty_assertions::assert_eq;

pub fn transformation_config_request(
    merchant_id: &MerchantId,
    profile_id: &str,
    account_id: &str,
    ingestion_id: &str,
    unique_field: EntryField,
    processing_mode: StagingEntryProcessingMode,
    metadata_fields: Vec<(EntryField, bool, FieldType, Option<MetadataExposeConfig>)>,
    skip_configs: Vec<SkipConfig>,
) -> CreateTransformationConfigV2Request {
    CreateTransformationConfigV2Request {
        ingestion_id: ingestion_id.to_string(),
        account_id: account_id.to_string(),
        name: "Test Transformation".to_string(),
        metadata_schema_data: MetadataSchemaData::BasicSchemaData {
            fields: FieldsConfig {
                currency: CurrencySchemaConfig {
                    identifier: EntryField::Currency.to_string(),
                    transformation_rules: vec![],
                },
                amount: AmountSchemaConfig {
                    identifier: EntryField::Amount.to_string(),
                    unit_config: AmountUnitConfigApi::MinorUnit {
                        validation_rules: vec![MinorUnitValidationRule::PositiveOnly],
                        transformation_rules: vec![],
                    },
                },
                effective_at: EffectiveAtSchemaConfig {
                    identifier: EntryField::EffectiveAt.to_string(),
                    date_time_format: basic_date_time_format(),
                    transformation_rules: vec![],
                    post_parse_transformations: vec![],
                },
                balance_direction: BalanceDirectionSchemaConfig {
                    identifier: EntryField::EntryType.to_string(),
                    credit_values: vec![
                        "CREDIT".to_string(),
                        "Cr".to_string(),
                        "credit".to_string(),
                        "cr".to_string(),
                    ],
                    debit_values: vec![
                        "DEBIT".to_string(),
                        "Db".to_string(),
                        "debit".to_string(),
                        "db".to_string(),
                    ],
                    transformation_rules: vec![],
                },
                order_id: OrderIdSchemaConfig {
                    identifier: EntryField::OrderId.to_string(),
                    transformation_rules: vec![],
                },
                metadata_fields: metadata_fields
                    .into_iter()
                    .map(|(field, required, field_type, metadata_exposed_config)| {
                        MetadataFieldSchemaConfig {
                            identifier: field.to_field_path_without_metadata_prefix(),
                            field_name: field,
                            field_config: field_type,
                            required,
                            description: "Metadata field".to_string(),
                            is_exposed: metadata_exposed_config,
                        }
                    })
                    .collect(),
            },
            unique_constraint: UniqueConstraint {
                constraint_type: UniqueConstraintType::SingleField {
                    field_name: unique_field,
                },
                description: "Order Id must be unique per account".to_string(),
            },
            processing_mode,
        },
        config: TransformationConfigV2 {
            merchant_id: merchant_id.clone(),
            profile_id: profile_id.to_string(),
            account_id: account_id.to_string(),
            parsing_config: ParsingConfig::Csv(CsvParsingConfig),
            skip_configs,
        },
        is_active: true,
    }
}

pub async fn create_transformation_config(
    profile_id: &str,
    account_id: &str,
    ingestion_id: Option<&str>,
    merchant_id: &MerchantId,
    token: &str,
    client: &crate::common::TestClient,
    unique_field: EntryField,
    processing_mode: StagingEntryProcessingMode,
    metadata_fields: Vec<(EntryField, bool, FieldType, Option<MetadataExposeConfig>)>,
) -> TransformationConfigResponse {
    let ingestion_id = match ingestion_id {
        Some(id) => id,
        None => {
            let ingestion_config_response =
                create_ingestion_config(account_id, profile_id, merchant_id, client).await;
            &(ingestion_config_response.ingestion_id.clone())
        }
    };

    // Create transformation config
    let request = transformation_config_request(
        merchant_id,
        profile_id,
        account_id,
        ingestion_id,
        unique_field,
        processing_mode,
        metadata_fields,
        vec![],
    );
    let create_response = client
        .post("/transformations/v2/configs")
        .with_auth(token)
        .json(&request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    client.expect_success(create_response).await
}

#[actix_web::test]
async fn create_transformation_config_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    // Create transformation config
    let request = transformation_config_request(
        &merchant_id,
        &profile_id,
        &account_id,
        &ingestion_config_response.ingestion_id,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
        vec![],
    );
    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        201,
        "Failed to create transformation config"
    );
    let body: TransformationConfigResponse = client.expect_success(response).await;
    assert!(body.transformation_id.starts_with("tran_conf_"));
    assert_eq!(body.name, "Test Transformation");
    assert!(body.is_active);
}

#[actix_web::test]
async fn create_transformation_config_ingestion_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to create config with non-existent profile_id
    let request = CreateTransformationConfigV2Request {
        ingestion_id: "test_ingestion_id".to_string(),
        account_id: "any_account_id".to_string(),
        name: "Test Transformation".to_string(),
        metadata_schema_data: MetadataSchemaData::BasicSchemaData {
            fields: FieldsConfig {
                currency: CurrencySchemaConfig {
                    identifier: EntryField::Currency.to_string(),
                    transformation_rules: vec![],
                },
                amount: AmountSchemaConfig {
                    identifier: EntryField::Amount.to_string(),
                    unit_config: AmountUnitConfigApi::MinorUnit {
                        validation_rules: vec![MinorUnitValidationRule::PositiveOnly],
                        transformation_rules: vec![],
                    },
                },
                effective_at: EffectiveAtSchemaConfig {
                    identifier: EntryField::EffectiveAt.to_string(),
                    date_time_format: basic_date_time_format(),
                    transformation_rules: vec![],
                    post_parse_transformations: vec![],
                },
                balance_direction: BalanceDirectionSchemaConfig {
                    identifier: EntryField::EntryType.to_string(),
                    credit_values: vec!["CREDIT".to_string(), "Cr".to_string()],
                    debit_values: vec!["DEBIT".to_string(), "Db".to_string()],
                    transformation_rules: vec![],
                },
                order_id: OrderIdSchemaConfig {
                    identifier: EntryField::OrderId.to_string(),
                    transformation_rules: vec![],
                },
                metadata_fields: Vec::new(),
            },
            unique_constraint: UniqueConstraint {
                constraint_type: UniqueConstraintType::SingleField {
                    field_name: EntryField::OrderId,
                },
                description: "Order Id must be unique per account".to_string(),
            },
            processing_mode: StagingEntryProcessingMode::Transaction,
        },
        config: TransformationConfigV2 {
            merchant_id,
            profile_id,
            account_id: "any_account_id".to_string(),
            parsing_config: ParsingConfig::Csv(CsvParsingConfig),
            skip_configs: vec![],
        },
        is_active: true,
    };

    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        404,
        "Expected 404 for ingestion not found"
    );
}

#[actix_web::test]
async fn create_transformation_config_invalid_account_id_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    // Try to create config with non-existent account_id
    let request = CreateTransformationConfigV2Request {
        ingestion_id: ingestion_config_response.ingestion_id.clone(),
        account_id: "any_account_id".to_string(),
        name: "Test Transformation".to_string(),
        metadata_schema_data: MetadataSchemaData::BasicSchemaData {
            fields: FieldsConfig {
                currency: CurrencySchemaConfig {
                    identifier: EntryField::Currency.to_string(),
                    transformation_rules: vec![],
                },
                amount: AmountSchemaConfig {
                    identifier: EntryField::Amount.to_string(),
                    unit_config: AmountUnitConfigApi::MinorUnit {
                        validation_rules: vec![MinorUnitValidationRule::PositiveOnly],
                        transformation_rules: vec![],
                    },
                },
                effective_at: EffectiveAtSchemaConfig {
                    identifier: EntryField::EffectiveAt.to_string(),
                    date_time_format: basic_date_time_format(),
                    transformation_rules: vec![],
                    post_parse_transformations: vec![],
                },
                balance_direction: BalanceDirectionSchemaConfig {
                    identifier: EntryField::EntryType.to_string(),
                    credit_values: vec!["CREDIT".to_string(), "Cr".to_string()],
                    debit_values: vec!["DEBIT".to_string(), "Db".to_string()],
                    transformation_rules: vec![],
                },
                order_id: OrderIdSchemaConfig {
                    identifier: EntryField::OrderId.to_string(),
                    transformation_rules: vec![],
                },
                metadata_fields: Vec::new(),
            },
            unique_constraint: UniqueConstraint {
                constraint_type: UniqueConstraintType::SingleField {
                    field_name: EntryField::OrderId,
                },
                description: "Order Id must be unique per account".to_string(),
            },
            processing_mode: StagingEntryProcessingMode::Transaction,
        },
        config: TransformationConfigV2 {
            merchant_id: merchant_id.clone(),
            profile_id: profile_id.clone(),
            account_id: "invalid_account_id".to_string(),
            parsing_config: ParsingConfig::Csv(CsvParsingConfig),
            skip_configs: vec![],
        },
        is_active: true,
    };

    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        400,
        "Expected 400 for invalid account_id"
    );
}

#[actix_web::test]
async fn create_transformation_config_account_not_belonging_to_profile_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id_1, profile_id_1, token_1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let (_merchant_id_2, _profile_id_2, token_2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Create account in profile_1
    let account_id_1 =
        create_test_account(&client, &token_1, "Test Account", BalanceDirection::Debit).await;

    // Create account in profile_2
    let account_id_2 =
        create_test_account(&client, &token_2, "Test Account", BalanceDirection::Credit).await;

    let ingestion_response =
        create_ingestion_config(&account_id_1, &profile_id_1, &merchant_id_1, &client).await;

    // Try to create transformation config in profile_1 with account from profile_2
    let request = transformation_config_request(
        &merchant_id_1,
        &profile_id_1,
        &account_id_2,
        &ingestion_response.ingestion_id,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
        vec![],
    );

    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token_1)
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        400,
        "Expected 400 for account not belonging to profile"
    );
}

#[actix_web::test]
async fn get_transformation_config_by_id_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Get transformation config by ID
    let get_response = client
        .get(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        get_response.status(),
        200,
        "Failed to get transformation config by ID"
    );
    let retrieved_config: TransformationConfigResponse = client.expect_success(get_response).await;

    // Verify the retrieved config matches the created one
    assert_eq!(
        retrieved_config.transformation_id,
        created_config.transformation_id
    );
    assert_eq!(retrieved_config.name, created_config.name);
    assert_eq!(retrieved_config.profile_id, created_config.profile_id);
    assert_eq!(retrieved_config.account_id, created_config.account_id);
    assert_eq!(retrieved_config.ingestion_id, created_config.ingestion_id);
    assert_eq!(retrieved_config.is_active, created_config.is_active);
}

#[actix_web::test]
async fn get_transformation_config_by_id_cross_profile_access_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Create first profile and transformation config
    let (merchant_id_1, profile_id_1, token_1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;

    let account_id_1 =
        create_test_account(&client, &token_1, "Test Account", BalanceDirection::Debit).await;

    let created_config_1 = create_transformation_config(
        &profile_id_1,
        &account_id_1,
        None,
        &merchant_id_1,
        &token_1,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Create second profile
    let (_merchant_id_2, _profile_id_2, token_2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Try to access config from profile 1 using token from profile 2
    let get_response = client
        .get(&format!(
            "/transformations/configs/{}",
            created_config_1.transformation_id
        ))
        .with_auth(&token_2)
        .send()
        .await;

    assert_eq!(
        get_response.status(),
        404,
        "Expected 404 when accessing transformation config from different profile"
    );
}

#[actix_web::test]
async fn get_transformation_config_by_id_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to get non-existent transformation config
    let get_response = client
        .get("/transformations/configs/non_existent_config_id")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        get_response.status(),
        404,
        "Expected 404 when accessing non-existent transformation config"
    );
}

#[actix_web::test]
async fn get_metadata_by_transformation_id_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let transformation_response = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_response.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Get metadata by transformation ID
    let get_response = client
        .get(&format!(
            "/transformations/configs/{}/metadata_schema",
            transformation_response.transformation_id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        get_response.status(),
        200,
        "Failed to get metadata by transformation ID"
    );
    let metadata: MetadataSchemaResponseV2 = client.expect_success(get_response).await;

    // Verify the metadata schema
    assert!(metadata.schema_id.starts_with("meta_sch_"));
    assert_eq!(metadata.profile_id, profile_id);
    assert_eq!(metadata.account_id, account_id);
    assert_eq!(metadata.version, 1);
}

fn extract_skip_configs(response: &TransformationConfigResponse) -> &[DieselSkipConfig] {
    let TransformationConfigData::V2(config_data) = &response.config;
    &config_data.skip_configs
}

#[actix_web::test]
async fn create_transformation_config_with_row_skip_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let request = transformation_config_request(
        &merchant_id,
        &profile_id,
        &account_id,
        &ingestion_config_response.ingestion_id,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
        vec![SkipConfig {
            conditions: vec![SkipCondition::RowSkip(RowSkipConfig::LineNo {
                line_number: 2,
            })],
        }],
    );

    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let body: TransformationConfigResponse = client.expect_success(response).await;

    let skip_configs = extract_skip_configs(&body);
    assert_eq!(skip_configs.len(), 1);
    let DieselSkipConfig::V1(skip_v1) = &skip_configs[0];
    assert_eq!(skip_v1.conditions.len(), 1);
    match &skip_v1.conditions[0] {
        DieselSkipCondition::RowSkip(DieselRowSkipConfig::LineNo { line_number }) => {
            assert_eq!(*line_number, 2);
        }
        other => panic!("expected RowSkip::LineNo, got {other:?}"),
    }
}

#[actix_web::test]
async fn create_transformation_config_with_value_skip_both_operators_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let request = transformation_config_request(
        &merchant_id,
        &profile_id,
        &account_id,
        &ingestion_config_response.ingestion_id,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
        vec![
            SkipConfig {
                conditions: vec![SkipCondition::ValueSkip(ValueSkipConfig {
                    identifier: "status".to_string(),
                    operator: SkipOperator::Equals,
                    value: Some("IGNORED".to_string()),
                    transformation_rules: vec![StringTransformationRule::Trim],
                })],
            },
            SkipConfig {
                conditions: vec![SkipCondition::ValueSkip(ValueSkipConfig {
                    identifier: "currency".to_string(),
                    operator: SkipOperator::NotEquals,
                    value: Some("USD".to_string()),
                    transformation_rules: vec![],
                })],
            },
        ],
    );

    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let body: TransformationConfigResponse = client.expect_success(response).await;

    let skip_configs = extract_skip_configs(&body);
    assert_eq!(skip_configs.len(), 2);

    let DieselSkipConfig::V1(first) = &skip_configs[0];
    match &first.conditions[..] {
        [DieselSkipCondition::ValueSkip(vs)] => {
            assert_eq!(vs.identifier, "status");
            assert_eq!(vs.value.as_deref(), Some("IGNORED"));
            assert!(matches!(vs.operator, DieselSkipOperator::Equals));
            assert!(matches!(
                vs.transformation_rules.as_slice(),
                [DieselStringTransformationRule::V1(
                    DieselStringTransformationRuleV1::Trim
                )]
            ));
        }
        other => panic!("expected single ValueSkip condition, got {other:?}"),
    }

    let DieselSkipConfig::V1(second) = &skip_configs[1];
    match &second.conditions[..] {
        [DieselSkipCondition::ValueSkip(vs)] => {
            assert_eq!(vs.identifier, "currency");
            assert_eq!(vs.value.as_deref(), Some("USD"));
            assert!(matches!(vs.operator, DieselSkipOperator::NotEquals));
        }
        other => panic!("expected single ValueSkip condition, got {other:?}"),
    }
}

#[actix_web::test]
async fn create_transformation_config_omitted_skip_configs_defaults_to_empty() {
    // Covers `#[serde(default)]` on skip_configs: a client on an older schema
    // that omits the field entirely should still succeed and round-trip an empty vec.
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let request = transformation_config_request(
        &merchant_id,
        &profile_id,
        &account_id,
        &ingestion_config_response.ingestion_id,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
        vec![],
    );

    let mut raw_request = serde_json::to_value(&request).unwrap();
    raw_request
        .get_mut("config")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("skip_configs");

    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&raw_request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let body: TransformationConfigResponse = client.expect_success(response).await;
    assert!(extract_skip_configs(&body).is_empty());
}

#[actix_web::test]
async fn create_transformation_config_with_empty_skip_conditions_fails() {
    // A SkipConfig with no conditions matches every row under AND semantics
    // (vacuous truth), which would silently drop all rows. The API guards
    // against this — `validate_transformation_config_request` rejects it
    // with TR/1011 before it reaches the DB.
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config_response =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let request = transformation_config_request(
        &merchant_id,
        &profile_id,
        &account_id,
        &ingestion_config_response.ingestion_id,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
        vec![SkipConfig { conditions: vec![] }],
    );

    let response = client
        .post("/transformations/v2/configs")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 400);
}

#[actix_web::test]
async fn update_transformation_config_name_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let update_request = UpdateTransformationConfigRequest {
        name: Some("Renamed Transformation".to_string()),
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: None,
    };

    let update_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), 200);
    let body: TransformationConfigResponse = client.expect_success(update_response).await;

    assert_eq!(body.transformation_id, created_config.transformation_id);
    assert_eq!(body.name, "Renamed Transformation");
    // Identity fields are left untouched
    assert_eq!(body.profile_id, created_config.profile_id);
    assert_eq!(body.account_id, created_config.account_id);
    assert_eq!(body.ingestion_id, created_config.ingestion_id);
    assert_eq!(body.metadata_schema_id, created_config.metadata_schema_id);
}

#[actix_web::test]
async fn update_transformation_config_config_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Config starts with no skip configs
    assert!(extract_skip_configs(&created_config).is_empty());

    let update_request = UpdateTransformationConfigRequest {
        name: None,
        skip_configs: Some(vec![SkipConfig {
            conditions: vec![SkipCondition::RowSkip(RowSkipConfig::LineNo {
                line_number: 7,
            })],
        }]),
        parsing_config: None,
        metadata_schema_data: None,
        is_active: None,
    };

    let update_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), 200);
    let body: TransformationConfigResponse = client.expect_success(update_response).await;

    // Name is unchanged, skip configs are now applied
    assert_eq!(body.name, created_config.name);
    let skip_configs = extract_skip_configs(&body);
    assert_eq!(skip_configs.len(), 1);
    let DieselSkipConfig::V1(skip_v1) = &skip_configs[0];
    match &skip_v1.conditions[..] {
        [DieselSkipCondition::RowSkip(DieselRowSkipConfig::LineNo { line_number })] => {
            assert_eq!(*line_number, 7);
        }
        other => panic!("expected RowSkip::LineNo, got {other:?}"),
    }
}

#[actix_web::test]
async fn update_transformation_config_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let update_request = UpdateTransformationConfigRequest {
        name: Some("New Name".to_string()),
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: None,
    };

    let update_response = client
        .put("/transformations/configs/non_existent_config_id")
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(
        update_response.status(),
        404,
        "Expected 404 when updating non-existent transformation config"
    );
}

#[actix_web::test]
async fn update_transformation_config_cross_profile_access_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id_1, profile_id_1, token_1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;

    let account_id_1 =
        create_test_account(&client, &token_1, "Test Account", BalanceDirection::Debit).await;

    let created_config_1 = create_transformation_config(
        &profile_id_1,
        &account_id_1,
        None,
        &merchant_id_1,
        &token_1,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let (_merchant_id_2, _profile_id_2, token_2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    let update_request = UpdateTransformationConfigRequest {
        name: Some("Hijacked Name".to_string()),
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: None,
    };

    let update_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config_1.transformation_id
        ))
        .with_auth(&token_2)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(
        update_response.status(),
        404,
        "Expected 404 when updating transformation config from a different profile"
    );
}

#[actix_web::test]
async fn update_transformation_config_empty_name_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let update_request = UpdateTransformationConfigRequest {
        name: Some("   ".to_string()),
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: None,
    };

    let update_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(
        update_response.status(),
        400,
        "Expected 400 when updating with a blank name"
    );
}

#[actix_web::test]
async fn update_transformation_config_empty_skip_conditions_fails() {
    // Mirrors create: a SkipConfig with no conditions matches every row under
    // AND semantics, so the update path must run the same TR/1011 guard.
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let update_request = UpdateTransformationConfigRequest {
        name: None,
        skip_configs: Some(vec![SkipConfig { conditions: vec![] }]),
        parsing_config: None,
        metadata_schema_data: None,
        is_active: None,
    };

    let update_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), 400);
}

#[actix_web::test]
async fn update_transformation_config_metadata_schema_data_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Update the linked metadata schema in-place with new schema data.
    let new_schema_data = create_basic_schema_request(&account_id).schema_data;
    let update_request = UpdateTransformationConfigRequest {
        name: None,
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: Some(new_schema_data),
        is_active: None,
    };

    let update_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), 200);
    let body: TransformationConfigResponse = client.expect_success(update_response).await;

    // The transformation config pointer is unchanged — same ID, same linked schema ID.
    assert_eq!(body.transformation_id, created_config.transformation_id);
    assert_eq!(body.metadata_schema_id, created_config.metadata_schema_id);
    assert_eq!(body.name, created_config.name);

    // Confirm the linked schema is still reachable and belongs to this config.
    let schema_response = client
        .get(&format!(
            "/transformations/configs/{}/metadata_schema",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(schema_response.status(), 200);
    let schema: MetadataSchemaResponseV2 = client.expect_success(schema_response).await;
    assert_eq!(schema.id, created_config.metadata_schema_id);
}

#[actix_web::test]
async fn update_transformation_config_is_active_toggle() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;
    assert!(created_config.is_active);

    let deactivate = UpdateTransformationConfigRequest {
        name: None,
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: Some(false),
    };

    let deactivate_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&deactivate)
        .send()
        .await;

    assert_eq!(deactivate_response.status(), 200);
    let body: TransformationConfigResponse = client.expect_success(deactivate_response).await;
    assert!(!body.is_active);
    assert_eq!(body.name, created_config.name);
    assert!(body.last_modified_at > created_config.last_modified_at);

    let fetched = client
        .get(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(fetched.status(), 200);
    let persisted: TransformationConfigResponse = client.expect_success(fetched).await;
    assert!(!persisted.is_active);

    let reactivate = UpdateTransformationConfigRequest {
        name: None,
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: Some(true),
    };

    let reactivate_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&reactivate)
        .send()
        .await;

    assert_eq!(reactivate_response.status(), 200);
    let body: TransformationConfigResponse = client.expect_success(reactivate_response).await;
    assert!(body.is_active);
}

#[actix_web::test]
async fn update_transformation_config_is_active_unchanged_when_omitted() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let created_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let deactivate = UpdateTransformationConfigRequest {
        name: None,
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: Some(false),
    };

    let deactivate_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&deactivate)
        .send()
        .await;
    assert_eq!(deactivate_response.status(), 200);

    let name_only = UpdateTransformationConfigRequest {
        name: Some("Renamed While Inactive".to_string()),
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: None,
    };

    let name_only_response = client
        .put(&format!(
            "/transformations/configs/{}",
            created_config.transformation_id
        ))
        .with_auth(&token)
        .json(&name_only)
        .send()
        .await;

    assert_eq!(name_only_response.status(), 200);
    let body: TransformationConfigResponse = client.expect_success(name_only_response).await;
    assert_eq!(body.name, "Renamed While Inactive");
    assert!(
        !body.is_active,
        "Omitting is_active must leave the stored value untouched"
    );
}

#[actix_web::test]
async fn update_transformation_config_inactive_excluded_from_list_filter() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let active_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let inactive_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let deactivate = UpdateTransformationConfigRequest {
        name: None,
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: Some(false),
    };

    let deactivate_response = client
        .put(&format!(
            "/transformations/configs/{}",
            inactive_config.transformation_id
        ))
        .with_auth(&token)
        .json(&deactivate)
        .send()
        .await;
    assert_eq!(deactivate_response.status(), 200);

    let active_listing = client
        .get("/transformations/configs?is_active=true")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(active_listing.status(), 200);
    let active_configs: Vec<TransformationConfigResponse> =
        client.expect_success(active_listing).await;
    let active_ids: Vec<&str> = active_configs
        .iter()
        .map(|config| config.transformation_id.as_str())
        .collect();
    assert!(active_ids.contains(&active_config.transformation_id.as_str()));
    assert!(!active_ids.contains(&inactive_config.transformation_id.as_str()));

    let inactive_listing = client
        .get("/transformations/configs?is_active=false")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(inactive_listing.status(), 200);
    let inactive_configs: Vec<TransformationConfigResponse> =
        client.expect_success(inactive_listing).await;
    let inactive_ids: Vec<&str> = inactive_configs
        .iter()
        .map(|config| config.transformation_id.as_str())
        .collect();
    assert_eq!(
        inactive_ids,
        vec![inactive_config.transformation_id.as_str()]
    );
}

#[actix_web::test]
async fn update_transformation_config_inactive_excluded_from_execution() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, &client).await;

    let active_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let inactive_config = create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config.ingestion_id),
        &merchant_id,
        &token,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    let deactivate = UpdateTransformationConfigRequest {
        name: None,
        parsing_config: None,
        skip_configs: None,
        metadata_schema_data: None,
        is_active: Some(false),
    };

    let deactivate_response = client
        .put(&format!(
            "/transformations/configs/{}",
            inactive_config.transformation_id
        ))
        .with_auth(&token)
        .json(&deactivate)
        .send()
        .await;
    assert_eq!(deactivate_response.status(), 200);

    let trigger_request = TransformationTriggerRequest {
        file_path: PathBuf::from("no_such_file_for_is_active_test.csv"),
    };

    let trigger_response = client
        .post(&format!(
            "/transformations/{}/ing_hist_is_active_test/trigger",
            ingestion_config.ingestion_id
        ))
        .with_internal_key()
        .json(&trigger_request)
        .send()
        .await;
    assert_ne!(
        trigger_response.status(),
        404,
        "The active config must still be found for this ingestion"
    );

    let history_response = client
        .get("/transformations/history")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: Vec<TransformationHistoryResponse> = client.expect_success(history_response).await;

    let triggered_ids: Vec<&str> = history
        .iter()
        .map(|entry| entry.transformation_id.as_str())
        .collect();
    assert_eq!(
        triggered_ids,
        vec![active_config.transformation_id.as_str()],
        "Only the active config should be executed for the ingestion"
    );
}

#[actix_web::test]
async fn test_find_transformation_history_is_scoped_to_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id_1, profile_id_1, token_1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;

    let account_id_1 =
        create_test_account(&client, &token_1, "Test Account", BalanceDirection::Debit).await;

    let ingestion_config =
        create_ingestion_config(&account_id_1, &profile_id_1, &merchant_id_1, &client).await;

    create_transformation_config(
        &profile_id_1,
        &account_id_1,
        Some(&ingestion_config.ingestion_id),
        &merchant_id_1,
        &token_1,
        &client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;

    // Trigger a transformation so profile 1 owns a history record
    let trigger_request = TransformationTriggerRequest {
        file_path: PathBuf::from("no_such_file_for_scoping_test.csv"),
    };
    client
        .post(&format!(
            "/transformations/{}/ing_hist_scoping_test/trigger",
            ingestion_config.ingestion_id
        ))
        .with_internal_key()
        .json(&trigger_request)
        .send()
        .await;

    let history_response = client
        .get("/transformations/history")
        .with_auth(&token_1)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: Vec<TransformationHistoryResponse> = client.expect_success(history_response).await;
    let history_entry = history
        .first()
        .expect("Should have a transformation history entry");

    // Create second profile
    let (_merchant_id_2, _profile_id_2, token_2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Owner can read it
    let owner_response = client
        .get(&format!(
            "/transformations/history/{}",
            history_entry.transformation_history_id
        ))
        .with_auth(&token_1)
        .send()
        .await;

    assert_eq!(
        owner_response.status(),
        200,
        "Owning profile should be able to read its own transformation history"
    );

    // Another profile must not
    let cross_tenant_response = client
        .get(&format!(
            "/transformations/history/{}",
            history_entry.transformation_history_id
        ))
        .with_auth(&token_2)
        .send()
        .await;

    assert_eq!(
        cross_tenant_response.status(),
        404,
        "another profile's transformation history id must not resolve"
    );
}

#[actix_web::test]
async fn find_transformation_history_with_config_by_id_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client
        .get("/transformations/history/non_existent_history_id")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        response.status(),
        404,
        "Expected 404 for a non-existent transformation history id"
    );
}

/// Sets up one profile with an ingestion and an active transformation config.
/// Returns `(ingestion_id, owner_token)`.
async fn setup_trigger_target(client: &crate::common::TestClient) -> (String, String) {
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let account_id =
        create_test_account(client, &token, "Test Account", BalanceDirection::Debit).await;
    let ingestion_config =
        create_ingestion_config(&account_id, &profile_id, &merchant_id, client).await;
    create_transformation_config(
        &profile_id,
        &account_id,
        Some(&ingestion_config.ingestion_id),
        &merchant_id,
        &token,
        client,
        "order_id".parse().unwrap(),
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;
    (ingestion_config.ingestion_id, token)
}

fn trigger_path(ingestion_id: &str) -> String {
    format!("/transformations/{ingestion_id}/ing_hist_internal_auth_test/trigger")
}

fn trigger_body() -> TransformationTriggerRequest {
    TransformationTriggerRequest {
        file_path: PathBuf::from("no_such_file_for_internal_auth_test.csv"),
    }
}

#[actix_web::test]
async fn trigger_without_internal_key_is_401() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (ingestion_id, token) = setup_trigger_target(&client).await;

    let response = client
        .post(&trigger_path(&ingestion_id))
        .json(&trigger_body())
        .send()
        .await;
    assert_eq!(response.status(), 401);

    // A rejected trigger must not have written anything into the owner's ledger.
    let history_response = client
        .get("/transformations/history")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: Vec<TransformationHistoryResponse> = client.expect_success(history_response).await;
    assert!(
        history.is_empty(),
        "Unauthenticated trigger must not create transformation history"
    );
}

#[actix_web::test]
async fn trigger_with_internal_key_succeeds() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (ingestion_id, token) = setup_trigger_target(&client).await;

    let response = client
        .post(&trigger_path(&ingestion_id))
        .with_internal_key()
        .json(&trigger_body())
        .send()
        .await;
    assert_ne!(response.status(), 401, "Valid internal key must pass auth");

    let history_response = client
        .get("/transformations/history")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: Vec<TransformationHistoryResponse> = client.expect_success(history_response).await;
    assert_eq!(
        history.len(),
        1,
        "Authenticated trigger should create a transformation history row"
    );
}
