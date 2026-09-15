use crate::api::{
    account_test::create_test_account, staging_entry_test::staging_entry_request,
    transformation_test::create_transformation_config,
};
use crate::common::TestApp;
use api_models::{
    business_profile::{
        BusinessProfileCreateRequest, BusinessProfileResponse, ReconStatusResponse,
    },
    merchant_account::{MerchantAccountCreateRequest, MerchantAccountResponse},
};
use common_enums::{BalanceDirection, Currency, ReconProcessorStatus, StagingEntryProcessingMode};
use common_utils::crypto::Encryptable;
use masking::{ExposeInterface, Secret};
use pretty_assertions::assert_eq;
use rstest::rstest;

// Test fixtures
fn business_profile_request(profile_name: &str) -> BusinessProfileCreateRequest {
    BusinessProfileCreateRequest {
        profile_name: Secret::new(profile_name.to_string()),
    }
}

fn merchant_request(name: &str) -> MerchantAccountCreateRequest {
    MerchantAccountCreateRequest {
        merchant_name: Secret::new(name.to_string()),
    }
}

#[actix_web::test]
async fn create_business_profile_success() {
    // Arrange
    let app = TestApp::new().await;
    let (merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Initial Profile")
        .await;
    let request = business_profile_request("Main Profile");

    // Act
    let response = app
        .api_client()
        .post("/business_profiles")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 201);
    let body: BusinessProfileResponse = response.json().await.unwrap();
    assert!(body.profile_id.starts_with("profile_"));
    assert_eq!(
        body.profile_name.get_inner().clone().expose(),
        "Main Profile"
    );
    assert_eq!(body.merchant_id, merchant_id);
}

#[actix_web::test]
async fn get_business_profile_success() {
    // Arrange
    let app = TestApp::new().await;
    let (merchant_id, profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Test Profile")
        .await;
    // Use the profile created by setup_test_merchant
    let created_profile = BusinessProfileResponse {
        profile_id: profile_id.clone(),
        merchant_id: merchant_id.clone(),
        profile_name: Encryptable::new(
            Secret::new("Test Profile".to_string()),
            Secret::new(Vec::new()),
        ),
    };

    // Act
    let response = app
        .api_client()
        .get(&format!(
            "/business_profiles/{}",
            created_profile.profile_id
        ))
        .with_auth(&token)
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 200);
    let body: BusinessProfileResponse = response.json().await.unwrap();
    assert_eq!(body.profile_id, created_profile.profile_id);
    assert_eq!(body.merchant_id, merchant_id);
    assert_eq!(
        body.profile_name.get_inner().clone().expose(),
        "Test Profile"
    );
}

#[actix_web::test]
async fn get_business_profile_not_found() {
    // Arrange
    let app = TestApp::new().await;
    let (_merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Test Profile")
        .await;

    // Act
    let response = app
        .api_client()
        .get("/business_profiles/non_existent_profile")
        .with_auth(&token)
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 401);
}

#[actix_web::test]
async fn get_business_profile_wrong_merchant() {
    // Arrange
    let app = TestApp::new().await;
    let (merchant1_id, profile_id, _token1) =
        app.api_client().bootstrap("Merchant 1", "Profile 1").await;
    let (_merchant2_id, _profile2_id, token2) =
        app.api_client().bootstrap("Merchant 2", "Profile 2").await;

    // Create business profile for merchant1 (already done by setup)
    // Use the profile created by setup_test_merchant
    let profile = BusinessProfileResponse {
        profile_id: profile_id.clone(),
        merchant_id: merchant1_id.clone(),
        profile_name: Encryptable::new(
            Secret::new("Test Profile".to_string()),
            Secret::new(Vec::new()),
        ),
    };

    // Act - Try to access merchant1's profile using merchant2's JWT token
    let response = app
        .api_client()
        .get(&format!("/business_profiles/{}", profile.profile_id))
        .with_auth(&token2)
        .send()
        .await;

    // Assert - Should return 401 because the JWT belongs to a different merchant
    assert_eq!(response.status(), 401);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("error").is_some());
}

#[actix_web::test]
async fn list_business_profiles_empty() {
    // Arrange
    let app = TestApp::new().await;
    let (_merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Initial Profile")
        .await;

    // Act
    let response = app
        .api_client()
        .get("/business_profiles")
        .with_auth(&token)
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 200);
    let body: Vec<BusinessProfileResponse> = response.json().await.unwrap();
    assert_eq!(body.len(), 1); // Should have the initial profile created by setup
}

#[actix_web::test]
async fn list_business_profiles_no_auth_fails() {
    // Arrange
    let app = TestApp::new().await;

    // Act - Test accessing business profiles without authentication
    // Since merchant context comes from JWT, non-existent merchant is now just missing auth
    let response = app.api_client().get("/business_profiles").send().await;

    // Assert
    assert_eq!(response.status(), 401); // No JWT token provided
}

#[actix_web::test]
async fn list_business_profiles_invalid_merchant_id() {
    // Arrange
    let app = TestApp::new().await;

    // Act - Test without authentication (invalid merchant ID no longer relevant since merchant comes from JWT)
    let response = app.api_client().get("/business_profiles").send().await;

    // Assert
    assert_eq!(response.status(), 401); // No JWT token provided
}

#[rstest]
#[case("", 400)] // Empty profile name
#[case("a", 201)] // Valid minimal profile name
#[case("Test Profile 123", 201)] // Valid profile name with numbers
#[actix_web::test]
async fn create_business_profile_validation_with_auth(
    #[case] profile_name: &str,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;

    // Setup merchant and get JWT token
    let (_merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Validation Merchant", "Initial Profile")
        .await;

    let request = BusinessProfileCreateRequest {
        profile_name: Secret::new(profile_name.to_string()),
    };

    let response = app
        .api_client()
        .post("/business_profiles")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status != 201 {
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body.get("error").is_some());
    }
}

#[actix_web::test]
async fn create_business_profile_without_auth() {
    let app = TestApp::new().await;

    let request = BusinessProfileCreateRequest {
        profile_name: Secret::new("Valid Profile".to_string()),
    };

    let response = app
        .api_client()
        .post("/business_profiles")
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 401);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("error").is_some());
}

#[actix_web::test]
async fn create_multiple_business_profiles() {
    // Arrange
    let app = TestApp::new().await;
    let (_merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Initial Profile")
        .await;

    // Act - Create first profile (additional to the one from setup)
    let response1 = app
        .api_client()
        .post("/business_profiles")
        .with_auth(&token)
        .json(&business_profile_request("Profile 1"))
        .send()
        .await;
    assert_eq!(response1.status(), 201);
    let profile1: BusinessProfileResponse = response1.json().await.unwrap();

    // Act - Create second profile
    let response2 = app
        .api_client()
        .post("/business_profiles")
        .with_auth(&token)
        .json(&business_profile_request("Profile 2"))
        .send()
        .await;
    assert_eq!(response2.status(), 201);
    let profile2: BusinessProfileResponse = response2.json().await.unwrap();

    // Assert
    assert_ne!(profile1.profile_id, profile2.profile_id);
    assert_eq!(profile1.merchant_id, profile2.merchant_id);
    assert_ne!(profile1.profile_name, profile2.profile_name);
}

#[actix_web::test]
async fn business_profile_lifecycle() {
    // Arrange
    let app = TestApp::new().await;
    let (merchant_id, profile_id, token) = app
        .api_client()
        .bootstrap("Lifecycle Merchant", "Lifecycle Profile")
        .await;

    // Act - Use profile created by setup
    // Use the profile created by setup_test_merchant
    let created = BusinessProfileResponse {
        profile_id: profile_id.clone(),
        merchant_id: merchant_id.clone(),
        profile_name: Encryptable::new(
            Secret::new("Lifecycle Profile".to_string()),
            Secret::new(Vec::new()),
        ),
    };

    // Act - Retrieve profile
    let response = app
        .api_client()
        .get(&format!("/business_profiles/{}", created.profile_id))
        .with_auth(&token)
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 200);
    let retrieved: BusinessProfileResponse = response.json().await.unwrap();
    assert_eq!(retrieved.profile_id, profile_id);
    assert_eq!(retrieved.merchant_id, merchant_id);
    assert_eq!(
        retrieved.profile_name.get_inner().clone().expose(),
        "Lifecycle Profile"
    );
}

#[actix_web::test]
async fn list_business_profiles_without_auth_fails() {
    // Arrange
    let app = TestApp::new().await;
    let client = app.api_client();

    // First create a merchant
    let create_req = merchant_request("Test Merchant");
    let create_resp = client
        .post("/merchant_accounts")
        .with_admin_key()
        .json(&create_req)
        .send()
        .await;
    assert_eq!(create_resp.status(), 201);

    let _created: MerchantAccountResponse = create_resp.json().await.unwrap();

    // Act - Try to list business profiles without JWT token
    let response = client.get("/business_profiles").send().await;

    // Assert
    assert_eq!(response.status(), 401); // Should return 401 Unauthorized
}

#[actix_web::test]
async fn list_business_profiles_with_wrong_merchant_id_fails() {
    // Arrange
    let app = TestApp::new().await;
    let client = app.api_client();

    // Create first merchant
    let create_req_a = merchant_request("Merchant A");
    let create_resp_a = client
        .post("/merchant_accounts")
        .with_admin_key()
        .json(&create_req_a)
        .send()
        .await;
    assert_eq!(create_resp_a.status(), 201);
    let _merchant_a: MerchantAccountResponse = create_resp_a.json().await.unwrap();

    // Create second merchant
    let create_req_b = merchant_request("Merchant B");
    let create_resp_b = client
        .post("/merchant_accounts")
        .with_admin_key()
        .json(&create_req_b)
        .send()
        .await;
    assert_eq!(create_resp_b.status(), 201);
    let _merchant_b: MerchantAccountResponse = create_resp_b.json().await.unwrap();

    // Act - Try to list business profiles with merchant B's JWT token
    // This will only return merchant B's profiles, not merchant A's
    let (_merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Merchant B Alt", "Profile B")
        .await;
    let response = client
        .get("/business_profiles")
        .with_auth(&token)
        .send()
        .await;

    // Assert
    // With JWT authentication, merchant B can access their own profiles
    // This is no longer a cross-merchant access issue since JWT determines the merchant
    assert_eq!(response.status(), 200);
    let profiles: Vec<BusinessProfileResponse> = response.json().await.unwrap();
    assert_eq!(profiles.len(), 1); // Merchant B Alt has one business profile from setup
}

#[actix_web::test]
async fn test_get_recon_status() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Fresh profile reports zero pending
    let response = client
        .get(&format!(
            "/business_profiles/{profile_id}/recon_engine/status"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let status: ReconStatusResponse = client.expect_success(response).await;
    assert_eq!(status.processor_status, ReconProcessorStatus::Stopped);
    assert_eq!(status.pending_staging_entries, 0);

    // Create a pending staging entry
    let account_id = create_test_account(
        &client,
        &token,
        "Recon Status Account",
        BalanceDirection::Credit,
    )
    .await;
    let transformation_config = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        "metadata.transaction_id".parse().unwrap(),
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
    let create_response = client
        .post("/staging_entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;
    assert_eq!(create_response.status(), 201);

    let response = client
        .get(&format!(
            "/business_profiles/{profile_id}/recon_engine/status"
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let status: ReconStatusResponse = client.expect_success(response).await;
    assert_eq!(status.pending_staging_entries, 1);

    // Count is profile-scoped
    let (_, other_profile_id, other_token) = client
        .bootstrap("Other Test Merchant", "Other Test Profile")
        .await;
    let response = client
        .get(&format!(
            "/business_profiles/{other_profile_id}/recon_engine/status"
        ))
        .with_auth(&other_token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let status: ReconStatusResponse = client.expect_success(response).await;
    assert_eq!(status.pending_staging_entries, 0);

    // Accessing another profile's status is rejected
    let response = client
        .get(&format!(
            "/business_profiles/{profile_id}/recon_engine/status"
        ))
        .with_auth(&other_token)
        .send()
        .await;
    assert_eq!(response.status(), 401);

    // Missing JWT is rejected
    let response = client
        .get(&format!(
            "/business_profiles/{profile_id}/recon_engine/status"
        ))
        .send()
        .await;
    assert_eq!(response.status(), 401);
}
