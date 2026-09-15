use crate::{
    api::{account_test::create_test_account, transaction_test::create_test_recon_rule},
    common::TestApp,
};
use api_models::{
    account::{AccountCreateRequest, GetAccountResponse},
    entry::{
        ApiEntryListFilters, EntryCreateRequest, EntryListPage, GetEntryResponse,
        ListEntriesRequest,
    },
    recon_rule::aging_config::AgingConfigApiRequest,
    transaction::{
        DirectTransactionEntry, GetTransactionResponse, TransactionEntry, TransactionRequest,
        VoidTransactionRequest,
    },
};
use common_enums::{BalanceDirection, Currency, EntryStatus};
use common_types::{
    date_time::TimeRange,
    entry::EntryListSortField,
    pagination::{CursorDirection, CursorPagination, SortOrder},
    units::FloatMajorUnit,
};
use common_utils::id_generation::generate_id_with_default_len;
use masking::{ExposeInterface, Secret};
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::json;

// Helper function to create an entry request
fn entry_request(
    account_id: &str,
    transaction_id: &str,
    entry_type: BalanceDirection,
    amount: f64,
    currency: Currency,
) -> EntryCreateRequest {
    EntryCreateRequest {
        account_id: account_id.to_string(),
        transaction_id: transaction_id.to_string(),
        entry_type,
        amount: FloatMajorUnit::new(amount),
        currency,
        effective_at: common_utils::date_time::now(),
        created_at: common_utils::date_time::now(),
        metadata: Secret::new(json!({"test": "data"})),
        order_id: "order_test".to_string(),
    }
}

// Helper function to generate unique transaction IDs
fn unique_transaction_id() -> String {
    generate_id_with_default_len("transaction")
}

// Test create_entry with valid amount
#[actix_web::test]
async fn test_create_entry_valid() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let transaction_id = unique_transaction_id();
    let request = entry_request(
        &account_id,
        &transaction_id,
        BalanceDirection::Credit,
        100.50,
        Currency::USD,
    );

    let response = client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let body: GetEntryResponse = response.json().await.unwrap();

    assert_eq!(body.account_id, account_id);
    assert_eq!(body.transaction_id, transaction_id);
    assert_eq!(body.entry_type, BalanceDirection::Credit);
    assert_eq!(body.amount.value.0, 100.50);
    assert_eq!(body.amount.currency, Currency::USD);
    assert_eq!(body.status, EntryStatus::Pending);
    // Verify metadata exists by checking the inner value is not empty
    assert!(!body.metadata.get_inner().clone().expose().is_null());
    assert_eq!(body.discarded_at, None);
}

// Test create_entry with invalid amounts
#[rstest]
#[case(-100.0)] // Negative amount
#[case(f64::INFINITY)] // Infinity amount
#[actix_web::test]
async fn test_create_entry_invalid(#[case] amount: f64) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let transaction_id = unique_transaction_id();
    let request = entry_request(
        &account_id,
        &transaction_id,
        BalanceDirection::Credit,
        amount,
        Currency::USD,
    );

    let response = client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 400);
}

// Test get_entry_by_id with valid ID
#[actix_web::test]
async fn test_get_entry_by_id_valid() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let transaction_id = unique_transaction_id();
    let request = entry_request(
        &account_id,
        &transaction_id,
        BalanceDirection::Credit,
        100.50,
        Currency::USD,
    );
    let create_response = client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;
    let created_entry: GetEntryResponse = create_response.json().await.unwrap();
    let entry_id = created_entry.entry_id;

    let response = client
        .get(&format!("/entries/{entry_id}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let retrieved_entry: GetEntryResponse = response.json().await.unwrap();
    assert_eq!(retrieved_entry.entry_id, entry_id);
    assert!(retrieved_entry.entry_id.starts_with("entry_"));
    assert_eq!(retrieved_entry.entry_type, BalanceDirection::Credit);
    assert_eq!(retrieved_entry.amount.value.0, 100.50);
    assert_eq!(retrieved_entry.amount.currency, Currency::USD);
}

// Test get_entry_by_id with invalid ID
#[actix_web::test]
async fn test_get_entry_by_id_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Use a non-existent ID
    let entry_id = uuid::Uuid::new_v4().to_string();

    let response = client
        .get(&format!("/entries/{entry_id}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

// Test get_entries_by_account_id with entries
#[actix_web::test]
async fn test_get_entries_by_account_id_with_entries() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    // Create multiple entries for the same account
    let transaction_id1 = unique_transaction_id();
    let transaction_id2 = unique_transaction_id();
    let request1 = entry_request(
        &account_id,
        &transaction_id1,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
    );
    let request2 = entry_request(
        &account_id,
        &transaction_id2,
        BalanceDirection::Debit,
        50.0,
        Currency::USD,
    );

    client
        .post("/entries")
        .with_auth(&token)
        .json(&request1)
        .send()
        .await;

    client
        .post("/entries")
        .with_auth(&token)
        .json(&request2)
        .send()
        .await;

    let response = client
        .get(&format!("/accounts/{account_id}/entries"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), 2);

    // Verify we have one credit and one debit
    let credits = entries
        .iter()
        .filter(|e| e.entry_type == BalanceDirection::Credit)
        .count();
    let debits = entries
        .iter()
        .filter(|e| e.entry_type == BalanceDirection::Debit)
        .count();
    assert_eq!(credits, 1);
    assert_eq!(debits, 1);
}

// Test get_entries_by_account_id with no entries
#[actix_web::test]
async fn test_get_entries_by_account_id_empty() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    let response = client
        .get(&format!("/accounts/{account_id}/entries"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), 0);
}

// Test get_entries_by_transaction_id with entries
#[actix_web::test]
async fn test_get_entries_by_transaction_id_with_entries() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let transaction_id = unique_transaction_id();

    // Create accounts first
    let account_id1 =
        create_test_account(&client, &token, "Test Account 1", BalanceDirection::Debit).await;
    let account_id2 =
        create_test_account(&client, &token, "Test Account 2", BalanceDirection::Credit).await;

    // Create multiple entries for the same transaction
    let request1 = entry_request(
        &account_id1,
        &transaction_id,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
    );
    let request2 = entry_request(
        &account_id2,
        &transaction_id,
        BalanceDirection::Debit,
        100.0,
        Currency::USD,
    );

    client
        .post("/entries")
        .with_auth(&token)
        .json(&request1)
        .send()
        .await;

    client
        .post("/entries")
        .with_auth(&token)
        .json(&request2)
        .send()
        .await;

    let response = client
        .get(&format!("/transactions/{transaction_id}/entries"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), 2);

    // Verify all entries belong to the correct transaction and have correct format
    for entry in &entries {
        assert_eq!(entry.transaction_id, transaction_id);
        assert!(entry.entry_id.starts_with("entry_"));
    }

    // Verify we have one credit and one debit
    let credits = entries
        .iter()
        .filter(|e| e.entry_type == BalanceDirection::Credit)
        .count();
    let debits = entries
        .iter()
        .filter(|e| e.entry_type == BalanceDirection::Debit)
        .count();
    assert_eq!(credits, 1);
    assert_eq!(debits, 1);
}

// Test time range validation for entries
#[rstest]
#[case(Some(1), None, 200)] // start_time only - valid
#[case(Some(1), Some(2), 200)] // start < end - valid
#[case(Some(2), Some(1), 400)] // start > end - invalid
#[case(Some(1), Some(1), 200)] // start = end - valid
#[case(None, None, 200)] // no time filter - valid
#[actix_web::test]
async fn test_entries_time_range_validation(
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account and entry for testing
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let request = entry_request(
        &account_id,
        &unique_transaction_id(),
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
    );
    client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    // Build query string with time range
    let mut query_params = Vec::new();
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
        .get(&format!("/accounts/{account_id}/entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 200 {
        let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
        // For valid requests, we should get some response (exact count depends on time range)
        assert!(entries.len() <= 1); // At most 1 entry we created
    } else {
        // For invalid requests, check error response
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

// Test get_entries_by_account_id with various filter combinations
#[rstest]
#[case(None, None, 4)] // No filters - returns all entries
#[case(Some("pending"), None, 4)] // Status filter only
#[case(None, Some("credit"), 2)] // Entry type filter only - credit
#[case(None, Some("debit"), 2)] // Entry type filter only - debit
#[case(Some("pending"), Some("credit"), 2)] // Status + entry type
#[case(Some("pending"), Some("debit"), 2)] // Status + entry type
#[actix_web::test]
async fn test_get_entries_by_account_id_with_filters(
    #[case] status: Option<&str>,
    #[case] entry_type: Option<&str>,
    #[case] expected_count: usize,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    // Create multiple entries with different types
    let entries_to_create = vec![
        (BalanceDirection::Credit, 100.0),
        (BalanceDirection::Credit, 50.0),
        (BalanceDirection::Debit, 75.0),
        (BalanceDirection::Debit, 25.0),
    ];

    for (entry_type, amount) in entries_to_create {
        let request = entry_request(
            &account_id,
            &unique_transaction_id(),
            entry_type,
            amount,
            Currency::USD,
        );
        client
            .post("/entries")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;
    }

    // Build query string
    let mut query_params = Vec::new();
    if let Some(s) = status {
        query_params.push(format!("status={s}"));
    }
    if let Some(et) = entry_type {
        query_params.push(format!("entry_type={et}"));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    // Test the filters
    let response = client
        .get(&format!("/accounts/{account_id}/entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), expected_count);

    // Validate filtered results match expected criteria
    for entry in &entries {
        if let Some(s) = status {
            assert_eq!(entry.status.to_string().to_lowercase(), s);
        }
        if let Some(et) = entry_type {
            let entry_types: Vec<&str> = et.split(',').collect();
            assert!(entry_types.contains(&entry.entry_type.to_string().to_lowercase().as_str()));
        }
    }
}

// Test time range filters with specific scenarios
#[rstest]
#[case(true, true, 1)] // Entry within time range
#[case(false, true, 0)] // Entry before time range
#[case(true, false, 1)] // Entry with only start time
#[actix_web::test]
async fn test_get_entries_with_time_range_filters(
    #[case] include_entry: bool,
    #[case] use_end_time: bool,
    #[case] expected_count: usize,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account and entry
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let request = entry_request(
        &account_id,
        &unique_transaction_id(),
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
    );
    client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    // Build time range query
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z").unwrap();
    let start_time = if include_entry {
        (common_utils::date_time::now() - time::Duration::hours(1))
            .format(&format)
            .unwrap()
    } else {
        (common_utils::date_time::now() + time::Duration::hours(1))
            .format(&format)
            .unwrap()
    };

    let query_string = if use_end_time {
        let end_time = (common_utils::date_time::now() + time::Duration::hours(2))
            .format(&format)
            .unwrap();
        format!("?start_time={start_time}&end_time={end_time}")
    } else {
        format!("?start_time={start_time}")
    };

    // Make request
    let response = client
        .get(&format!("/accounts/{account_id}/entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), expected_count);
}

// Test edge cases for filters
#[rstest]
#[case("", "", "", 4)] // Empty filters return all
#[case(",,", ",,", "", 4)] // Only commas return all
#[case("pending,,posted", "credit,,debit", "", 4)] // Mixed empty values are filtered out
#[actix_web::test]
async fn test_get_entries_filter_edge_cases(
    #[case] status: &str,
    #[case] entry_type: &str,
    #[case] _time_range: &str,
    #[case] expected_count: usize,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account first
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    // Create multiple entries
    let entries_to_create = vec![
        (BalanceDirection::Credit, 100.0),
        (BalanceDirection::Credit, 50.0),
        (BalanceDirection::Debit, 75.0),
        (BalanceDirection::Debit, 25.0),
    ];

    for (entry_type_val, amount) in entries_to_create {
        let request = entry_request(
            &account_id,
            &unique_transaction_id(),
            entry_type_val,
            amount,
            Currency::USD,
        );
        client
            .post("/entries")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;
    }

    // Build query string
    let mut query_params = Vec::new();
    if !status.is_empty() {
        query_params.push(format!("status={status}"));
    }
    if !entry_type.is_empty() {
        query_params.push(format!("entry_type={entry_type}"));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    // Test the filters
    let response = client
        .get(&format!("/accounts/{account_id}/entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), expected_count);
}

// Test get_entries_by_transaction_id with no entries
#[actix_web::test]
async fn test_get_entries_by_transaction_id_empty() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let transaction_id = unique_transaction_id();

    let response = client
        .get(&format!("/transactions/{transaction_id}/entries"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), 0);
}

// Test unauthorized access
#[actix_web::test]
async fn test_unauthorized_access() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let account_id = "some_account_id";
    let transaction_id = unique_transaction_id();
    let request = entry_request(
        account_id,
        &transaction_id,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
    );

    // Test create without auth
    let response = client.post("/entries").json(&request).send().await;
    assert_eq!(response.status(), 401);

    // Test get by id without auth
    let response = client.get("/entries/some_id").send().await;
    assert_eq!(response.status(), 401);

    // Test get by account without auth
    let response = client.get("/accounts/some_account/entries").send().await;
    assert_eq!(response.status(), 401);

    // Test get by transaction without auth
    let response = client
        .get("/transactions/some_transaction/entries")
        .send()
        .await;
    assert_eq!(response.status(), 401);
}

// Test balance updates for different account and entry type combinations
#[rstest]
#[case(
    BalanceDirection::Debit,
    BalanceDirection::Credit,
    1000.0,
    100.0,
    900.0
)] // Debit account + Credit entry = decrease
#[case(
    BalanceDirection::Credit,
    BalanceDirection::Credit,
    500.0,
    200.0,
    700.0
)] // Credit account + Credit entry = increase
#[actix_web::test]
async fn test_balance_updates_on_entry_creation(
    #[case] account_type: BalanceDirection,
    #[case] entry_type: BalanceDirection,
    #[case] initial_balance: f64,
    #[case] entry_amount: f64,
    #[case] expected_balance: f64,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_request = AccountCreateRequest {
        account_name: format!("{account_type:?} Account"),
        account_type,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(initial_balance),
    };

    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&account_request)
        .send()
        .await;
    let account: GetAccountResponse = response.json().await.unwrap();

    let entry_request = entry_request(
        &account.account_id,
        &unique_transaction_id(),
        entry_type,
        entry_amount,
        Currency::USD,
    );
    client
        .post("/entries")
        .with_auth(&token)
        .json(&entry_request)
        .send()
        .await;

    let response = client
        .get(&format!("/accounts/{}", account.account_id))
        .with_auth(&token)
        .send()
        .await;
    let updated_account: GetAccountResponse = response.json().await.unwrap();
    assert_eq!(updated_account.pending_balance.value.0, expected_balance);
    assert_eq!(updated_account.posted_balance.value.0, initial_balance); // Posted unchanged for pending entries
}

// Test that different account types handle entries correctly
#[rstest]
#[case(BalanceDirection::Debit, BalanceDirection::Debit, 0.0, 100.0, 100.0)] // Debit account + Debit entry = increase
#[case(BalanceDirection::Debit, BalanceDirection::Credit, 100.0, 50.0, 50.0)] // Debit account + Credit entry = decrease
#[case(BalanceDirection::Credit, BalanceDirection::Credit, 0.0, 100.0, 100.0)] // Credit account + Credit entry = increase
#[case(BalanceDirection::Credit, BalanceDirection::Debit, 100.0, 50.0, 50.0)] // Credit account + Debit entry = decrease
#[actix_web::test]
async fn test_balance_updates_for_different_account_types(
    #[case] account_type: BalanceDirection,
    #[case] entry_type: BalanceDirection,
    #[case] _initial_balance: f64,
    #[case] entry_amount: f64,
    #[case] expected_balance: f64,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account with specific initial balance for the test case
    let account_request = AccountCreateRequest {
        account_name: format!("{account_type:?} Account"),
        account_type,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(_initial_balance),
    };

    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&account_request)
        .send()
        .await;
    let account: GetAccountResponse = response.json().await.unwrap();

    // Create an entry
    let entry_request = entry_request(
        &account.account_id,
        &unique_transaction_id(),
        entry_type,
        entry_amount,
        Currency::USD,
    );
    client
        .post("/entries")
        .with_auth(&token)
        .json(&entry_request)
        .send()
        .await;

    // Check balance
    let response = client
        .get(&format!("/accounts/{}", account.account_id))
        .with_auth(&token)
        .send()
        .await;
    let updated_account: GetAccountResponse = response.json().await.unwrap();
    assert_eq!(updated_account.pending_balance.value.0, expected_balance);
}

// Test multiple entries and complex balance calculations
#[actix_web::test]
async fn test_complex_balance_calculations() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id = create_test_account(
        &client,
        &token,
        "Complex Test Account",
        BalanceDirection::Debit,
    )
    .await;

    // Create multiple entries: Credit 200, Debit 50, Debit 300
    let entries = [
        (BalanceDirection::Credit, 200.0),
        (BalanceDirection::Debit, 50.0),
        (BalanceDirection::Debit, 300.0),
    ];

    for (entry_type, amount) in entries {
        let transaction_id = unique_transaction_id();
        let entry_request = entry_request(
            &account_id,
            &transaction_id,
            entry_type,
            amount,
            Currency::USD,
        );
        client
            .post("/entries")
            .with_auth(&token)
            .json(&entry_request)
            .send()
            .await;
    }

    let response = client
        .get(&format!("/accounts/{account_id}"))
        .with_auth(&token)
        .send()
        .await;
    let account: GetAccountResponse = response.json().await.unwrap();

    // Debit account: 1000 - 200 + 50 + 300 = 1150
    assert_eq!(account.pending_balance.value.0, 1150.0);
    assert_eq!(account.posted_balance.value.0, 1000.0); // Unchanged
}

// Test edge cases for balance updates
#[rstest]
#[case(0.0, 1000.0, 1000.0)] // Zero amount - no change
#[case(500_000.0, 1_000_000.0, 1_500_000.0)] // Large amounts
#[case(25.75, 100.50, 126.25)] // Decimal precision
#[actix_web::test]
async fn test_balance_update_edge_cases(
    #[case] entry_amount: f64,
    #[case] initial_balance: f64,
    #[case] expected_balance: f64,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_request = AccountCreateRequest {
        account_name: "Edge Case Test Account".to_string(),
        account_type: BalanceDirection::Debit,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(initial_balance),
    };

    let response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&account_request)
        .send()
        .await;
    let account: GetAccountResponse = response.json().await.unwrap();

    let entry_request = entry_request(
        &account.account_id,
        &unique_transaction_id(),
        BalanceDirection::Debit,
        entry_amount,
        Currency::USD,
    );
    let entry_response = client
        .post("/entries")
        .with_auth(&token)
        .json(&entry_request)
        .send()
        .await;
    assert_eq!(entry_response.status(), 201);

    let response = client
        .get(&format!("/accounts/{}", account.account_id))
        .with_auth(&token)
        .send()
        .await;
    let updated_account: GetAccountResponse = response.json().await.unwrap();
    assert_eq!(updated_account.pending_balance.value.0, expected_balance);
    assert_eq!(updated_account.posted_balance.value.0, initial_balance); // Posted unchanged
}

// Test combined filters with time range validation for entries
#[rstest]
#[case(Some("pending"), Some("credit"), Some(1), Some(2), 200, 0)] // Valid time range with filters (future time = no entries)
#[case(Some("pending"), Some("debit"), Some(2), Some(1), 400, 0)] // Invalid time range with filters
#[case(None, None, Some(-1), None, 200, 4)] // Past time with start only
#[actix_web::test]
async fn test_entries_combined_filters_with_time_range(
    #[case] status: Option<&str>,
    #[case] entry_type: Option<&str>,
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_status: u16,
    #[case] expected_count: usize,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;

    // Create entries with different properties
    let entries = vec![
        (BalanceDirection::Credit, 100.0),
        (BalanceDirection::Debit, 50.0),
        (BalanceDirection::Credit, 75.0),
        (BalanceDirection::Debit, 25.0),
    ];

    for (entry_type_val, amount) in entries {
        let request = entry_request(
            &account_id,
            &unique_transaction_id(),
            entry_type_val,
            amount,
            Currency::USD,
        );
        client
            .post("/entries")
            .with_auth(&token)
            .json(&request)
            .send()
            .await;
    }

    // Build query string
    let mut query_params = Vec::new();
    if let Some(s) = status {
        query_params.push(format!("status={s}"));
    }
    if let Some(et) = entry_type {
        query_params.push(format!("entry_type={et}"));
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
        .get(&format!("/accounts/{account_id}/entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 200 {
        let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
        assert_eq!(entries.len(), expected_count);

        // Validate filtered results match expected criteria
        for entry in &entries {
            if let Some(s) = status {
                assert_eq!(entry.status.to_string().to_lowercase(), s);
            }
            if let Some(et) = entry_type {
                let entry_types: Vec<&str> = et.split(',').collect();
                assert!(entry_types.contains(&entry.entry_type.to_string().to_lowercase().as_str()));
            }
        }
    } else {
        // For invalid requests, check error response
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

// Test edge cases for time range validation in entries
#[rstest]
#[case(Some(0), Some(0), 200)] // start_time = end_time (same time) - valid
#[case(Some(-24), Some(-1), 200)] // Past time range - valid
#[case(Some(1), Some(24), 200)] // Future time range - valid
#[case(Some(-1), Some(-2), 400)] // start_time > end_time (past) - invalid
#[actix_web::test]
async fn test_entries_time_range_edge_cases(
    #[case] start_hour: Option<i64>,
    #[case] end_hour: Option<i64>,
    #[case] expected_status: u16,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create an account and entry for testing
    let account_id =
        create_test_account(&client, &token, "Test Account", BalanceDirection::Debit).await;
    let request = entry_request(
        &account_id,
        &unique_transaction_id(),
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
    );
    client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    // Build query string with time range
    let mut query_params = Vec::new();
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

    let query_string = format!("?{}", query_params.join("&"));

    // Make request
    let response = client
        .get(&format!("/accounts/{account_id}/entries{query_string}"))
        .with_auth(&token)
        .send()
        .await;

    assert_eq!(response.status(), expected_status);

    if expected_status == 400 {
        // For invalid requests, check error response exists
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(error.get("error").is_some());
    }
}

// Test that balance updates are atomic and multiple entries aggregate correctly
#[actix_web::test]
async fn test_multiple_entries_balance_aggregation() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id = create_test_account(
        &client,
        &token,
        "Aggregation Test Account",
        BalanceDirection::Debit,
    )
    .await;

    // Create multiple entries and verify cumulative balance
    let entries = [
        (BalanceDirection::Debit, 100.0),
        (BalanceDirection::Credit, 50.0),
        (BalanceDirection::Debit, 200.0),
    ];

    let mut expected_balance = 1000.0; // Initial balance from helper
    for (entry_type, amount) in entries {
        let entry_request = entry_request(
            &account_id,
            &unique_transaction_id(),
            entry_type,
            amount,
            Currency::USD,
        );
        let response = client
            .post("/entries")
            .with_auth(&token)
            .json(&entry_request)
            .send()
            .await;
        assert_eq!(response.status(), 201);

        // Update expected balance based on entry type (debit account)
        match entry_type {
            BalanceDirection::Debit => expected_balance += amount,
            BalanceDirection::Credit => expected_balance -= amount,
        }
    }

    let response = client
        .get(&format!("/accounts/{account_id}"))
        .with_auth(&token)
        .send()
        .await;
    let account: GetAccountResponse = response.json().await.unwrap();
    assert_eq!(account.pending_balance.value.0, expected_balance); // 1000 + 100 - 50 + 200 = 1250

    // Verify all entries exist
    let response = client
        .get(&format!("/accounts/{account_id}/entries"))
        .with_auth(&token)
        .send()
        .await;
    let entries: Vec<GetEntryResponse> = response.json().await.unwrap();
    assert_eq!(entries.len(), 3);
}

// ================================================================================================
// VERSION FIELD TESTS
// ================================================================================================

// Test basic version field functionality
#[actix_web::test]
async fn test_entry_version_field_basic() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let account_id = create_test_account(
        &client,
        &token,
        "Version Test Account",
        BalanceDirection::Debit,
    )
    .await;
    let transaction_id = unique_transaction_id();
    let request = entry_request(
        &account_id,
        &transaction_id,
        BalanceDirection::Credit,
        100.0,
        Currency::USD,
    );

    // Create entry and verify version = 1
    let create_response = client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(create_response.status(), 201);
    let created_entry: GetEntryResponse = create_response.json().await.unwrap();
    assert_eq!(created_entry.version, 1);

    // Verify version persists through retrieval
    let get_response = client
        .get(&format!("/entries/{}", created_entry.entry_id))
        .with_auth(&token)
        .send()
        .await;
    let retrieved_entry: GetEntryResponse = get_response.json().await.unwrap();
    assert_eq!(retrieved_entry.version, 1);

    // Verify version appears in list operations
    let list_response = client
        .get(&format!("/accounts/{account_id}/entries"))
        .with_auth(&token)
        .send()
        .await;
    let entries: Vec<GetEntryResponse> = list_response.json().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].version, 1);
}

// Test version field with different scenarios using rstest
#[rstest]
#[case(Currency::USD, BalanceDirection::Credit, 100.0)]
#[case(Currency::EUR, BalanceDirection::Debit, 50.0)]
#[case(Currency::GBP, BalanceDirection::Credit, 0.01)]
#[case(Currency::USD, BalanceDirection::Debit, 999999.99)]
#[actix_web::test]
async fn test_entry_version_field_scenarios(
    #[case] currency: Currency,
    #[case] entry_type: BalanceDirection,
    #[case] amount: f64,
) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    // Create account with the specific currency
    let account_request = AccountCreateRequest {
        account_name: format!("Version Test Account {currency}"),
        account_type: BalanceDirection::Debit,
        currency,
        initial_balance: FloatMajorUnit::new(1000.0),
    };

    let account_response = client
        .post("/accounts")
        .with_auth(&token)
        .json(&account_request)
        .send()
        .await;
    let account: GetAccountResponse = account_response.json().await.unwrap();

    let request = entry_request(
        &account.account_id,
        &unique_transaction_id(),
        entry_type,
        amount,
        currency,
    );

    let response = client
        .post("/entries")
        .with_auth(&token)
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 201);
    let entry: GetEntryResponse = response.json().await.unwrap();
    assert_eq!(entry.version, 1);
    assert_eq!(entry.entry_type, entry_type);
    assert_eq!(entry.amount.value.0, amount);
    assert_eq!(entry.amount.currency, currency);
}

// ===== Cursor-paginated entries list (`POST /entries/list`) =====

fn direct_entry(
    account_id: &str,
    entry_type: BalanceDirection,
    amount: f64,
    effective_at: time::PrimitiveDateTime,
    order_id: &str,
) -> TransactionEntry {
    TransactionEntry::Direct(DirectTransactionEntry {
        account_id: account_id.to_string(),
        entry_type,
        amount: FloatMajorUnit::new(amount),
        currency: Currency::USD,
        effective_at,
        metadata: Secret::new(json!({"test": "data"})),
        staging_entry_id: None,
        transformation_id: None,
        order_id: order_id.to_string(),
    })
}

fn list_request(
    primary_transaction_id: &str,
    filters: ApiEntryListFilters,
    limit: Option<u16>,
    direction: CursorDirection,
    sort_by: EntryListSortField,
    order: SortOrder,
) -> ListEntriesRequest {
    ListEntriesRequest {
        filters: ApiEntryListFilters {
            primary_transaction_id: primary_transaction_id.to_string(),
            ..filters
        },
        cursor_payload: CursorPagination {
            limit,
            direction,
            sort_by,
            order,
        },
    }
}

async fn transaction_entries(
    client: &crate::common::TestClient,
    token: &str,
    logical_transaction_id: &str,
) -> Vec<GetEntryResponse> {
    let response = client
        .get(&format!("/transactions/{logical_transaction_id}/entries"))
        .with_auth(token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

struct SeededTransaction {
    primary_id: String,
    credit_account_id: String,
    debit_account_id: String,
    credit_entry_ids: Vec<String>,
    debit_entry_id: String,
    entries: Vec<GetEntryResponse>,
}

async fn seed_transaction_entries(
    client: &crate::common::TestClient,
    token: &str,
    spacing_minutes: i64,
) -> SeededTransaction {
    let debit_account_id =
        create_test_account(client, token, "List Debit Account", BalanceDirection::Debit).await;
    let credit_account_id = create_test_account(
        client,
        token,
        "List Credit Account",
        BalanceDirection::Credit,
    )
    .await;
    let rule_id = create_test_recon_rule(
        client,
        token,
        "entry_list_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let base = common_utils::date_time::now();
    let mut entries = vec![direct_entry(
        &debit_account_id,
        BalanceDirection::Debit,
        300.0,
        base,
        "order_funding",
    )];
    for (index, order_id) in ["order_a", "order_b", "order_c"].iter().enumerate() {
        let offset = i64::try_from(index).expect("seed index fits in i64");
        entries.push(direct_entry(
            &credit_account_id,
            BalanceDirection::Credit,
            100.0,
            base + time::Duration::minutes(spacing_minutes * offset),
            order_id,
        ));
    }

    let response = client
        .post("/transactions")
        .with_auth(token)
        .json(&TransactionRequest { entries, rule_id })
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let transaction: GetTransactionResponse = response.json().await.unwrap();

    let entries = transaction_entries(client, token, &transaction.transaction_id).await;
    let entry_id_for = |order_id: &str| {
        entries
            .iter()
            .find(|entry| entry.order_id == order_id)
            .unwrap_or_else(|| panic!("seeded entry {order_id} missing"))
            .entry_id
            .clone()
    };

    SeededTransaction {
        primary_id: transaction.id,
        credit_entry_ids: vec![
            entry_id_for("order_a"),
            entry_id_for("order_b"),
            entry_id_for("order_c"),
        ],
        debit_entry_id: entry_id_for("order_funding"),
        credit_account_id,
        debit_account_id,
        entries,
    }
}

async fn list_entries(
    client: &crate::common::TestClient,
    token: &str,
    request: &ListEntriesRequest,
) -> EntryListPage {
    let response = client
        .post("/entries/list")
        .with_auth(token)
        .json(request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[actix_web::test]
async fn test_list_entries_route_is_not_shadowed_by_entry_id() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client
        .post("/entries/list")
        .with_auth(&token)
        .json(&list_request(
            "txn_does_not_exist",
            ApiEntryListFilters::default(),
            None,
            CursorDirection::Next,
            EntryListSortField::Id { id: None },
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(response.status(), 404, "POST /entries/list must not 405");
}

#[actix_web::test]
async fn test_list_entries_pages_forward_and_backward() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let seeded = seed_transaction_entries(&client, &token, 1).await;

    let filters = ApiEntryListFilters {
        account_ids: Some(vec![seeded.credit_account_id.clone()]),
        ..Default::default()
    };

    let first = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters.clone(),
            Some(1),
            CursorDirection::Next,
            EntryListSortField::EffectiveAt { cursor_value: None },
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].entry_id, seeded.credit_entry_ids[2]);
    assert!(
        first.prev_cursor.is_none(),
        "first page has nothing before it"
    );
    let next_cursor = first.next_cursor.clone().expect("more pages remain");

    let second = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters.clone(),
            Some(1),
            CursorDirection::Next,
            next_cursor,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].entry_id, seeded.credit_entry_ids[1]);
    assert!(second.next_cursor.is_some());
    let back_cursor = second.prev_cursor.clone().expect("page 2 can go back");

    let third = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters.clone(),
            Some(1),
            CursorDirection::Next,
            second.next_cursor.clone().expect("page 3 exists"),
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(third.items.len(), 1);
    assert_eq!(third.items[0].entry_id, seeded.credit_entry_ids[0]);
    assert!(third.next_cursor.is_none(), "last page has no next cursor");

    let back = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters.clone(),
            Some(1),
            CursorDirection::Previous,
            back_cursor,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(back.items.len(), 1);
    assert_eq!(back.items[0].entry_id, first.items[0].entry_id);

    let ascending = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters,
            None,
            CursorDirection::Next,
            EntryListSortField::EffectiveAt { cursor_value: None },
            SortOrder::Asc,
        ),
    )
    .await;

    let ascending_ids: Vec<_> = ascending.items.iter().map(|e| &e.entry_id).collect();
    assert_eq!(
        ascending_ids,
        vec![
            &seeded.credit_entry_ids[0],
            &seeded.credit_entry_ids[1],
            &seeded.credit_entry_ids[2]
        ]
    );
}

#[actix_web::test]
async fn test_list_entries_applies_filters() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let seeded = seed_transaction_entries(&client, &token, 1).await;
    let other = seed_transaction_entries(&client, &token, 1).await;

    let seeded_ids: Vec<String> = seeded
        .entries
        .iter()
        .map(|entry| entry.entry_id.clone())
        .collect();
    let other_ids: Vec<String> = other
        .entries
        .iter()
        .map(|entry| entry.entry_id.clone())
        .collect();
    let mut observed_statuses: Vec<EntryStatus> = Vec::new();
    for entry in &seeded.entries {
        if !observed_statuses.contains(&entry.status) {
            observed_statuses.push(entry.status);
        }
    }

    let cases: Vec<(&str, ApiEntryListFilters, Vec<String>, Vec<String>)> = vec![
        (
            "transaction pin only",
            ApiEntryListFilters::default(),
            seeded_ids.clone(),
            other_ids.clone(),
        ),
        (
            "account_ids (credit leg)",
            ApiEntryListFilters {
                account_ids: Some(vec![seeded.credit_account_id.clone()]),
                ..Default::default()
            },
            seeded.credit_entry_ids.clone(),
            vec![seeded.debit_entry_id.clone()],
        ),
        (
            "account_ids (debit leg)",
            ApiEntryListFilters {
                account_ids: Some(vec![seeded.debit_account_id.clone()]),
                ..Default::default()
            },
            vec![seeded.debit_entry_id.clone()],
            seeded.credit_entry_ids.clone(),
        ),
        (
            "entry_type",
            ApiEntryListFilters {
                entry_type: Some(BalanceDirection::Debit),
                ..Default::default()
            },
            vec![seeded.debit_entry_id.clone()],
            seeded.credit_entry_ids.clone(),
        ),
        (
            "order_ids",
            ApiEntryListFilters {
                order_ids: Some(vec!["order_a".to_string()]),
                ..Default::default()
            },
            vec![seeded.credit_entry_ids[0].clone()],
            vec![
                seeded.credit_entry_ids[1].clone(),
                seeded.credit_entry_ids[2].clone(),
                seeded.debit_entry_id.clone(),
            ],
        ),
        (
            "status (observed)",
            ApiEntryListFilters {
                status: Some(observed_statuses),
                ..Default::default()
            },
            seeded_ids.clone(),
            vec![],
        ),
        (
            "status (archived, none seeded)",
            ApiEntryListFilters {
                status: Some(vec![EntryStatus::Archived]),
                ..Default::default()
            },
            vec![],
            seeded_ids.clone(),
        ),
        (
            "currency (matching)",
            ApiEntryListFilters {
                currency: Some(vec![Currency::USD]),
                ..Default::default()
            },
            seeded_ids.clone(),
            vec![],
        ),
        (
            "currency (absent)",
            ApiEntryListFilters {
                currency: Some(vec![Currency::EUR]),
                ..Default::default()
            },
            vec![],
            seeded_ids.clone(),
        ),
    ];

    for (name, filters, expected, excluded) in cases {
        let page = list_entries(
            &client,
            &token,
            &list_request(
                &seeded.primary_id,
                filters,
                Some(50),
                CursorDirection::Next,
                EntryListSortField::Id { id: None },
                SortOrder::Desc,
            ),
        )
        .await;

        let returned: Vec<&String> = page.items.iter().map(|e| &e.entry_id).collect();

        for id in &expected {
            assert!(
                returned.contains(&id),
                "filter `{name}` should include {id}"
            );
        }
        for id in &excluded {
            assert!(
                !returned.contains(&id),
                "filter `{name}` should exclude {id}"
            );
        }
    }
}

#[actix_web::test]
async fn test_list_entries_is_scoped_to_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let seeded = seed_transaction_entries(&client, &token, 1).await;

    let (_, _, other_token) = client.bootstrap("Other Merchant", "Other Profile").await;

    let response = client
        .post("/entries/list")
        .with_auth(&other_token)
        .json(&list_request(
            &seeded.primary_id,
            ApiEntryListFilters::default(),
            Some(50),
            CursorDirection::Next,
            EntryListSortField::Id { id: None },
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(
        response.status(),
        404,
        "another profile's transaction id must not resolve"
    );
}

#[actix_web::test]
async fn test_list_entries_unauthorized() {
    let app = TestApp::new().await;
    let client = app.api_client();

    let response = client
        .post("/entries/list")
        .json(&list_request(
            "txn_does_not_exist",
            ApiEntryListFilters::default(),
            None,
            CursorDirection::Next,
            EntryListSortField::Id { id: None },
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(response.status(), 401);
}

#[actix_web::test]
async fn test_list_entries_effective_at_cursor_tiebreaks_equal_timestamps() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let seeded = seed_transaction_entries(&client, &token, 0).await;

    let filters = ApiEntryListFilters {
        account_ids: Some(vec![seeded.credit_account_id.clone()]),
        ..Default::default()
    };

    let mut forward_ids = Vec::new();
    let mut prev_cursor_of_last_page;
    let mut cursor = EntryListSortField::EffectiveAt { cursor_value: None };
    loop {
        let page = list_entries(
            &client,
            &token,
            &list_request(
                &seeded.primary_id,
                filters.clone(),
                Some(1),
                CursorDirection::Next,
                cursor,
                SortOrder::Desc,
            ),
        )
        .await;

        assert_eq!(page.items.len(), 1);
        forward_ids.push(page.items[0].entry_id.clone());
        assert!(
            forward_ids.len() <= seeded.credit_entry_ids.len(),
            "cursor must not revisit rows: {forward_ids:?}"
        );
        prev_cursor_of_last_page = page.prev_cursor.clone();
        match page.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }

    let mut unique_forward = forward_ids.clone();
    unique_forward.sort();
    unique_forward.dedup();
    let mut expected = seeded.credit_entry_ids.clone();
    expected.sort();
    assert_eq!(
        unique_forward, expected,
        "every seeded entry appears exactly once across pages"
    );

    let mut backward_ids = Vec::new();
    let mut back_cursor = prev_cursor_of_last_page;
    while let Some(cursor) = back_cursor {
        let page = list_entries(
            &client,
            &token,
            &list_request(
                &seeded.primary_id,
                filters.clone(),
                Some(1),
                CursorDirection::Previous,
                cursor,
                SortOrder::Desc,
            ),
        )
        .await;

        assert_eq!(page.items.len(), 1);
        backward_ids.push(page.items[0].entry_id.clone());
        assert!(
            backward_ids.len() < seeded.credit_entry_ids.len(),
            "backward walk must terminate before revisiting rows: {backward_ids:?}"
        );
        back_cursor = page.prev_cursor.clone();
    }

    let expected_backward: Vec<String> = forward_ids[..forward_ids.len() - 1]
        .iter()
        .rev()
        .cloned()
        .collect();
    assert_eq!(
        backward_ids, expected_backward,
        "backward walk must revisit the forward pages in reverse"
    );
}

#[actix_web::test]
async fn test_list_entries_pages_by_id_cursor() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;
    let seeded = seed_transaction_entries(&client, &token, 1).await;

    let filters = ApiEntryListFilters {
        account_ids: Some(vec![seeded.credit_account_id.clone()]),
        ..Default::default()
    };

    let first_page = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters.clone(),
            Some(2),
            CursorDirection::Next,
            EntryListSortField::Id { id: None },
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(first_page.items.len(), 2);
    assert!(first_page.prev_cursor.is_none());
    let first_page_ids: Vec<String> = first_page
        .items
        .iter()
        .map(|entry| entry.entry_id.clone())
        .collect();
    let next_cursor = first_page
        .next_cursor
        .clone()
        .expect("first page should include a next cursor");

    match &next_cursor {
        EntryListSortField::Id { id: Some(id) } => {
            assert_eq!(Some(id), first_page_ids.last());
        }
        cursor => panic!("expected id cursor, got {cursor:?}"),
    }

    let second_page = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters.clone(),
            Some(2),
            CursorDirection::Next,
            next_cursor,
            SortOrder::Desc,
        ),
    )
    .await;

    assert_eq!(second_page.items.len(), 1);
    assert!(second_page.next_cursor.is_none());
    let second_page_ids: Vec<String> = second_page
        .items
        .iter()
        .map(|entry| entry.entry_id.clone())
        .collect();
    assert!(first_page_ids
        .iter()
        .all(|id| !second_page_ids.contains(id)));

    let previous_page = list_entries(
        &client,
        &token,
        &list_request(
            &seeded.primary_id,
            filters,
            Some(2),
            CursorDirection::Previous,
            second_page
                .prev_cursor
                .expect("second page should include a previous cursor"),
            SortOrder::Desc,
        ),
    )
    .await;

    let previous_page_ids: Vec<String> = previous_page
        .items
        .iter()
        .map(|entry| entry.entry_id.clone())
        .collect();
    assert_eq!(previous_page_ids, first_page_ids);
}

#[actix_web::test]
async fn test_list_entries_returns_archived_entries_of_superseded_version() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let debit_account_id = create_test_account(
        &client,
        &token,
        "Void Debit Account",
        BalanceDirection::Debit,
    )
    .await;
    let credit_account_id = create_test_account(
        &client,
        &token,
        "Void Credit Account",
        BalanceDirection::Credit,
    )
    .await;
    let rule_id = create_test_recon_rule(
        &client,
        &token,
        "entry_list_void_rule",
        debit_account_id.clone(),
        credit_account_id.clone(),
        AgingConfigApiRequest::NoAging,
    )
    .await;

    let effective_at = common_utils::date_time::now();
    let transaction_request = TransactionRequest {
        entries: vec![
            direct_entry(
                &debit_account_id,
                BalanceDirection::Debit,
                100.0,
                effective_at,
                "order_void_list",
            ),
            direct_entry(
                &credit_account_id,
                BalanceDirection::Credit,
                100.0,
                effective_at,
                "order_void_list",
            ),
        ],
        rule_id,
    };

    let create_response = client
        .post("/transactions")
        .with_auth(&token)
        .json(&transaction_request)
        .send()
        .await;
    assert_eq!(create_response.status(), 201);
    let transaction: GetTransactionResponse = create_response.json().await.unwrap();
    let original_entry_ids = transaction.entry_id.clone();

    let void_response = client
        .put(&format!("/transactions/{}/void", transaction.id))
        .with_auth(&token)
        .json(&VoidTransactionRequest {
            reason: Some("archive the original entries".to_string()),
        })
        .send()
        .await;
    assert_eq!(void_response.status(), 200);
    let void_transaction: GetTransactionResponse = void_response.json().await.unwrap();

    let list_by_primary_id = async |primary_id: String, status: Option<Vec<EntryStatus>>| {
        let page = list_entries(
            &client,
            &token,
            &list_request(
                &primary_id,
                ApiEntryListFilters {
                    status,
                    ..Default::default()
                },
                Some(50),
                CursorDirection::Next,
                EntryListSortField::Id { id: None },
                SortOrder::Desc,
            ),
        )
        .await;

        let mut ids: Vec<String> = page
            .items
            .iter()
            .map(|entry| entry.entry_id.clone())
            .collect();
        ids.sort();
        (page, ids)
    };

    let (page, listed_ids) = list_by_primary_id(void_transaction.id.clone(), None).await;
    let mut void_entry_ids = void_transaction.entry_id.clone();
    void_entry_ids.sort();
    assert_eq!(
        listed_ids, void_entry_ids,
        "the void version's entries are listable"
    );
    for item in &page.items {
        assert_eq!(item.status, EntryStatus::Void);
    }

    let (page, listed_ids) = list_by_primary_id(transaction.id.clone(), None).await;
    let mut expected_original = original_entry_ids.clone();
    expected_original.sort();
    assert_eq!(
        listed_ids, expected_original,
        "a superseded version's entries are returned by default"
    );
    for item in &page.items {
        assert_eq!(item.status, EntryStatus::Archived);
    }

    let (_, void_filtered) =
        list_by_primary_id(transaction.id.clone(), Some(vec![EntryStatus::Void])).await;
    assert!(
        void_filtered.is_empty(),
        "the superseded version has no void entries"
    );
}

#[actix_web::test]
async fn test_list_entries_unknown_primary_transaction_id_is_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client
        .post("/entries/list")
        .with_auth(&token)
        .json(&list_request(
            "txn_does_not_exist",
            ApiEntryListFilters::default(),
            None,
            CursorDirection::Next,
            EntryListSortField::Id { id: None },
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

#[actix_web::test]
async fn test_list_entries_rejects_invalid_time_range() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Test Merchant", "Test Profile").await;

    let now = common_utils::date_time::now();
    let response = client
        .post("/entries/list")
        .with_auth(&token)
        .json(&list_request(
            "txn_does_not_exist",
            ApiEntryListFilters {
                time_range: Some(TimeRange {
                    start_time: now,
                    end_time: Some(now - time::Duration::hours(1)),
                }),
                ..Default::default()
            },
            None,
            CursorDirection::Next,
            EntryListSortField::Id { id: None },
            SortOrder::Desc,
        ))
        .send()
        .await;

    assert_eq!(
        response.status(),
        400,
        "start_time after end_time must be rejected"
    );
}
