use api_models::{AccountCreateRequest, GetAccountResponse};
use common_enums::{BalanceDirection, Currency};
use common_types::units::FloatMajorUnit;
use futures::future;

use crate::common::TestClient;
pub mod single_single_test;

pub async fn create_accounts_for_one_to_many(
    client: &TestClient,
    token: &str,
    source_account_name: &str,
    target_account_names: &[String],
) -> (Vec<GetAccountResponse>, Vec<GetAccountResponse>) {
    if target_account_names.is_empty() {
        panic!("Target account names cannot be empty for one-to-many reconciliation");
    }

    let source_response = client
        .post("/accounts")
        .with_auth(token)
        .json(&AccountCreateRequest {
            account_name: source_account_name.to_string(),
            account_type: BalanceDirection::Debit,
            currency: Currency::USD,
            initial_balance: FloatMajorUnit::new(1000.0),
        })
        .send()
        .await;

    let target_responses: Vec<_> = future::join_all(target_account_names.iter().map(|name| {
        client
            .post("/accounts")
            .with_auth(token)
            .json(&AccountCreateRequest {
                account_name: name.to_string(),
                account_type: BalanceDirection::Credit,
                currency: Currency::USD,
                initial_balance: FloatMajorUnit::new(1000.0),
            })
            .send()
    }))
    .await;

    assert_eq!(source_response.status(), 201);
    let source_account: GetAccountResponse = source_response.json().await.unwrap();

    for response in &target_responses {
        assert_eq!(response.status(), 201);
    }

    let target_accounts: Vec<GetAccountResponse> =
        future::join_all(target_responses.into_iter().map(|response| response.json()))
            .await
            .into_iter()
            .map(|result| result.unwrap())
            .collect();

    (vec![source_account], target_accounts)
}
