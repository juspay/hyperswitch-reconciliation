use crate::{api::account_test::create_test_account, common::TestApp};
use api_models::ingestion::{
    IngestionConfigCreateRequest, IngestionConfigData, IngestionConfigListResponse,
    IngestionConfigResponse, IngestionConfigUpdateRequest, IngestionHistoryListResponse,
};
use base64::{engine::general_purpose, Engine};
use common_enums::BalanceDirection;
use common_types::ingestion::adyen::{
    AdditionalData, AdyenTriggerData, Amount, EventCode, NotificationItemWrapper,
    NotificationRequestItem,
};
use common_utils::id_type::MerchantId;
use masking::Secret;

use hmac::{Hmac, Mac};
use pretty_assertions::assert_eq;
use rstest::rstest;
use sha2::Sha256;
use std::path::PathBuf;

type HmacSha256 = Hmac<Sha256>;

fn create_adyen_config() -> IngestionConfigData {
    IngestionConfigData::Adyen {
        hmac_secret: Secret::new(
            "44782DEF547AAA06C910C43932B1EB0C71FC68D9D0C057550C48EC2ACF6BA056".to_string(),
        ),
        webhook_basic_auth_username: "webhook_user".to_string(),
        webhook_basic_auth_password: Secret::new("webhook_pass".to_string()),
        report_basic_auth_username: "report_user".to_string(),
        report_basic_auth_password: Secret::new("report_pass".to_string()),
    }
}

fn create_sftp_internal_config() -> IngestionConfigData {
    IngestionConfigData::SftpInternal {
        file_path: PathBuf::from("test-files/settlement_report.csv"),
    }
}

pub fn ingestion_config_request(
    account_id: &str,
    profile_id: &str,
    merchant_id: &MerchantId,
) -> IngestionConfigCreateRequest {
    IngestionConfigCreateRequest {
        merchant_id: merchant_id.clone(),
        profile_id: profile_id.to_string(),
        name: "Test Adyen Integration".to_string(),
        description: Some("Test ingestion for Adyen settlement reports".to_string()),
        account_id: account_id.to_string(),
        config: create_adyen_config(),
        is_active: true,
    }
}

pub async fn create_ingestion_config(
    account_id: &str,
    profile_id: &str,
    merchant_id: &MerchantId,
    client: &crate::common::TestClient,
) -> IngestionConfigResponse {
    let request = ingestion_config_request(account_id, profile_id, merchant_id);
    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201, "Failed to create ingestion config");
    client.expect_success(response).await
}

fn create_test_webhook_payload() -> AdyenTriggerData {
    AdyenTriggerData {
        live: "false".to_string(),
        notification_items: vec![NotificationItemWrapper {
            notification_request_item: NotificationRequestItem {
                amount: Amount {
                    currency: "EUR".to_string(),
                    value: 10100,
                },
                event_code: EventCode::ReportAvailable,
                event_date: "2023-01-01T12:00:00Z".to_string(),
                merchant_account_code: "TestMerchant".to_string(),
                merchant_reference: "settlement_report_123".to_string(),
                original_reference: Some("original_ref_123".to_string()),
                psp_reference: "settlement_detail_report_batch_589.csv".to_string(),
                reason: "https://ca-test.adyen.com/reports/download/Settlement/TestMerchant/settlement_detail_report_batch_589.csv".to_string(),
                success: "true".to_string(),
                additional_data: AdditionalData {
                    hmac_signature: String::new(), // Will be filled by helper function
                },
            },
        }],
    }
}

fn generate_valid_hmac_signature(
    webhook_data: &mut AdyenTriggerData,
    hmac_secret: &str,
) -> Result<(), String> {
    let notification = &webhook_data.notification_items[0].notification_request_item;

    // Create the payload string following Adyen's format
    let payload_string = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        notification.psp_reference,
        notification.original_reference.as_deref().unwrap_or(""),
        notification.merchant_account_code,
        notification.merchant_reference,
        notification.amount.value,
        notification.amount.currency,
        "REPORT_AVAILABLE",
        notification.success.to_lowercase(),
    );

    // Decode the hex secret
    let hmac_key_bytes =
        hex::decode(hmac_secret).map_err(|_| "Test setup error: Invalid hex secret".to_string())?;

    // Compute HMAC
    let mut mac = HmacSha256::new_from_slice(&hmac_key_bytes)
        .map_err(|_| "Test setup error: Invalid HMAC key".to_string())?;
    mac.update(payload_string.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_base64 = general_purpose::STANDARD.encode(signature);

    // Update the webhook data
    webhook_data.notification_items[0]
        .notification_request_item
        .additional_data
        .hmac_signature = signature_base64;

    Ok(())
}

fn create_basic_auth_header(username: &str, password: &str) -> String {
    let credentials = format!("{username}:{password}");
    let encoded = general_purpose::STANDARD.encode(credentials);
    format!("Basic {encoded}")
}

// ========== Common Test Setup Helpers ==========

/// Helper to create an ingestion config and return the response
async fn create_test_ingestion_config(
    client: &crate::common::TestClient,
    merchant_id: &MerchantId,
    profile_id: &str,
    account_id: &str,
    config_data: IngestionConfigData,
    name: &str,
) -> IngestionConfigResponse {
    let request = IngestionConfigCreateRequest {
        merchant_id: merchant_id.clone(),
        profile_id: profile_id.to_string(),
        name: name.to_string(),
        description: None,
        account_id: account_id.to_string(),
        config: config_data,
        is_active: true,
    };

    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201, "Failed to create ingestion config");
    client.expect_success(response).await
}

/// Helper to upload a file to a manual ingestion config
async fn upload_test_file(
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

    assert_eq!(response.status(), 200, "File upload should succeed");
}

#[actix_web::test]
async fn create_ingestion_config_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap test environment
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a test account first (ingestion configs require an account_id)
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    // Create ingestion config
    let request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201, "Failed to create ingestion config");
    let body: IngestionConfigResponse = client.expect_success(response).await;
    assert!(body.ingestion_id.starts_with("ing_"));
    assert_eq!(body.name, "Test Adyen Integration");
    assert!(body.is_active);
}

#[actix_web::test]
async fn create_ingestion_config_requires_admin_key() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to create config with JWT instead of admin key
    let request = ingestion_config_request("test_account_id", &profile_id, &merchant_id);
    let response = client
        .post("/ingestions/config")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        401,
        "Expected 401 when using JWT instead of admin key"
    );
}

#[actix_web::test]
async fn list_ingestion_configs_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    // Create two ingestion configs
    for i in 1..=2 {
        let mut request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
        request.name = format!("Test Config {i}");

        let response = client
            .post("/ingestions/config")
            .with_admin_key()
            .json(&request)
            .send()
            .await;
        assert_eq!(response.status(), 201, "Failed to create test config {}", i);
    }

    // List configs
    let response = client
        .get("/ingestions/config")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200, "Failed to list ingestion configs");
    let configs: IngestionConfigListResponse = client.expect_success(response).await;
    assert_eq!(configs.len(), 2);
    assert!(configs.iter().any(|c| c.name == "Test Config 1"));
    assert!(configs.iter().any(|c| c.name == "Test Config 2"));
}

#[actix_web::test]
async fn get_ingestion_config_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    let create_response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        create_response.status(),
        201,
        "Failed to create ingestion config"
    );
    let created: IngestionConfigResponse = client.expect_success(create_response).await;

    // Get the config
    let get_response = client
        .get(&format!("/ingestions/config/{}", created.ingestion_id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(get_response.status(), 200, "Failed to get ingestion config");
    let fetched: IngestionConfigResponse = client.expect_success(get_response).await;
    assert_eq!(fetched.ingestion_id, created.ingestion_id);
    assert_eq!(fetched.name, "Test Adyen Integration");
}

#[actix_web::test]
async fn update_ingestion_config_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    let create_response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        create_response.status(),
        201,
        "Failed to create ingestion config"
    );
    let created: IngestionConfigResponse = client.expect_success(create_response).await;

    // Update the config
    let update_request = IngestionConfigUpdateRequest {
        is_active: Some(false),
        config: None,
    };

    let update_response = client
        .put(&format!("/ingestions/config/{}", created.ingestion_id))
        .with_admin_key()
        .json(&update_request)
        .send()
        .await;

    assert_eq!(
        update_response.status(),
        200,
        "Failed to update ingestion config"
    );
}

#[actix_web::test]
async fn trigger_ingestion_invalid_auth_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    let create_response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        create_response.status(),
        201,
        "Failed to create ingestion config"
    );
    let created: IngestionConfigResponse = client.expect_success(create_response).await;

    let mut webhook_payload = create_test_webhook_payload();
    let _ = generate_valid_hmac_signature(
        &mut webhook_payload,
        "44782DEF547AAA06C910C43932B1EB0C71FC68D9D0C057550C48EC2ACF6BA056",
    );

    // Send with wrong basic auth credentials
    let response = client
        .post(&format!("/ingestions/{}/trigger", created.ingestion_id))
        .header(
            "Authorization",
            &create_basic_auth_header("wrong_user", "wrong_pass"),
        )
        .json(&webhook_payload)
        .send()
        .await;

    assert_eq!(response.status(), 400); // Should fail validation
}

#[actix_web::test]
async fn trigger_ingestion_inactive_config_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let mut request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    request.is_active = false; // Create inactive config

    let create_response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        create_response.status(),
        201,
        "Failed to create ingestion config"
    );
    let created: IngestionConfigResponse = client.expect_success(create_response).await;

    let mut webhook_payload = create_test_webhook_payload();
    let _ = generate_valid_hmac_signature(
        &mut webhook_payload,
        "44782DEF547AAA06C910C43932B1EB0C71FC68D9D0C057550C48EC2ACF6BA056",
    );

    let response = client
        .post(&format!("/ingestions/{}/trigger", created.ingestion_id))
        .header(
            "Authorization",
            &create_basic_auth_header("webhook_user", "webhook_pass"),
        )
        .json(&webhook_payload)
        .send()
        .await;

    assert_eq!(response.status(), 400);
}

#[actix_web::test]
async fn trigger_ingestion_invalid_hmac_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    let create_response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        create_response.status(),
        201,
        "Failed to create ingestion config"
    );
    let created: IngestionConfigResponse = client.expect_success(create_response).await;

    // Create webhook with invalid HMAC signature
    let mut webhook_payload = create_test_webhook_payload();
    webhook_payload.notification_items[0]
        .notification_request_item
        .additional_data
        .hmac_signature = "invalid_signature".to_string();

    let response = client
        .post(&format!("/ingestions/{}/trigger", created.ingestion_id))
        .header(
            "Authorization",
            &create_basic_auth_header("webhook_user", "webhook_pass"),
        )
        .json(&webhook_payload)
        .send()
        .await;

    assert_eq!(response.status(), 400);

    // Check ingestion history shows versioned failure with proper discarding
    let history_response = client
        .get("/ingestions/history")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        history_response.status(),
        200,
        "Failed to list ingestion history"
    );
    let history: IngestionHistoryListResponse = client.expect_success(history_response).await;

    // Should have version entries showing failure progression: Pending (v1) -> Failed (v2)
    assert!(!history.is_empty(), "Should have ingestion history entries");

    let history_by_version: std::collections::BTreeMap<
        i32,
        &api_models::ingestion::IngestionHistoryResponse,
    > = history.iter().map(|h| (h.version, h)).collect();

    assert!(
        history_by_version.contains_key(&1),
        "Should have version 1 (Pending)"
    );
    assert!(
        history_by_version.contains_key(&2),
        "Should have version 2 (Failed)"
    );

    // Verify status progression and discarding behavior
    // Version 1 was originally Pending but is now Discarded (status updated to Discarded)
    assert_eq!(
        history_by_version[&1].status,
        common_enums::IngestionStatus::Discarded,
        "Version 1 should have status Discarded after being superseded"
    );
    assert!(
        history_by_version[&1].discarded_at.is_some(),
        "Version 1 should have discarded_at timestamp"
    );
    assert_eq!(
        history_by_version[&1].discarded_status,
        Some(common_enums::IngestionStatus::Pending),
        "Version 1 should have discarded_status = Pending (original status)"
    );

    // Version 2 (Failed) should be active (not discarded) - it's the latest
    assert_eq!(
        history_by_version[&2].status,
        common_enums::IngestionStatus::Failed,
        "Version 2 should be Failed"
    );
    assert!(
        history_by_version[&2].discarded_at.is_none(),
        "Version 2 should be active (not discarded)"
    );
    assert!(
        history_by_version[&2].discarded_status.is_none(),
        "Version 2 should have no discarded_status"
    );
}

#[actix_web::test]
async fn list_ingestion_history_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    let create_response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        create_response.status(),
        201,
        "Failed to create ingestion config"
    );
    let _created: IngestionConfigResponse = client.expect_success(create_response).await;

    // List history (should be empty initially)
    let response = client
        .get("/ingestions/history")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200, "Failed to list ingestion history");
    let history: IngestionHistoryListResponse = client.expect_success(response).await;
    assert_eq!(history.len(), 0);
}

#[rstest]
#[case("", 201)] // Empty name currently allowed
#[case("Test Config", 201)] // Valid name
#[actix_web::test]
async fn create_ingestion_config_validation(
    #[case] config_name: &str,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let mut request = ingestion_config_request(&account_id, &profile_id, &merchant_id);
    request.name = config_name.to_string();

    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        expected_status,
        "Unexpected status for config name: '{}'",
        config_name
    );
}

#[actix_web::test]
async fn ingestion_config_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client
        .get("/ingestions/config/non_existent_id")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        response.status(),
        404,
        "Expected 404 for non-existent config"
    );
}

#[actix_web::test]
async fn create_sftp_internal_ingestion_config_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create a test account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    // Create SFTP Internal ingestion config
    let request = IngestionConfigCreateRequest {
        merchant_id: merchant_id.clone(),
        profile_id: profile_id.to_string(),
        name: "Test SFTP Internal Integration".to_string(),
        description: Some("Test ingestion for SFTP Internal S3 files".to_string()),
        account_id: account_id.to_string(),
        config: create_sftp_internal_config(),
        is_active: true,
    };

    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        201,
        "Failed to create SFTP Internal ingestion config"
    );
    let created: IngestionConfigResponse = client.expect_success(response).await;
    assert_eq!(created.name, "Test SFTP Internal Integration");
    assert!(created.is_active);

    // Verify we can retrieve it
    let get_response = client
        .get(&format!("/ingestions/config/{}", created.ingestion_id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        get_response.status(),
        200,
        "Failed to get SFTP Internal config"
    );
    let fetched: IngestionConfigResponse = client.expect_success(get_response).await;
    assert_eq!(fetched.ingestion_id, created.ingestion_id);
}

#[actix_web::test]
async fn create_ingestion_config_invalid_profile_id_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, _profile_id, _token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to create config with non-existent profile_id
    let request = IngestionConfigCreateRequest {
        merchant_id,
        profile_id: "invalid_profile_id".to_string(),
        name: "Test Config".to_string(),
        description: None,
        account_id: "any_account_id".to_string(),
        config: create_adyen_config(),
        is_active: true,
    };

    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        response.status(),
        400,
        "Expected 400 for invalid profile_id"
    );
}

#[actix_web::test]
async fn create_ingestion_config_invalid_account_id_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, _token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to create config with non-existent account_id
    let request = IngestionConfigCreateRequest {
        merchant_id,
        profile_id,
        name: "Test Config".to_string(),
        description: None,
        account_id: "invalid_account_id".to_string(),
        config: create_adyen_config(),
        is_active: true,
    };

    let response = client
        .post("/ingestions/config")
        .with_admin_key()
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
async fn create_ingestion_config_account_not_belonging_to_profile_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id_1, profile_id_1, _token_1) =
        client.bootstrap("Test Merchant 1", "Test Profile 1").await;
    let (_merchant_id_2, _profile_id_2, token_2) =
        client.bootstrap("Test Merchant 2", "Test Profile 2").await;

    // Create account in profile_2
    let account_id =
        create_test_account(&client, &token_2, "Test Account", BalanceDirection::Debit).await;

    // Try to create ingestion config in profile_1 with account from profile_2
    let request = IngestionConfigCreateRequest {
        merchant_id: merchant_id_1,
        profile_id: profile_id_1,
        name: "Test Config".to_string(),
        description: None,
        account_id,
        config: create_adyen_config(),
        is_active: true,
    };

    let response = client
        .post("/ingestions/config")
        .with_admin_key()
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
async fn sftp_internal_manual_upload_not_supported() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and SFTP Internal config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let request = IngestionConfigCreateRequest {
        merchant_id: merchant_id.clone(),
        profile_id: profile_id.to_string(),
        name: "Test SFTP Internal".to_string(),
        description: None,
        account_id: account_id.to_string(),
        config: create_sftp_internal_config(),
        is_active: true,
    };

    let create_response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(
        create_response.status(),
        201,
        "Failed to create SFTP Internal config"
    );
    let created: IngestionConfigResponse = client.expect_success(create_response).await;

    // Try to upload a file manually (should fail)
    let file_content = b"test,data\n1,2";
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.csv\"\r\n\
         Content-Type: text/csv\r\n\r\n\
         {}\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        std::str::from_utf8(file_content).unwrap()
    );

    let response = client
        .post(&format!("/ingestions/{}/upload", created.ingestion_id))
        .with_auth(&token)
        .header(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .body(&body)
        .send()
        .await;

    // SFTP Internal doesn't support manual upload
    assert_eq!(
        response.status(),
        400,
        "Expected 400 for unsupported manual upload"
    );
}

// ========== Download Ingested File Tests ==========

#[actix_web::test]
async fn download_ingested_file_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Setup account and manual ingestion config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let config = create_test_ingestion_config(
        &client,
        &merchant_id,
        &profile_id,
        &account_id,
        IngestionConfigData::Manual,
        "Test Manual Upload",
    )
    .await;

    // Upload a test file
    let file_content = b"test,data\n1,2\n3,4";
    let file_name = "test_report.csv";
    upload_test_file(
        &client,
        &token,
        &config.ingestion_id,
        file_content,
        file_name,
    )
    .await;

    // Get the history to find the history_id
    let history_response = client
        .get("/ingestions/history")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history_list: IngestionHistoryListResponse = client.expect_success(history_response).await;
    assert!(!history_list.is_empty(), "Should have ingestion history");

    // Find the processed entry (latest non-discarded version)
    let processed_ingestion_history_entry = history_list
        .iter()
        .find(|h| h.status == common_enums::IngestionStatus::Processed && h.discarded_at.is_none())
        .expect("Should have a processed entry");

    // Download the file
    let download_response = client
        .get(&format!(
            "/ingestions/history/{}/download",
            processed_ingestion_history_entry.id
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        download_response.status(),
        200,
        "File download should succeed"
    );

    // Verify headers
    let headers = download_response.headers();
    assert_eq!(
        headers.get("content-type").unwrap(),
        "application/octet-stream",
        "Content-Type should be application/octet-stream"
    );

    let content_disposition = headers
        .get("content-disposition")
        .expect("Should have Content-Disposition header");
    assert!(
        content_disposition.to_str().unwrap().contains("attachment"),
        "Should be an attachment"
    );
    assert!(
        content_disposition.to_str().unwrap().contains(file_name),
        "Should include original filename"
    );

    // Verify file content
    let downloaded_content = download_response.bytes().await.unwrap();
    assert_eq!(
        downloaded_content.as_ref(),
        file_content,
        "Downloaded content should match uploaded content"
    );
}

#[actix_web::test]
async fn download_ingested_file_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to download with non-existent history_id
    let response = client
        .get("/ingestions/history/non_existent_id/download")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        response.status(),
        404,
        "Should return 404 for non-existent history"
    );
}

#[actix_web::test]
async fn download_ingested_file_wrong_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Create two separate merchants with different profiles
    let (merchant_id_1, profile_id_1, token_1) = client.bootstrap("Merchant 1", "Profile 1").await;
    let (_merchant_id_2, _profile_id_2, token_2) =
        client.bootstrap("Merchant 2", "Profile 2").await;

    // Setup account and config for merchant 1
    let account_id =
        create_test_account(&client, &token_1, "Test Account", BalanceDirection::Debit).await;
    let config = create_test_ingestion_config(
        &client,
        &merchant_id_1,
        &profile_id_1,
        &account_id,
        IngestionConfigData::Manual,
        "Test Manual Upload",
    )
    .await;

    // Upload a file for merchant 1
    let file_content = b"test,data\n1,2";
    upload_test_file(
        &client,
        &token_1,
        &config.ingestion_id,
        file_content,
        "test.csv",
    )
    .await;

    // Get history for merchant 1
    let history_response = client
        .get("/ingestions/history")
        .with_auth(&token_1)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: IngestionHistoryListResponse = client.expect_success(history_response).await;

    let processed_entry = history
        .iter()
        .find(|h| h.status == common_enums::IngestionStatus::Processed && h.discarded_at.is_none())
        .expect("Should have processed entry");

    // Try to download using merchant 2's token (should fail)
    let download_response = client
        .get(&format!(
            "/ingestions/history/{}/download",
            processed_entry.id
        ))
        .with_auth(&token_2)
        .send()
        .await;

    assert_eq!(
        download_response.status(),
        404,
        "Should return 404 for security (don't leak existence)"
    );
}

#[actix_web::test]
async fn download_ingested_file_not_processed() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Setup account and Adyen config
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let config = create_test_ingestion_config(
        &client,
        &merchant_id,
        &profile_id,
        &account_id,
        create_adyen_config(),
        "Test Adyen Integration",
    )
    .await;

    // Trigger ingestion with invalid HMAC to create a failed entry
    let mut webhook_payload = create_test_webhook_payload();
    webhook_payload.notification_items[0]
        .notification_request_item
        .additional_data
        .hmac_signature = "invalid_signature".to_string();

    let trigger_response = client
        .post(&format!("/ingestions/{}/trigger", config.ingestion_id))
        .header(
            "Authorization",
            &create_basic_auth_header("webhook_user", "webhook_pass"),
        )
        .json(&webhook_payload)
        .send()
        .await;
    assert_eq!(trigger_response.status(), 400);

    // Get the failed history entry
    let history_response = client
        .get("/ingestions/history")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: IngestionHistoryListResponse = client.expect_success(history_response).await;

    let failed_entry = history
        .iter()
        .find(|h| h.status == common_enums::IngestionStatus::Failed && h.discarded_at.is_none())
        .expect("Should have a failed entry");

    // Try to download the failed entry
    let download_response = client
        .get(&format!("/ingestions/history/{}/download", failed_entry.id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(
        download_response.status(),
        404,
        "Should return 404 for failed ingestion (file not available)"
    );
}

#[actix_web::test]
async fn test_find_ingestion_history_is_scoped_to_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Create two separate merchants with different profiles
    let (merchant_id_1, profile_id_1, token_1) = client.bootstrap("Merchant 1", "Profile 1").await;
    let (_merchant_id_2, _profile_id_2, token_2) =
        client.bootstrap("Merchant 2", "Profile 2").await;

    // Setup account and config for merchant 1, then upload a file to produce history
    let account_id =
        create_test_account(&client, &token_1, "Test Account", BalanceDirection::Debit).await;
    let config = create_test_ingestion_config(
        &client,
        &merchant_id_1,
        &profile_id_1,
        &account_id,
        IngestionConfigData::Manual,
        "Test Manual Upload",
    )
    .await;

    upload_test_file(
        &client,
        &token_1,
        &config.ingestion_id,
        b"test,data\n1,2",
        "test.csv",
    )
    .await;

    let history_response = client
        .get("/ingestions/history")
        .with_auth(&token_1)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: IngestionHistoryListResponse = client.expect_success(history_response).await;

    let processed_entry = history
        .iter()
        .find(|h| h.status == common_enums::IngestionStatus::Processed && h.discarded_at.is_none())
        .expect("Should have processed entry");

    // Owner can read it
    let owner_response = client
        .get(&format!("/ingestions/history/{}", processed_entry.id))
        .with_auth(&token_1)
        .send()
        .await;

    assert_eq!(
        owner_response.status(),
        200,
        "Owning profile should be able to read its own ingestion history"
    );

    // Another profile must not
    let cross_tenant_response = client
        .get(&format!("/ingestions/history/{}", processed_entry.id))
        .with_auth(&token_2)
        .send()
        .await;

    assert_eq!(
        cross_tenant_response.status(),
        404,
        "another profile's ingestion history id must not resolve"
    );
}

#[actix_web::test]
async fn find_ingestion_history_with_config_by_id_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Try to find with non-existent history_id
    let response = client
        .get("/ingestions/history/non_existent_id")
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

/// Creates a config of the given type and returns `(ingestion_id, owner_token)`.
async fn setup_trigger_config(
    client: &crate::common::TestClient,
    config: IngestionConfigData,
    is_active: bool,
) -> (String, String) {
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let account_id =
        create_test_account(client, &token, "Test Account", BalanceDirection::Debit).await;

    let request = IngestionConfigCreateRequest {
        merchant_id,
        profile_id,
        name: "Trigger Auth Test".to_string(),
        description: None,
        account_id,
        config,
        is_active,
    };
    let response = client
        .post("/ingestions/config")
        .with_admin_key()
        .json(&request)
        .send()
        .await;
    assert_eq!(response.status(), 201, "Failed to create ingestion config");
    let created: IngestionConfigResponse = client.expect_success(response).await;
    (created.ingestion_id, token)
}

async fn assert_failed_ingestion_history(client: &crate::common::TestClient, token: &str) {
    let response = client
        .get("/ingestions/history")
        .with_auth(token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let history: IngestionHistoryListResponse = client.expect_success(response).await;
    assert!(
        history
            .iter()
            .any(|entry| entry.status == common_enums::IngestionStatus::Failed),
        "A trigger rejected at the parse step must be recorded as a failed ingestion history"
    );
}

#[actix_web::test]
async fn trigger_sftp_without_internal_key_is_rejected() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (ingestion_id, token) =
        setup_trigger_config(&client, create_sftp_internal_config(), true).await;

    let response = client
        .post(&format!("/ingestions/{ingestion_id}/trigger"))
        .json(&serde_json::json!({}))
        .send()
        .await;

    // Auth runs in the parse step, so the rejection is a recorded parse failure
    assert_eq!(response.status(), 400);
    assert_failed_ingestion_history(&client, &token).await;
}

#[actix_web::test]
async fn trigger_sftp_with_internal_key_passes_auth() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (ingestion_id, token) =
        setup_trigger_config(&client, create_sftp_internal_config(), true).await;

    let response = client
        .post(&format!("/ingestions/{ingestion_id}/trigger"))
        .with_internal_key()
        .json(&serde_json::json!({}))
        .send()
        .await;

    // The file is not in the test bucket, so a download failure is expected, but not an auth failure.
    assert_ne!(response.status(), 401, "Valid internal key must pass auth");

    let history_response = client
        .get("/ingestions/history")
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(history_response.status(), 200);
    let history: IngestionHistoryListResponse = client.expect_success(history_response).await;
    assert!(
        !history.is_empty(),
        "An authenticated SFTP trigger should reach processing and record history"
    );
}
