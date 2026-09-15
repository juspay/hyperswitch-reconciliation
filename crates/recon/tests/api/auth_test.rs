use api_models::auth::{
    TestMerchantSetupRequest, TestMerchantSetupResponse, TestMerchantSetupWithIdsRequest,
};
use common_utils::{id_type::MerchantId, GenerateId};
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common;

#[actix_web::test]
async fn bootstrap_without_admin_key_fails() {
    // Arrange
    let app = common::TestApp::new().await;
    let client = app.api_client();

    let request = TestMerchantSetupRequest {
        merchant_name: "Test Merchant".to_string(),
        profile_name: "Test Profile".to_string(),
    };

    // Act - Try to bootstrap without admin API key
    let response = client.post("/auth/bootstrap").json(&request).send().await;

    // Assert
    assert_eq!(response.status(), 401); // Should return 401 Unauthorized
}

#[actix_web::test]
async fn bootstrap_success() {
    // Arrange
    let app = common::TestApp::new().await;
    let client = app.api_client();

    let request = TestMerchantSetupRequest {
        merchant_name: "Test Merchant".to_string(),
        profile_name: "Test Profile".to_string(),
    };

    // Act
    let response = client
        .post("/auth/bootstrap")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    // Assert
    let setup_response: TestMerchantSetupResponse = client.expect_success(response).await;

    assert!(setup_response
        .merchant_id
        .get_string_repr()
        .starts_with("merchant_"));
    assert!(setup_response.profile_id.starts_with("profile_"));
}

#[actix_web::test]
async fn bootstrap_with_merchant_id_profile_id_success() {
    // Arrange
    let app = common::TestApp::new().await;
    let client = app.api_client();

    // Generate unique IDs for this test run to avoid database collisions
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let merchant_id = MerchantId::generate();
    let profile_id = format!("profile_abcdefghij_{unique_suffix}");

    let request = TestMerchantSetupWithIdsRequest {
        merchant_id: merchant_id.clone(),
        merchant_name: "Test Merchant from Hyperswitch".to_string(),
        profile_id: profile_id.clone(),
        profile_name: "Test Profile from Hyperswitch".to_string(),
    };

    // Act
    let response = client
        .post("/auth/bootstrap_with_ids")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    // Assert
    let setup_response: TestMerchantSetupResponse = client.expect_success(response).await;

    // Verify the IDs match what we provided
    assert_eq!(setup_response.merchant_id, merchant_id);
    assert_eq!(setup_response.profile_id, profile_id);
}

#[actix_web::test]
async fn bootstrap_with_merchant_id_profile_id_without_admin_key_fails() {
    // Arrange
    let app = common::TestApp::new().await;
    let client = app.api_client();

    let request = TestMerchantSetupWithIdsRequest {
        merchant_id: MerchantId::generate(),
        merchant_name: "Test Merchant".to_string(),
        profile_id: "profile_abcdefghij".to_string(),
        profile_name: "Test Profile".to_string(),
    };

    // Act - Try to bootstrap without admin API key
    let response = client
        .post("/auth/bootstrap_with_ids")
        .json(&request)
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 401); // Should return 401 Unauthorized
}

#[actix_web::test]
async fn bootstrap_with_merchant_id_profile_id_create_profile_in_existing_merchant() {
    // Arrange
    let app = common::TestApp::new().await;
    let client = app.api_client();

    // Generate unique IDs for this test run to avoid database collisions
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let merchant_id = MerchantId::generate();

    // First, create a merchant with specific ID
    let request1 = TestMerchantSetupWithIdsRequest {
        merchant_id: merchant_id.clone(),
        merchant_name: "Test Merchant".to_string(),
        profile_id: format!("profile_first_{unique_suffix}"),
        profile_name: "First Profile".to_string(),
    };

    let response1 = client
        .post("/auth/bootstrap_with_ids")
        .with_admin_key()
        .json(&request1)
        .send()
        .await;

    // Verify first creation succeeds
    let _setup_response: TestMerchantSetupResponse = client.expect_success(response1).await;

    // Now create another profile in the same merchant
    let request2 = TestMerchantSetupWithIdsRequest {
        merchant_id: merchant_id.clone(), // Same merchant_id
        merchant_name: "Test Merchant".to_string(),
        profile_id: format!("profile_second_{unique_suffix}"),
        profile_name: "Second Profile".to_string(),
    };

    // Act
    let response2 = client
        .post("/auth/bootstrap_with_ids")
        .with_admin_key()
        .json(&request2)
        .send()
        .await;

    // Assert - Should succeed, creating a new profile in existing merchant
    let setup_response: TestMerchantSetupResponse = client.expect_success(response2).await;
    assert_eq!(setup_response.merchant_id, merchant_id);
    assert_eq!(
        setup_response.profile_id,
        format!("profile_second_{unique_suffix}")
    );
}

#[actix_web::test]
async fn bootstrap_with_merchant_id_profile_id_both_exist_fails() {
    // Arrange
    let app = common::TestApp::new().await;
    let client = app.api_client();

    // Generate unique IDs for this test run to avoid database collisions
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let merchant_id = MerchantId::generate();
    let profile_id = format!("profile_both_{unique_suffix}");

    // First, create a merchant and profile
    let request1 = TestMerchantSetupWithIdsRequest {
        merchant_id: merchant_id.clone(),
        merchant_name: "Test Merchant".to_string(),
        profile_id: profile_id.clone(),
        profile_name: "Test Profile".to_string(),
    };

    let response1 = client
        .post("/auth/bootstrap_with_ids")
        .with_admin_key()
        .json(&request1)
        .send()
        .await;

    // Verify first creation succeeds
    let _setup_response: TestMerchantSetupResponse = client.expect_success(response1).await;

    // Now try to create with the same merchant_id and profile_id
    let request2 = TestMerchantSetupWithIdsRequest {
        merchant_id: merchant_id.clone(),
        merchant_name: "Test Merchant".to_string(),
        profile_id: profile_id.clone(),
        profile_name: "Test Profile".to_string(),
    };

    // Act
    let response2 = client
        .post("/auth/bootstrap_with_ids")
        .with_admin_key()
        .json(&request2)
        .send()
        .await;

    // Assert - Should fail due to both merchant and profile already existing
    let error_message = client.expect_error(response2, 409).await;
    assert!(error_message.contains("already exist"));
}

#[rstest]
#[case("merchant_123", "", "profile_123", "Test Profile")] // Empty merchant_name
#[case("merchant_123", "Test Merchant", "", "Test Profile")] // Empty profile_id
#[case("merchant_123", "Test Merchant", "profile_123", "")] // Empty profile_name
#[actix_web::test]
async fn bootstrap_with_merchant_id_profile_id_validation_errors(
    #[case] merchant_id: &str,
    #[case] merchant_name: &str,
    #[case] profile_id: &str,
    #[case] profile_name: &str,
) {
    // Arrange
    let app = common::TestApp::new().await;
    let client = app.api_client();

    let request = TestMerchantSetupWithIdsRequest {
        merchant_id: MerchantId::wrap(merchant_id.to_string()).unwrap(),
        merchant_name: merchant_name.to_string(),
        profile_id: profile_id.to_string(),
        profile_name: profile_name.to_string(),
    };

    // Act
    let response = client
        .post("/auth/bootstrap_with_ids")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    // Assert - Should fail validation
    assert_eq!(response.status(), 400);
}
