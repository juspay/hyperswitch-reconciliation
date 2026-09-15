use api_models::{AccountCreateRequest, GetAccountResponse};
use common_enums::{BalanceDirection, Currency};
use common_types::units::FloatMajorUnit;

use crate::common::TestClient;

pub mod many_many_test;
pub mod many_single_test;
pub mod single_many_test;
pub mod single_single_test;

pub async fn create_accounts_for_one_to_one(
    client: &TestClient,
    token: &str,
    source_account_name: &str,
    target_account_name: &str,
) -> (Vec<GetAccountResponse>, Vec<GetAccountResponse>) {
    let source_account_request = AccountCreateRequest {
        account_name: source_account_name.to_string(),
        account_type: BalanceDirection::Debit,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(1000.0),
    };

    let target_account_request = AccountCreateRequest {
        account_name: target_account_name.to_string(),
        account_type: BalanceDirection::Credit,
        currency: Currency::USD,
        initial_balance: FloatMajorUnit::new(1000.0),
    };

    let source_response = client
        .post("/accounts")
        .with_auth(token)
        .json(&source_account_request)
        .send()
        .await;

    let target_response = client
        .post("/accounts")
        .with_auth(token)
        .json(&target_account_request)
        .send()
        .await;

    assert_eq!(source_response.status(), 201);
    let source_account: GetAccountResponse = source_response.json().await.unwrap();

    assert_eq!(target_response.status(), 201);
    let target_account: GetAccountResponse = target_response.json().await.unwrap();

    (vec![source_account], vec![target_account])
}
