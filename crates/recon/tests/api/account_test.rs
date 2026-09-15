use crate::common::{TestApp, TestClient};
use api_models::account::{AccountCreateRequest, AccountUpdateRequest, GetAccountResponse};
use common_enums::{BalanceDirection, Currency};
use common_types::units::FloatMajorUnit;
use pretty_assertions::assert_eq;
use rstest::rstest;

/// # Panics
/// Panics if the HTTP response cannot be deserialized as JSON.
pub async fn create_test_account(
    client: &TestClient,
    token: &str,
    name: &str,
    balance_direction: BalanceDirection,
) -> String {
    use api_models::account::{AccountCreateRequest, GetAccountResponse};

    let account_request = AccountCreateRequest {
        account_name: name.to_string(),
        account_type: balance_direction,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(1000.0),
    };

    let response = client
        .post("/accounts")
        .with_auth(token)
        .json(&account_request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let account: GetAccountResponse = response.json().await.unwrap();
    account.account_id
}

// Local test fixtures
fn account_request(name: &str) -> AccountCreateRequest {
    AccountCreateRequest {
        account_name: name.to_string(),
        account_type: BalanceDirection::Debit,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(1000.0),
    }
}

#[rstest]
#[case("Checking Account", 201)] // Valid account name
#[case("", 400)] // Empty account name - should fail validation
#[case("a", 201)] // Valid minimal account name
#[case("Test Account 123", 201)] // Valid account name with numbers
#[actix_web::test]
async fn create_account_success_and_validation(
    #[case] account_name: &str,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile first
    let (_merchant_id, profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Test Profile")
        .await;

    let request = account_request(account_name);

    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    // Only check response body for successful cases
    if expected_status == 201 {
        let body: GetAccountResponse = response.json().await.unwrap();
        assert!(body.account_id.starts_with("account_"));
        assert_eq!(body.account_name, account_name);
        assert_eq!(body.account_type, BalanceDirection::Debit);
        assert_eq!(body.profile_id, profile_id);
        assert_eq!(body.initial_balance.value.0, 1000.0);
        assert_eq!(body.initial_balance.currency, Currency::USD);
    }
}

#[actix_web::test]
async fn get_account_success() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap and create account
    let (_merchant_id, profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Test Profile")
        .await;

    let create_request = account_request("Savings Account");
    let create_response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&create_request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    let created_account: GetAccountResponse = create_response.json().await.unwrap();

    // Get the account using the correct route
    let get_response = client
        .get(&format!("/accounts/{}", created_account.account_id))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(get_response.status(), 200);

    let retrieved_account: GetAccountResponse = get_response.json().await.unwrap();
    assert_eq!(retrieved_account.account_id, created_account.account_id);
    assert_eq!(retrieved_account.account_name, "Savings Account");
    assert_eq!(retrieved_account.profile_id, profile_id);
}

#[actix_web::test]
async fn list_accounts_by_profile_id_success() {
    let app = TestApp::new().await;
    let client = app.api_client();
    // Bootstrap and create account
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    // Create one account
    let account_request = account_request("Test Account");

    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&account_request)
        .send()
        .await;
    assert_eq!(response.status(), 201);

    // List accounts by profile_id
    let list_response = client.get("/accounts").with_auth(&token).send().await;

    assert_eq!(list_response.status(), 200);

    let accounts: Vec<GetAccountResponse> = list_response.json().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account_name, "Test Account");
}

#[rstest]
#[case(Some("Updated Account Name".to_string()), 200)]
#[case(None, 200)]
#[case(Some("".to_string()), 400)]
#[case(Some("   ".to_string()), 400)]
#[actix_web::test]
async fn update_account_success_and_validation(
    #[case] account_name: Option<String>,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap and create account
    let (_merchant_id, profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Test Profile")
        .await;

    let create_request = account_request("Original Account Name");
    let create_response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&create_request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    let created_account: GetAccountResponse = create_response.json().await.unwrap();

    // Update the account
    let update_request = AccountUpdateRequest { account_name };
    let update_response = client
        .put(&format!("/accounts/{}", created_account.account_id))
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), expected_status);

    // Only check response body for successful cases
    if expected_status == 200 {
        let updated_account: GetAccountResponse = update_response.json().await.unwrap();
        assert_eq!(updated_account.account_id, created_account.account_id);
        assert_eq!(updated_account.profile_id, profile_id);
        assert_eq!(updated_account.account_type, BalanceDirection::Debit);

        // Check the account name based on what was provided
        match &update_request.account_name {
            Some(name) if !name.trim().is_empty() => {
                assert_eq!(updated_account.account_name, *name);
            }
            _ => {
                // If None was provided, the name should remain unchanged
                assert_eq!(updated_account.account_name, "Original Account Name");
            }
        }

        // Balance should remain unchanged
        assert_eq!(updated_account.initial_balance.value.0, 1000.0);
        assert_eq!(updated_account.initial_balance.currency, Currency::USD);
    }
}

#[actix_web::test]
async fn update_account_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();

    // Bootstrap merchant and profile
    let (_merchant_id, _profile_id, token) = app
        .api_client()
        .bootstrap("Test Merchant", "Test Profile")
        .await;

    // Try to update a non-existent account
    let update_request = AccountUpdateRequest {
        account_name: Some("New Name".to_string()),
    };

    let update_response = client
        .put("/accounts/non_existent_account_id")
        .with_auth(&token)
        .json(&update_request)
        .send()
        .await;

    assert_eq!(update_response.status(), 404);
}
