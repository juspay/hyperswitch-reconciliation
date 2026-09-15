# Pattern: Writing Integration Tests

## Test Execution
<!-- AI-UPDATED: 2025-07-07 - Use just test command -->
- **ALWAYS use `just test`** instead of `cargo test` for running tests
- The project uses `cargo nextest` under the hood for faster test execution
- All test arguments are passed directly to `nextest run`

## Testing Philosophy
<!-- AI-UPDATED: 2025-06-24 - Focus on business logic testing -->
- **Business Logic Only**: Test domain rules and business constraints, not framework features
- **No Auth Tests**: Authentication is handled by framework middleware
- **No Deserialization Tests**: Serde handles JSON parsing correctly
- **No Validation Tests**: Input validation is a framework concern
- **Keep It Concise**: 1-2 focused tests per feature using rstest where applicable
- **Test What Matters**: Processing decisions, security boundaries, domain rules

## When to Use
- Testing business logic and domain rules
- Verifying security boundaries (e.g., profile isolation)
- Testing complex workflows and state transitions
- Ensuring correct processing decisions
- Validating domain-specific error handling

## Quick Example
```rust
#[actix_web::test]
async fn test_create_account() {
    let app = TestApp::new().await;
    let client = app.api_client();
    
    let response = client
        .post("/accounts")
        .json(&account_request("Test Account"))
        .send()
        .await;
    
    assert_eq!(response.status(), 201);
    let body: AccountResponse = response.json().await.unwrap();
    assert_eq!(body.account_name, "Test Account");
}
```

## Full Implementation
```rust
// 1. Test file structure
// crates/recon/tests/api/account_test.rs

use common::TestApp;
use api_models::account::{AccountCreateRequest, AccountResponse};

// 2. Local test fixtures (not shared)
fn account_request(name: &str) -> AccountCreateRequest {
    AccountCreateRequest {
        account_name: name.to_string(),
        account_type: common_enums::BalanceDirection::Credit,
        initial_balance: 1000.0,
        currency: common_enums::Currency::USD,
    }
}

fn merchant_and_profile() -> (String, String) {
    ("merchant_test123".to_string(), "profile_test456".to_string())
}

// 3. Basic CRUD test
#[actix_web::test]
async fn test_account_lifecycle() {
    // Setup
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id) = merchant_and_profile();
    
    // Create account
    let create_response = client
        .post("/accounts")
        .header("x-merchant-id", &merchant_id)
        .header("x-profile-id", &profile_id)
        .json(&account_request("Savings Account"))
        .send()
        .await;
    
    assert_eq!(create_response.status(), 201);
    let created: AccountResponse = create_response.json().await.unwrap();
    assert_eq!(created.account_name, "Savings Account");
    
    // Read account
    let get_response = client
        .get(&{
            let account_id = created.account_id;
            format!("/accounts/{account_id}")
        })
        .header("x-merchant-id", &merchant_id)
        .send()
        .await;
    
    assert_eq!(get_response.status(), 200);
    let fetched: AccountResponse = get_response.json().await.unwrap();
    assert_eq!(fetched.account_id, created.account_id);
    
    // Update account
    let update_request = serde_json::json!({
        "account_name": "Updated Savings"
    });
    
    let update_response = client
        .put(&{
            let account_id = created.account_id;
            format!("/accounts/{account_id}")
        })
        .header("x-merchant-id", &merchant_id)
        .json(&update_request)
        .send()
        .await;
    
    assert_eq!(update_response.status(), 200);
    let updated: AccountResponse = update_response.json().await.unwrap();
    assert_eq!(updated.account_name, "Updated Savings");
}

// 4. Testing with authentication
#[actix_web::test]
async fn test_authenticated_endpoint() {
    let app = TestApp::new().await;
    let client = app.api_client();
    
    // Bootstrap to get auth token
    let bootstrap_response = client
        .post("/auth/bootstrap")
        .json(&serde_json::json!({
            "merchant_name": "Test Merchant",
            "profile_name": "Test Profile"
        }))
        .send()
        .await;
    
    assert_eq!(bootstrap_response.status(), 201);
    let bootstrap: serde_json::Value = bootstrap_response.json().await.unwrap();
    let token = bootstrap["token"].as_str().unwrap();
    let merchant_id = bootstrap["merchant_id"].as_str().unwrap();
    
    // Use token for authenticated request
    let auth_response = client
        .get(&format!("/merchant_accounts/{merchant_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
    
    assert_eq!(auth_response.status(), 200);
}

// 5. Error scenario testing
#[actix_web::test]
async fn test_validation_errors() {
    let app = TestApp::new().await;
    let client = app.api_client();
    
    // Test empty name validation
    let invalid_request = AccountCreateRequest {
        account_name: "".to_string(), // Invalid: empty
        account_type: common_enums::BalanceDirection::Credit,
        initial_balance: 1000.0,
        currency: common_enums::Currency::USD,
    };
    
    let response = client
        .post("/accounts")
        .json(&invalid_request)
        .send()
        .await;
    
    assert_eq!(response.status(), 400);
    let error: serde_json::Value = response.json().await.unwrap();
    assert_eq!(error["error_code"], "INVALID_ACCOUNT_NAME");
}

// 6. Testing with database state verification
#[actix_web::test]
async fn test_database_state_changes() {
    let app = TestApp::new().await;
    let client = app.api_client();
    
    // Create two accounts
    let account1 = client
        .post("/accounts")
        .json(&account_request("Account 1"))
        .send()
        .await
        .json::<AccountResponse>()
        .await
        .unwrap();
    
    let account2 = client
        .post("/accounts")
        .json(&account_request("Account 2"))
        .send()
        .await
        .json::<AccountResponse>()
        .await
        .unwrap();
    
    // List all accounts
    let list_response = client
        .get("/accounts")
        .header("x-profile-id", &account1.profile_id)
        .send()
        .await;
    
    assert_eq!(list_response.status(), 200);
    let accounts: Vec<AccountResponse> = list_response.json().await.unwrap();
    assert_eq!(accounts.len(), 2);
    
    // Verify both accounts exist
    let account_ids: Vec<String> = accounts
        .iter()
        .map(|a| a.account_id.clone())
        .collect();
    assert!(account_ids.contains(&account1.account_id));
    assert!(account_ids.contains(&account2.account_id));
}

// 7. Testing pagination
#[actix_web::test]
async fn test_pagination() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let profile_id = "test_profile";
    
    // Create 25 accounts
    for i in 0..25 {
        client
            .post("/accounts")
            .header("x-profile-id", profile_id)
            .json(&account_request(&format!("Account {i}"))
            .send()
            .await;
    }
    
    // Get first page
    let page1 = client
        .get("/accounts")
        .header("x-profile-id", profile_id)
        .query(&[("limit", "10"), ("offset", "0")])
        .send()
        .await;
    
    assert_eq!(page1.status(), 200);
    let page1_accounts: Vec<AccountResponse> = page1.json().await.unwrap();
    assert_eq!(page1_accounts.len(), 10);
    
    // Get second page
    let page2 = client
        .get("/accounts")
        .header("x-profile-id", profile_id)
        .query(&[("limit", "10"), ("offset", "10")])
        .send()
        .await;
    
    let page2_accounts: Vec<AccountResponse> = page2.json().await.unwrap();
    assert_eq!(page2_accounts.len(), 10);
    
    // Verify no overlap
    let page1_ids: Vec<_> = page1_accounts.iter().map(|a| &a.account_id).collect();
    let page2_ids: Vec<_> = page2_accounts.iter().map(|a| &a.account_id).collect();
    for id in page1_ids {
        assert!(!page2_ids.contains(&id));
    }
}

// 8. Parameterized tests with rstest
#[rstest]
#[case("", 400, "INVALID_ACCOUNT_NAME")]
#[case("A", 400, "NAME_TOO_SHORT")]
#[case("Valid Name", 201, "")]
#[case("x".repeat(256).as_str(), 400, "NAME_TOO_LONG")]
async fn test_account_name_validation(
    #[case] name: &str,
    #[case] expected_status: u16,
    #[case] expected_error: &str,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    
    let response = client
        .post("/accounts")
        .json(&account_request(name))
        .send()
        .await;
    
    assert_eq!(response.status(), expected_status);
    
    if expected_status != 201 {
        let error: serde_json::Value = response.json().await.unwrap();
        assert_eq!(error["error_code"], expected_error);
    }
}
```

## Key Points
- **TestApp handles setup** - Database, server, configuration
- **Builder pattern for requests** - `.post().json().send()`
- **Local fixtures** - Define test data in test files
- **Explicit assertions** - Check status codes and response bodies
- **Clean state** - Each test gets fresh database
- **No hidden behavior** - Tests show exact HTTP flow

## Centralized Test Helpers
<!-- AI-UPDATED: 2024-12-08 - Account helper centralized -->
- `create_test_account()` is centralized in `crates/recon/tests/api/account_test.rs`
- Import and use it: `use crate::api::account_test::create_test_account;`
- Don't create duplicate account helpers in other test files
- Other helpers (transaction, recon_rule) still need centralization - WIP

## Common Mistakes
- Sharing fixtures across test files (causes coupling)
- Not checking HTTP status codes explicitly
- Using unwrap() without clear test failure messages
- Testing implementation details instead of API contract
- Not testing error scenarios
- Forgetting to test authentication requirements

## Related Patterns
- [ADD-ENDPOINT](ADD-ENDPOINT.md) - What endpoints to test
- [ERROR-HANDLING](ERROR-HANDLING.md) - Testing error responses
- [ADD-AUTH](ADD-AUTH.md) - Testing protected endpoints

## Gotchas to Remember
- Tests run with fresh database (via `just resurrect`)
- Use `#[actix_web::test]` for async test attribute
- TestClient returns raw `reqwest::Response`
- Parse JSON responses explicitly in tests
- Test both success and failure paths
- Keep test data simple and focused