use crate::common::TestApp;
use api_models::merchant_account::{MerchantAccountCreateRequest, MerchantAccountResponse};
use masking::{ExposeInterface, Secret};
use pretty_assertions::assert_eq;
use rstest::rstest;

// Local test fixtures
fn merchant_request(name: &str) -> MerchantAccountCreateRequest {
    MerchantAccountCreateRequest {
        merchant_name: Secret::new(name.to_string()),
    }
}

#[actix_web::test]
async fn create_merchant_account_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let request = merchant_request("Acme Corp");

    let response = client
        .post("/merchant_accounts")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);

    let body: MerchantAccountResponse = response.json().await.unwrap();
    assert!(body.merchant_id.get_string_repr().starts_with("merchant_"));
    assert_eq!(body.merchant_name.get_inner().clone().expose(), "Acme Corp");
}

#[actix_web::test]
async fn get_merchant_account_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Use bootstrap to create both merchant and profile, then access that merchant
    let (merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Test Profile")
        .await;

    let get_resp = client
        .get(&format!(
            "/merchant_accounts/{}",
            merchant_id.get_string_repr()
        ))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(get_resp.status(), 200);

    let body: MerchantAccountResponse = get_resp.json().await.unwrap();
    assert_eq!(body.merchant_id, merchant_id);
    assert_eq!(
        body.merchant_name.get_inner().clone().expose(),
        "Test Merchant"
    );
}

#[actix_web::test]
async fn get_merchant_account_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Act - try to access a non-existent merchant without authentication
    let response = client
        .get("/merchant_accounts/non_existent_merchant")
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 401); // Should return 401 when no authentication provided
}

#[actix_web::test]
async fn create_multiple_merchants_with_same_name_succeeds() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let request = merchant_request("Test Merchant");

    let resp1 = client
        .post("/merchant_accounts")
        .with_admin_key()
        .json(&request)
        .send()
        .await;
    assert_eq!(resp1.status(), 201);
    let merchant1: MerchantAccountResponse = resp1.json().await.unwrap();

    let resp2 = client
        .post("/merchant_accounts")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(resp2.status(), 201);
    let merchant2: MerchantAccountResponse = resp2.json().await.unwrap();

    assert_ne!(merchant1.merchant_id, merchant2.merchant_id);
    assert_eq!(merchant1.merchant_name, merchant2.merchant_name);
}

#[rstest]
#[case("", 400)]
#[case("a", 201)]
#[case("Test Merchant 123", 201)]
#[actix_web::test]
async fn create_merchant_account_validation(
    #[case] merchant_name: &str,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    let request = MerchantAccountCreateRequest {
        merchant_name: Secret::new(merchant_name.to_string()),
    };

    let response = client
        .post("/merchant_accounts")
        .with_admin_key()
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);
}

#[actix_web::test]
async fn merchant_account_lifecycle() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Use bootstrap to create both merchant and profile, then access that merchant
    let (merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Lifecycle Test Merchant", "Lifecycle Profile")
        .await;

    // Verify the created merchant can be retrieved
    let get_resp = client
        .get(&format!(
            "/merchant_accounts/{}",
            merchant_id.get_string_repr()
        ))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(get_resp.status(), 200);

    let retrieved: MerchantAccountResponse = get_resp.json().await.unwrap();
    assert_eq!(retrieved.merchant_id.clone(), merchant_id);
    assert_eq!(
        retrieved.merchant_name.get_inner().clone().expose(),
        "Lifecycle Test Merchant"
    );
}

#[actix_web::test]
async fn get_merchant_account_without_auth_fails() {
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

    let created: MerchantAccountResponse = create_resp.json().await.unwrap();

    // Act - Try to get merchant without JWT token
    let response = client
        .get(&format!(
            "/merchant_accounts/{}",
            created.merchant_id.get_string_repr()
        ))
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 401); // Should return 401 Unauthorized
}

#[actix_web::test]
async fn create_merchant_account_without_admin_key_fails() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let request = merchant_request("Test Merchant");

    let response = client
        .post("/merchant_accounts")
        .json(&request)
        .send()
        .await;

    // Should return 401 Unauthorized when admin API key is missing
    assert_eq!(response.status(), 401);
}

#[actix_web::test]
async fn get_merchant_account_with_wrong_merchant_id_fails() {
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
    let merchant_a: MerchantAccountResponse = create_resp_a.json().await.unwrap();

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

    // Act - Try to get merchant A's data with merchant B's JWT token
    let (_setup_merchant_id, _profile_id, token_b) = app
        .api_client()
        .bootstrap("Auth Merchant B", "Auth Profile B")
        .await;
    let response = client
        .get(&format!(
            "/merchant_accounts/{}",
            merchant_a.merchant_id.get_string_repr()
        ))
        .with_auth(&token_b)
        .send()
        .await;

    // Assert
    assert_eq!(response.status(), 401); // Should return 401 Unauthorized due to merchant_id mismatch
}
