use std::collections::HashSet;

use api_models::recon_rule::aging_config::AgingConfigApiRequest;
use api_models::reporting::{ExceptionReportFilters, TransactionReportFilters};
use common_enums::BalanceDirection;
use common_types::date_time::TimeRange;
use common_utils::date_time;
use rstest::rstest;
use time::Duration;

use crate::{
    api::{account_test::create_test_account, transaction_test::create_test_recon_rule},
    common::{TestApp, TestClient},
};

async fn setup_rule(client: &TestClient, token: &str) -> String {
    let source_account_id =
        create_test_account(client, token, "Source Account", BalanceDirection::Credit).await;
    let target_account_id =
        create_test_account(client, token, "Target Account", BalanceDirection::Debit).await;

    create_test_recon_rule(
        client,
        token,
        "test_rule_reporting",
        source_account_id,
        target_account_id,
        AgingConfigApiRequest::NoAging,
    )
    .await
}

fn time_range() -> TimeRange {
    TimeRange {
        start_time: date_time::now() - Duration::days(1),
        end_time: Some(date_time::now()),
    }
}

#[rstest]
#[case("/report/transactions")]
#[case("/report/exceptions")]
#[actix_web::test]
async fn test_trigger_report_success(#[case] path: &str) {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;
    let rule_id = setup_rule(&client, &token).await;

    let response = if path.ends_with("exceptions") {
        client
            .post(path)
            .with_auth(&token)
            .json(&ExceptionReportFilters {
                time_range: time_range(),
                rule_id,
                emails: HashSet::new(),
            })
            .send()
            .await
    } else {
        client
            .post(path)
            .with_auth(&token)
            .json(&TransactionReportFilters {
                time_range: time_range(),
                rule_id,
                emails: HashSet::new(),
            })
            .send()
            .await
    };

    assert_eq!(response.status(), 200);
}

#[actix_web::test]
async fn test_trigger_report_rule_not_found() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;

    let response = client
        .post("/report/transactions")
        .with_auth(&token)
        .json(&TransactionReportFilters {
            time_range: time_range(),
            rule_id: "recon_rule_does_not_exist".to_string(),
            emails: HashSet::new(),
        })
        .send()
        .await;

    assert_eq!(response.status(), 404);
}

#[actix_web::test]
async fn test_trigger_report_defaults_end_time_to_now() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_merchant_id, _profile_id, token) =
        client.bootstrap("Test Merchant", "Test Profile").await;
    let rule_id = setup_rule(&client, &token).await;

    let response = client
        .post("/report/transactions")
        .with_auth(&token)
        .json(&TransactionReportFilters {
            time_range: TimeRange {
                start_time: date_time::now() - Duration::days(1),
                end_time: None,
            },
            rule_id,
            emails: HashSet::new(),
        })
        .send()
        .await;

    assert_eq!(response.status(), 200);
}
