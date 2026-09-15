use crate::{api::account_test::create_test_account, common::TestApp};
use api_models::metadata_schema::{
    AmountSchemaConfig, AmountUnitConfigApi, BalanceDirectionSchemaConfig,
    CreateMetadataSchemaRequest, CurrencySchemaConfig, EffectiveAtSchemaConfig, FieldType,
    FieldsConfig, MetadataExposeConfig, MetadataFieldSchemaConfig, MetadataSchemaData,
    MetadataSchemaDataResponse, MetadataSchemaResponse, MetadataSchemaUpdateRequest,
    MinorUnitValidationRule, OrderIdSchemaConfig, StringValidationRule, UniqueConstraint,
    UniqueConstraintType,
};
use common_enums::{BalanceDirection, StagingEntryProcessingMode};
use common_types::{
    metadata_schema::datatypes::datetime::{
        DateFormatConfig, DateOrder, DateTimeFormat, Delimiter,
    },
    recon_rule::entry_field::EntryField,
};
use pretty_assertions::assert_eq;
use rstest::rstest;

pub fn basic_date_time_format() -> DateTimeFormat {
    DateTimeFormat {
        date_format: DateFormatConfig {
            order: DateOrder::DayMonthYear,
            delimiter: Delimiter::Hyphen,
        },
        time_format: None,
    }
}

// Helper function to create a basic schema request
pub fn create_basic_schema_request(account_id: &str) -> CreateMetadataSchemaRequest {
    CreateMetadataSchemaRequest {
        account_id: account_id.to_string(),
        schema_data: MetadataSchemaData::BasicSchemaData {
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
                    credit_values: vec!["credit".to_string(), "cr".to_string()],
                    debit_values: vec!["debit".to_string(), "dr".to_string()],
                    transformation_rules: vec![],
                },
                order_id: OrderIdSchemaConfig {
                    identifier: EntryField::OrderId.to_string(),
                    transformation_rules: vec![],
                },
                metadata_fields: vec![
                    MetadataFieldSchemaConfig {
                        identifier: "customer_id".to_string(),
                        field_name: "metadata.customer_id".parse().unwrap(),
                        field_config: FieldType::String {
                            validation_rules: vec![
                                StringValidationRule::MaxLength { value: 100 },
                                StringValidationRule::MinLength { value: 5 },
                            ],
                            transformation_rules: vec![],
                        },
                        required: true,
                        description: "Customer ID".to_string(),
                        is_exposed: None,
                    },
                    MetadataFieldSchemaConfig {
                        identifier: "customer_name".to_string(),
                        field_name: "metadata.customer_name".parse().unwrap(),
                        field_config: FieldType::String {
                            validation_rules: vec![
                                StringValidationRule::MaxLength { value: 100 },
                                StringValidationRule::MinLength { value: 1 },
                            ],
                            transformation_rules: vec![],
                        },
                        required: true,
                        description: "Customer's full name".to_string(),
                        is_exposed: None,
                    },
                    MetadataFieldSchemaConfig {
                        identifier: "customer_age".to_string(),
                        field_name: "metadata.customer_age".parse().unwrap(),
                        field_config: FieldType::Number {
                            validation_rules: vec![],
                            transformation_rules: vec![],
                        },
                        required: true,
                        description: "Customer's age".to_string(),
                        is_exposed: Some(MetadataExposeConfig {
                            is_searchable: true,
                        }),
                    },
                ],
            },
            unique_constraint: UniqueConstraint {
                constraint_type: UniqueConstraintType::SingleField {
                    field_name: "metadata.customer_id".parse().unwrap(),
                },
                description: "Customer ID must be unique per account".to_string(),
            },
            processing_mode: StagingEntryProcessingMode::Transaction,
        },
    }
}

// Helper function to create an update request
fn create_update_request() -> MetadataSchemaUpdateRequest {
    MetadataSchemaUpdateRequest {
        schema_data: MetadataSchemaData::BasicSchemaData {
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
                    credit_values: vec!["credit".to_string(), "cr".to_string()],
                    debit_values: vec!["debit".to_string(), "dr".to_string()],
                    transformation_rules: vec![],
                },
                order_id: OrderIdSchemaConfig {
                    identifier: EntryField::OrderId.to_string(),
                    transformation_rules: vec![],
                },
                metadata_fields: vec![
                    MetadataFieldSchemaConfig {
                        identifier: "customer_id".to_string(),
                        field_name: "metadata.customer_id".parse().unwrap(),
                        field_config: FieldType::String {
                            validation_rules: vec![
                                StringValidationRule::MaxLength { value: 100 },
                                StringValidationRule::MinLength { value: 5 },
                            ],
                            transformation_rules: vec![],
                        },
                        required: true,
                        description: "Customer ID".to_string(),
                        is_exposed: None,
                    },
                    MetadataFieldSchemaConfig {
                        identifier: "customer_name".to_string(),
                        field_name: "metadata.customer_name".parse().unwrap(),
                        field_config: FieldType::String {
                            validation_rules: vec![], // Empty validation rules
                            transformation_rules: vec![],
                        },
                        required: false,
                        description: "Customer's full name".to_string(),
                        is_exposed: None,
                    },
                ],
            },
            unique_constraint: UniqueConstraint {
                constraint_type: UniqueConstraintType::SingleField {
                    field_name: "metadata.customer_id".parse().unwrap(),
                },
                description: "Customer ID must be unique per account".to_string(),
            },
            processing_mode: StagingEntryProcessingMode::Transaction,
        },
    }
}

// Test create metadata schema - happy case
#[actix_web::test]
async fn test_create_metadata_schema_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let request = create_basic_schema_request(&account_id);

    let response = client
        .post("/metadata_schemas")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let schema: MetadataSchemaResponse = response.json().await.unwrap();

    // Verify the schema response structure
    assert!(schema.id.starts_with("meta_sch_"));
    assert!(schema.schema_id.starts_with("meta_sch_main_"));
    assert_eq!(schema.account_id, request.account_id);
    assert_eq!(schema.version, 1);

    // Verify schema data matches request
    match (&schema.schema_data, &request.schema_data) {
        (
            MetadataSchemaDataResponse::BasicSchemaData {
                fields: resp_fields,
                ..
            },
            MetadataSchemaData::BasicSchemaData {
                fields: req_fields, ..
            },
        ) => {
            assert_eq!(
                resp_fields.metadata_fields.len(),
                req_fields.metadata_fields.len()
            );

            // Check main fields
            assert_eq!(
                resp_fields.currency.identifier,
                req_fields.currency.identifier
            );
            assert_eq!(resp_fields.amount.identifier, req_fields.amount.identifier);
            assert_eq!(
                resp_fields.effective_at.identifier,
                req_fields.effective_at.identifier
            );
            assert_eq!(
                resp_fields.balance_direction.identifier,
                req_fields.balance_direction.identifier
            );

            // Check metadata fields
            for (resp_field, req_field) in resp_fields
                .metadata_fields
                .iter()
                .zip(req_fields.metadata_fields.iter())
            {
                assert_eq!(resp_field.identifier, req_field.identifier);
                assert_eq!(resp_field.field_name, req_field.field_name);
                assert_eq!(resp_field.description, req_field.description);
                assert_eq!(resp_field.is_exposed, req_field.is_exposed);
            }
        }
    }
}

// Test create metadata schema - wrong account (belongs to different profile)
#[actix_web::test]
async fn test_create_metadata_schema_wrong_account() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Create profile1 and account1
    let (_, _, token1) = client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let account1 =
        create_test_account(&client, &token1, "Test Account 1", BalanceDirection::Debit).await;

    // Create profile2
    let (_, _, token2) = client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    let request = create_basic_schema_request(&account1);

    // token2 trying to access account1 (belongs to profile1)
    let response = client
        .post("/metadata_schemas")
        .with_auth(&token2)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 404);
    let error: serde_json::Value = response.json().await.unwrap();
    assert!(error.get("error").is_some());
}

// Test get metadata schema by ID - happy case
#[actix_web::test]
async fn test_get_metadata_schema_by_id_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a schema first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let request = create_basic_schema_request(&account_id);

    let create_response = client
        .post("/metadata_schemas")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    let created_schema: MetadataSchemaResponse = create_response.json().await.unwrap();

    let response = client
        .get(&format!("/metadata_schemas/{}", created_schema.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let retrieved_schema: MetadataSchemaResponse = response.json().await.unwrap();
    assert_eq!(retrieved_schema.id, created_schema.id);
    assert!(retrieved_schema.id.starts_with("meta_sch_"));
    assert!(retrieved_schema.schema_id.starts_with("meta_sch_main_"));
    assert_eq!(retrieved_schema.version, 1);

    // Verify it's a BasicSchemaData as created
    match retrieved_schema.schema_data {
        MetadataSchemaDataResponse::BasicSchemaData { .. } => {} // Expected
    }
}

// Test get metadata schema by ID - invalid ID
#[actix_web::test]
async fn test_get_metadata_schema_by_id_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client
        .get("/metadata_schemas/non_existent_id")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

// Test list metadata schemas
#[actix_web::test]
async fn test_list_metadata_schemas() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create one account and multiple schemas
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let mut created_schema_ids = Vec::new();

    // Create 3 schemas for the same account
    for _ in 0..3 {
        let request = create_basic_schema_request(&account_id);

        let response = client
            .post("/metadata_schemas")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;

        assert_eq!(response.status(), 201);
        let created_schema: MetadataSchemaResponse = response.json().await.unwrap();
        created_schema_ids.push(created_schema.id);
    }

    // List all schemas
    let response = client
        .get("/metadata_schemas")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let schemas: Vec<MetadataSchemaResponse> = response.json().await.unwrap();

    // Should have created 3 schemas
    assert_eq!(schemas.len(), 3);

    // Verify all created schemas are in the list
    for created_id in &created_schema_ids {
        assert!(schemas.iter().any(|s| &s.id == created_id));
    }

    // Verify they're ordered by created_at descending (most recent first)
    for i in 1..schemas.len() {
        assert!(schemas[i - 1].created_at >= schemas[i].created_at);
    }
}

// Test list metadata schemas with filters (schema_id, account_id, time range, and validation)
#[rstest]
#[case(Some("first"), None, None, None, 1, 200)] // schema_id filter - first schema
#[case(Some("second"), None, None, None, 1, 200)] // schema_id filter - second schema
#[case(Some("third"), None, None, None, 1, 200)] // schema_id filter - third schema
#[case(Some("nonexistent"), None, None, None, 0, 200)] // schema_id filter - no match
#[case(None, Some("account1"), None, None, 2, 200)] // account_id filter - account1 schemas
#[case(None, Some("account2"), None, None, 1, 200)] // account_id filter - account2 schemas
#[case(None, Some("nonexistent"), None, None, 0, 200)] // account_id filter - no match
#[case(None, None, Some(-1), None, 3, 200)] // Past time range - all should match
#[case(None, None, Some(1), None, 0, 200)] // Future time range - none should match
#[case(None, None, Some(-1), Some(1), 3, 200)] // Wide time range - all should match
#[case(None, None, Some(1), Some(2), 0, 200)] // Future time range - none should match
#[case(Some("first"), None, Some(-1), Some(1), 1, 200)] // schema_id + time filter
#[case(None, Some("account1"), Some(-1), Some(1), 2, 200)] // account_id + time filter
#[case(None, None, Some(1), Some(2), 0, 200)] // start < end - valid
#[case(None, None, Some(2), Some(1), 0, 400)] // start > end - invalid
#[case(None, None, Some(1), Some(1), 0, 200)] // start = end - valid
#[case(None, None, Some(1), None, 0, 200)] // start_time only - valid
#[case(None, None, None, None, 3, 200)] // no time filter - valid
#[actix_web::test]
async fn test_list_metadata_schemas_with_filters(
    #[case] schema_id: Option<&str>,
    #[case] account_id: Option<&str>,
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_count: usize,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create test accounts and schemas for filtering
    let account1_id = create_test_account(
        &client,
        &token,
        "Filter Test Account 1",
        BalanceDirection::Debit,
    )
    .await;
    let account2_id = create_test_account(
        &client,
        &token,
        "Filter Test Account 2",
        BalanceDirection::Debit,
    )
    .await;
    let mut created_schemas = Vec::new();

    // Create 2 schemas for account1
    for _ in 0..2 {
        let request = create_basic_schema_request(&account1_id);

        let response = client
            .post("/metadata_schemas")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;

        assert_eq!(response.status(), 201);
        let created_schema: MetadataSchemaResponse = response.json().await.unwrap();
        created_schemas.push(created_schema);
    }

    // Create 1 schema for account2
    let request = create_basic_schema_request(&account2_id);
    let response = client
        .post("/metadata_schemas")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let created_schema: MetadataSchemaResponse = response.json().await.unwrap();
    created_schemas.push(created_schema);

    // Build query string
    let mut query_params = Vec::new();

    if let Some(schema_id_val) = schema_id {
        // Map the test case names to actual schema_ids
        let actual_schema_id = match schema_id_val {
            "first" => &created_schemas[0].schema_id,
            "second" => &created_schemas[1].schema_id,
            "third" => &created_schemas[2].schema_id,
            "nonexistent" => "nonexistent_schema_id",
            _ => schema_id_val, // fallback to literal value
        };
        query_params.push(format!("schema_id={actual_schema_id}"));
    }

    if let Some(account_id_val) = account_id {
        // Map the test case names to actual account_ids
        let actual_account_id = match account_id_val {
            "account1" => &account1_id,
            "account2" => &account2_id,
            "nonexistent" => "nonexistent_account_id",
            _ => account_id_val, // fallback to literal value
        };
        query_params.push(format!("account_id={actual_account_id}"));
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
        .get(&format!("/metadata_schemas{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 200 {
        let schemas: Vec<MetadataSchemaResponse> = response.json().await.unwrap();
        assert_eq!(schemas.len(), expected_count);

        // Validate filtered results match expected criteria
        for schema in &schemas {
            if let Some(schema_id_val) = schema_id {
                // Validate that the returned schema matches the requested schema_id
                let expected_schema_id = match schema_id_val {
                    "first" => &created_schemas[0].schema_id,
                    "second" => &created_schemas[1].schema_id,
                    "third" => &created_schemas[2].schema_id,
                    _ => schema_id_val, // For nonexistent or literal values
                };
                if schema_id_val != "nonexistent" {
                    assert_eq!(schema.schema_id, *expected_schema_id);
                }
            }

            if let Some(account_id_val) = account_id {
                // Validate that the returned schema matches the requested account_id
                let expected_account_id = match account_id_val {
                    "account1" => &account1_id,
                    "account2" => &account2_id,
                    _ => account_id_val, // For nonexistent or literal values
                };
                if account_id_val != "nonexistent" {
                    assert_eq!(schema.account_id, *expected_account_id);
                }
            }
        }
    } else {
        // For invalid requests, check error response
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

// Test update metadata schema - happy case
#[actix_web::test]
async fn test_update_metadata_schema_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a schema first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let request = create_basic_schema_request(&account_id);

    let create_response = client
        .post("/metadata_schemas")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    let created_schema: MetadataSchemaResponse = create_response.json().await.unwrap();

    let update_request = create_update_request();

    let response = client
        .put(&format!("/metadata_schemas/{}", created_schema.id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let updated_schema: MetadataSchemaResponse = response.json().await.unwrap();

    // Should create a new version
    assert_eq!(updated_schema.version, 2);
    assert_ne!(updated_schema.id, created_schema.id); // New ID for new version

    // Schema data should match update request
    match (&updated_schema.schema_data, &update_request.schema_data) {
        (
            MetadataSchemaDataResponse::BasicSchemaData {
                fields: resp_fields,
                ..
            },
            MetadataSchemaData::BasicSchemaData {
                fields: req_fields, ..
            },
        ) => {
            assert_eq!(
                resp_fields.metadata_fields.len(),
                req_fields.metadata_fields.len()
            );

            // Check main fields
            assert_eq!(
                resp_fields.currency.identifier,
                req_fields.currency.identifier
            );
            assert_eq!(resp_fields.amount.identifier, req_fields.amount.identifier);
            assert_eq!(
                resp_fields.effective_at.identifier,
                req_fields.effective_at.identifier
            );
            assert_eq!(
                resp_fields.balance_direction.identifier,
                req_fields.balance_direction.identifier
            );

            for (resp_field, req_field) in resp_fields
                .metadata_fields
                .iter()
                .zip(req_fields.metadata_fields.iter())
            {
                assert_eq!(resp_field.identifier, req_field.identifier);
                assert_eq!(resp_field.field_name, req_field.field_name);
                assert_eq!(resp_field.description, req_field.description);
                assert_eq!(resp_field.is_exposed, req_field.is_exposed);
            }
        }
    }
}

// Test update metadata schema - invalid schema ID
#[actix_web::test]
async fn test_update_metadata_schema_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let update_request = create_update_request();

    let response = client
        .put("/metadata_schemas/non_existent_id")
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

// Test profile isolation - ensure profiles only see their own schemas
#[actix_web::test]
async fn test_metadata_schemas_profile_isolation() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token1) = client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let (_, _, token2) = client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Create schemas for profile 1
    let account1_id =
        create_test_account(&client, &token1, "Account 1", BalanceDirection::Debit).await;
    let request1 = create_basic_schema_request(&account1_id);
    let create_response1 = client
        .post("/metadata_schemas")
        .with_auth(&token1)
        .json(&request1)
        .send()
        .await;
    assert_eq!(create_response1.status(), 201);

    // Create schemas for profile 2
    let account2_id =
        create_test_account(&client, &token2, "Account 2", BalanceDirection::Debit).await;
    let request2 = create_basic_schema_request(&account2_id);
    let create_response2 = client
        .post("/metadata_schemas")
        .with_auth(&token2)
        .json(&request2)
        .send()
        .await;
    assert_eq!(create_response2.status(), 201);

    // Profile 1 should only see its own schemas
    let response1 = client
        .get("/metadata_schemas")
        .with_auth(&token1)
        .send()
        .await;
    assert_eq!(response1.status(), 200);
    let schemas1: Vec<MetadataSchemaResponse> = response1.json().await.unwrap();
    assert_eq!(schemas1.len(), 1);
    assert_eq!(schemas1[0].account_id, account1_id);

    // Profile 2 should only see its own schemas
    let response2 = client
        .get("/metadata_schemas")
        .with_auth(&token2)
        .send()
        .await;
    assert_eq!(response2.status(), 200);
    let schemas2: Vec<MetadataSchemaResponse> = response2.json().await.unwrap();
    assert_eq!(schemas2.len(), 1);
    assert_eq!(schemas2[0].account_id, account2_id);
}

// ===== Config Validation Failure Tests =====

#[derive(Debug, Clone, Copy)]
enum ConfigValidationCase {
    EmptyFieldIdentifier,
    EmptyFieldDescription,
    DuplicateFieldNames,
    EmptyBalanceDirectionValues,
    DuplicateBalanceDirectionValues,
    EmptyBalanceDirectionValue,
    MultipleMaxLengthRules,
    EmptyUniqueConstraintDescription,
}

impl ConfigValidationCase {
    fn description(self) -> &'static str {
        match self {
            Self::EmptyFieldIdentifier => "empty field identifier",
            Self::EmptyFieldDescription => "empty field description",
            Self::DuplicateFieldNames => "duplicate field names",
            Self::EmptyBalanceDirectionValues => "empty balance direction values",
            Self::DuplicateBalanceDirectionValues => "duplicate balance direction values",
            Self::EmptyBalanceDirectionValue => "empty balance direction value",
            Self::MultipleMaxLengthRules => "multiple max length rules",
            Self::EmptyUniqueConstraintDescription => "empty unique constraint description",
        }
    }

    fn expected_error_message(self) -> &'static str {
        match self {
            Self::EmptyFieldIdentifier => "Empty field identifier",
            Self::EmptyFieldDescription => "Empty field description for identifier",
            Self::DuplicateFieldNames => "Duplicate metadata field name",
            Self::EmptyBalanceDirectionValues => "Balance direction arrays cannot be empty",
            Self::DuplicateBalanceDirectionValues => "Duplicate balance direction value",
            Self::EmptyBalanceDirectionValue => "Empty balance direction value",
            Self::MultipleMaxLengthRules => "Multiple rules of same type not allowed",
            Self::EmptyUniqueConstraintDescription => "Empty field description for identifier",
        }
    }

    fn apply_modification(self, request: &mut CreateMetadataSchemaRequest) {
        let MetadataSchemaData::BasicSchemaData {
            fields,
            unique_constraint,
            processing_mode: _,
        } = &mut request.schema_data;

        match self {
            Self::EmptyFieldIdentifier => {
                fields.metadata_fields[0].identifier = "".to_string();
            }
            Self::EmptyFieldDescription => {
                fields.metadata_fields[0].description = "".to_string();
            }
            Self::DuplicateFieldNames => {
                fields.metadata_fields[1].field_name = fields.metadata_fields[0].field_name.clone();
            }
            Self::EmptyBalanceDirectionValues => {
                fields.balance_direction.credit_values = vec![];
            }
            Self::DuplicateBalanceDirectionValues => {
                fields.balance_direction.credit_values =
                    vec!["credit".to_string(), "credit".to_string()];
            }
            Self::EmptyBalanceDirectionValue => {
                fields.balance_direction.credit_values.push("".to_string());
            }
            Self::MultipleMaxLengthRules => {
                fields.metadata_fields[0].field_config = FieldType::String {
                    validation_rules: vec![
                        StringValidationRule::MaxLength { value: 100 },
                        StringValidationRule::MaxLength { value: 200 }, // Duplicate
                    ],
                    transformation_rules: vec![],
                };
            }
            Self::EmptyUniqueConstraintDescription => {
                unique_constraint.description = "   ".to_string();
            }
        }
    }
}

fn create_invalid_schema_request(
    account_id: &str,
    case: ConfigValidationCase,
) -> CreateMetadataSchemaRequest {
    let mut request = create_basic_schema_request(account_id);
    case.apply_modification(&mut request);
    request
}

#[rstest]
#[case(ConfigValidationCase::EmptyFieldIdentifier)]
#[case(ConfigValidationCase::EmptyFieldDescription)]
#[case(ConfigValidationCase::DuplicateFieldNames)]
#[case(ConfigValidationCase::EmptyBalanceDirectionValues)]
#[case(ConfigValidationCase::DuplicateBalanceDirectionValues)]
#[case(ConfigValidationCase::EmptyBalanceDirectionValue)]
#[case(ConfigValidationCase::MultipleMaxLengthRules)]
#[case(ConfigValidationCase::EmptyUniqueConstraintDescription)]
#[actix_web::test]
async fn test_config_validation_failures(#[case] validation_case: ConfigValidationCase) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let request = create_invalid_schema_request(&account_id, validation_case);

    let response = client
        .post("/metadata_schemas")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        400,
        "Expected 400 for {}",
        validation_case.description()
    );

    let error: serde_json::Value = response.json().await.unwrap();
    let error_message = error
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap();

    assert!(
        error_message.contains(validation_case.expected_error_message()),
        "Expected error message '{}' not found in '{}'",
        validation_case.expected_error_message(),
        error_message
    );
}
