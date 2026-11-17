use std::collections::HashMap;

use api_models::{
    api_key::{ApiKeyResponse, CreateApiKeyResponse, ListApiKeysResponse},
    staging_entry::{GetStagingEntryResponse, StagingEntryCreateRequestV2},
};
use common_enums::{ApiKeyStatus, BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::recon_rule::entry_field::EntryField;
use common_utils::date_time;
use masking::PeekInterface;
use serde_json::{json, Value};
use time::Duration;

use crate::{
    api::{
        account_test::create_test_account, staging_entry_test::staging_entry_request,
        transformation_test::create_transformation_config,
    },
    common::TestApp,
};

const ADMIN_API_KEY: &str = "test_admin_12345678901234567890";

fn expires_in_days(days: i64) -> Value {
    json!((date_time::now() + Duration::days(days))
        .assume_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap())
}

fn create_body(name: &str, days: i64) -> Value {
    json!({
        "name": name,
        "description": "used by tests",
        "expires_at": expires_in_days(days),
    })
}

async fn create_key(
    client: &crate::common::TestClient,
    token: &str,
    name: &str,
) -> CreateApiKeyResponse {
    let response = client
        .post("/api_keys")
        .with_auth(token)
        .json(&create_body(name, 30))
        .send()
        .await;
    assert_eq!(response.status(), 201);
    client.expect_success(response).await
}

#[tokio::test]
async fn create_returns_plaintext_once() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, _, token) = client
        .bootstrap("ak_create_merchant", "ak_create_profile")
        .await;

    let response = client
        .post("/api_keys")
        .with_auth(&token)
        .json(&create_body("reporting", 30))
        .send()
        .await;
    assert_eq!(response.status(), 201);
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let created: CreateApiKeyResponse = response.json().await.unwrap();

    let plaintext = created.api_key.peek().clone();
    assert!(plaintext.starts_with("dev_"), "{}", plaintext.len());
    assert_eq!(created.api_key_details.merchant_id, merchant_id);
    assert_eq!(created.api_key_details.status, ApiKeyStatus::Active);
    assert!(created.api_key_details.id.starts_with("apikey_"));
    assert!(plaintext.starts_with(&created.api_key_details.prefix));
    assert!(created.api_key_details.created_by.is_some());

    let id = created.api_key_details.id;

    // Neither get nor list ever contain the plaintext.
    let get_body = client
        .get(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .send()
        .await
        .text()
        .await
        .unwrap();
    assert!(!get_body.contains(&plaintext));

    let list_body = client
        .get("/api_keys")
        .with_auth(&token)
        .send()
        .await
        .text()
        .await
        .unwrap();
    assert!(!list_body.contains(&plaintext));
    assert!(list_body.contains(&id));
}

async fn error_code(response: reqwest::Response) -> String {
    let body: Value = response.json().await.unwrap();
    body["error"]["code"]
        .as_str()
        .unwrap_or_else(|| panic!("no error code in {body}"))
        .to_string()
}

#[tokio::test]
async fn create_rejects_invalid_expiry_and_name() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_invalid_merchant", "ak_invalid_profile")
        .await;

    // Each validation failure has its own error code.
    let mut long_description = create_body("description", 30);
    long_description["description"] = json!("d".repeat(257));

    for (body, code) in [
        (create_body("past", -1), "RE_2105"),
        (create_body("   ", 30), "RE_2102"),
        (create_body(&"n".repeat(65), 30), "RE_2103"),
        (long_description, "RE_2104"),
    ] {
        let response = client
            .post("/api_keys")
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        assert_eq!(response.status(), 400, "{body}");
        assert_eq!(error_code(response).await, code, "{body}");
    }

    // Unparsable expiry values are rejected (a key that never expires omits the field).
    for body in [
        json!({ "name": "bad", "expires_at": "forever" }),
        json!({ "name": "bad", "expires_at": "never" }),
    ] {
        let response = client
            .post("/api_keys")
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        assert_eq!(response.status(), 400, "{body}");
    }
}

#[tokio::test]
async fn expiry_is_optional_and_not_capped() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_expiry_merchant", "ak_expiry_profile")
        .await;

    // Missing and null both mean the key never expires.
    for body in [
        json!({ "name": "no expiry" }),
        json!({ "name": "null expiry", "expires_at": null }),
    ] {
        let response = client
            .post("/api_keys")
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        assert_eq!(response.status(), 201, "{body}");
        let created: CreateApiKeyResponse = response.json().await.unwrap();
        assert_eq!(created.api_key_details.expires_at, None, "{body}");

        let response = client
            .get(&format!("/api_keys/{}", created.api_key_details.id))
            .with_auth(&token)
            .send()
            .await;
        let fetched: Value = client.expect_success(response).await;
        assert!(fetched["expires_at"].is_null(), "{body}");
    }

    // A date is still accepted, and there is no maximum.
    let response = client
        .post("/api_keys")
        .with_auth(&token)
        .json(&create_body("five years", 5 * 365))
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let created: CreateApiKeyResponse = response.json().await.unwrap();
    assert!(created.api_key_details.expires_at.is_some());
}

#[tokio::test]
async fn management_requires_a_jwt() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, profile_id, token) = client
        .bootstrap("ak_cred_merchant", "ak_cred_profile")
        .await;
    let key = create_key(&client, &token, "cannot manage keys").await;

    // No credential, the admin key, the old `api-key` header, a valid API key, a bad token.
    for builder in [
        client.get("/api_keys"),
        client.get("/api_keys").with_admin_key(),
        client
            .get("/api_keys")
            .header("api-key", ADMIN_API_KEY)
            .header("x-profile-id", &profile_id),
        client
            .get("/api_keys")
            .header("Authorization", &format!("api-key={}", key.api_key.peek()))
            .header("x-profile-id", &profile_id),
        client.get("/api_keys").with_auth("not-a-jwt"),
    ] {
        assert_eq!(builder.send().await.status(), 401);
    }

    let response = client.get("/api_keys").with_auth(&token).send().await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn name_is_stored_as_given() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_trim_merchant", "ak_trim_profile")
        .await;

    // Trimming is only used to reject blank names, not applied to what is stored.
    let created = create_key(&client, &token, "  padded  ")
        .await
        .api_key_details;
    assert_eq!(created.name, "  padded  ");

    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({ "update_type": "details_update", "name": " renamed " }))
        .send()
        .await;
    let updated: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(updated.name, " renamed ");
}

#[tokio::test]
async fn update_changes_name_and_description_only() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_update_merchant", "ak_update_profile")
        .await;
    let created = create_key(&client, &token, "before").await.api_key_details;

    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({ "update_type": "details_update", "name": "after" }))
        .send()
        .await;
    let updated: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(updated.name, "after");
    assert_eq!(updated.description, created.description);
    assert_eq!(updated.status, ApiKeyStatus::Active);
    assert_eq!(updated.expires_at, created.expires_at);
    assert!(updated.last_modified_at >= created.last_modified_at);

    // Description-only update leaves the name alone.
    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({ "update_type": "details_update", "description": "new description" }))
        .send()
        .await;
    let updated: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(updated.name, "after");
    assert_eq!(updated.description.as_deref(), Some("new description"));

    // `expires_at` can't be sent: extending a key's life is a new key.
    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({ "update_type": "details_update", "expires_at": expires_in_days(300) }))
        .send()
        .await;
    assert_eq!(response.status(), 400);

    // An empty update is rejected.
    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({ "update_type": "details_update" }))
        .send()
        .await;
    assert_eq!(response.status(), 400);
    assert_eq!(error_code(response).await, "RE_2108");

    // Invalid name and description.
    for (body, code) in [
        (
            json!({ "update_type": "details_update", "name": "  " }),
            "RE_2102",
        ),
        (
            json!({ "update_type": "details_update", "name": "n".repeat(65) }),
            "RE_2103",
        ),
        (
            json!({ "update_type": "details_update", "description": "d".repeat(257) }),
            "RE_2104",
        ),
    ] {
        let response = client
            .put(&format!("/api_keys/{}", created.id))
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        assert_eq!(response.status(), 400, "{body}");
        assert_eq!(error_code(response).await, code, "{body}");
    }
}

#[tokio::test]
async fn update_with_revoked_status_revokes_key_and_cannot_repeat() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_revoke_merchant", "ak_revoke_profile")
        .await;
    let id = create_key(&client, &token, "to revoke")
        .await
        .api_key_details
        .id;

    // Reactivation is not a thing: no update carries a status.
    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .json(&json!({ "update_type": "details_update", "status": "active" }))
        .send()
        .await;
    // Not a field of either update, so serde rejects it before the handler.
    assert_eq!(response.status(), 400);

    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .json(&json!({ "update_type": "revoke_update" }))
        .send()
        .await;
    let revoked: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(revoked.status, ApiKeyStatus::Revoked);
    assert_eq!(revoked.name, "to revoke");
    assert!(revoked.last_modified_at >= revoked.created_at);

    let response = client
        .get(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 404);

    let list: ListApiKeysResponse = client
        .expect_success(client.get("/api_keys").with_auth(&token).send().await)
        .await;
    assert!(list.api_keys.iter().all(|key| key.id != id));

    // A revoked key can't be revoked again, revived, or renamed.
    for (body, expected) in [
        (json!({ "update_type": "revoke_update" }), 404),
        (
            json!({ "update_type": "details_update", "status": "active" }),
            400,
        ),
        (
            json!({ "update_type": "details_update", "name": "revive" }),
            404,
        ),
    ] {
        let response = client
            .put(&format!("/api_keys/{id}"))
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        // `status` is rejected while parsing, everything else matches no active key.
        assert_eq!(response.status(), expected, "{body}");
    }
}

#[tokio::test]
async fn status_cannot_be_updated_together_with_details() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_both_merchant", "ak_both_profile")
        .await;
    let id = create_key(&client, &token, "before")
        .await
        .api_key_details
        .id;

    for body in [
        json!({ "update_type": "revoke_update", "name": "final name" }),
        json!({ "update_type": "revoke_update", "description": "final description" }),
        json!({ "update_type": "details_update", "name": "final name", "status": "revoked" }),
    ] {
        let response = client
            .put(&format!("/api_keys/{id}"))
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        // Neither update variant accepts this mix, so it is a parse error.
        assert_eq!(response.status(), 400, "{body}");
    }

    // The rejected requests changed nothing.
    let response = client
        .get(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .send()
        .await;
    let fetched: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(fetched.name, "before");
    assert_eq!(fetched.status, ApiKeyStatus::Active);
}

#[tokio::test]
async fn merchants_cannot_see_or_revoke_each_others_keys() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token_a) = client
        .bootstrap("ak_iso_merchant_a", "ak_iso_profile_a")
        .await;
    let (_, _, token_b) = client
        .bootstrap("ak_iso_merchant_b", "ak_iso_profile_b")
        .await;
    let id = create_key(&client, &token_a, "a's key")
        .await
        .api_key_details
        .id;

    let response = client
        .get(&format!("/api_keys/{id}"))
        .with_auth(&token_b)
        .send()
        .await;
    assert_eq!(response.status(), 404);

    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_auth(&token_b)
        .json(&json!({ "update_type": "details_update", "name": "stolen" }))
        .send()
        .await;
    assert_eq!(response.status(), 404);

    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_auth(&token_b)
        .json(&json!({ "update_type": "revoke_update" }))
        .send()
        .await;
    assert_eq!(response.status(), 404);

    let list: ListApiKeysResponse = client
        .expect_success(client.get("/api_keys").with_auth(&token_b).send().await)
        .await;
    assert!(list.api_keys.is_empty());

    // Still active for its owner.
    let response = client
        .get(&format!("/api_keys/{id}"))
        .with_auth(&token_a)
        .send()
        .await;
    let fetched: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(fetched.status, ApiKeyStatus::Active);
}

#[tokio::test]
async fn staging_entry_v2_create_accepts_an_api_key_without_modified_by() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, profile_id, token) = client
        .bootstrap("ak_staging_merchant", "ak_staging_profile")
        .await;
    let account_id = create_test_account(
        &client,
        &token,
        "ak staging account",
        BalanceDirection::Debit,
    )
    .await;
    let transformation = create_transformation_config(
        &profile_id,
        &account_id,
        None,
        &merchant_id,
        &token,
        &client,
        EntryField::OrderId,
        StagingEntryProcessingMode::Transaction,
        vec![],
    )
    .await;
    let key = create_key(&client, &token, "creates staging entries").await;
    let authorization = format!("api-key={}", key.api_key.peek());

    let request = StagingEntryCreateRequestV2 {
        account_id: account_id.clone(),
        transformation_id: transformation.transformation_id.clone(),
        metadata_schema_id: transformation.metadata_schema_id.clone(),
        raw_data: HashMap::from([
            (EntryField::Amount.to_string(), Some("100".to_string())),
            (EntryField::Currency.to_string(), Some("USD".to_string())),
            (
                EntryField::EffectiveAt.to_string(),
                Some("12-12-2025".to_string()),
            ),
            (
                EntryField::EntryType.to_string(),
                Some(BalanceDirection::Debit.to_string()),
            ),
            (
                EntryField::OrderId.to_string(),
                Some("ak_order_1".to_string()),
            ),
        ]),
        transformation_history_id: None,
    };

    // A key creates the entry. There is no user to record, so `modified_by` is empty.
    let response = client
        .post("/staging_entries/v2")
        .header("Authorization", &authorization)
        .header("x-profile-id", &profile_id)
        .json(&request)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let created: GetStagingEntryResponse = client.expect_success(response).await;
    assert!(created.modified_by.is_none());

    // A JWT can read it back.
    let response = client
        .get(&format!("/staging_entries/{}", created.id))
        .with_auth(&token)
        .send()
        .await;
    let fetched: GetStagingEntryResponse = client.expect_success(response).await;
    assert!(fetched.modified_by.is_none());

    // The v1 create route is not enabled for keys.
    let v1_request = staging_entry_request(
        &account_id,
        BalanceDirection::Debit,
        StagingEntryProcessingMode::Transaction,
        100.0,
        Currency::USD,
        transformation.transformation_id,
    );
    let response = client
        .post("/staging_entries")
        .header("Authorization", &authorization)
        .header("x-profile-id", &profile_id)
        .json(&v1_request)
        .send()
        .await;
    assert_eq!(response.status(), 401);
}

fn list_transactions_request() -> Value {
    json!({
        "filters": {},
        "cursor_payload": { "limit": 10, "sort_by": { "sort_field": "id", "id": null } },
    })
}

#[tokio::test]
async fn transactions_list_v2_validates_the_api_key_and_profile() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, profile_id, token) = client
        .bootstrap("ak_list_merchant", "ak_list_profile")
        .await;
    let (_, other_profile_id, _) = client
        .bootstrap("ak_list_other_merchant", "ak_list_other_profile")
        .await;
    let key = create_key(&client, &token, "lists transactions").await;
    let plaintext = key.api_key.peek().clone();
    let authorization = format!("api-key={plaintext}");

    let response = client
        .post("/transactions/v2/list")
        .header("Authorization", &authorization)
        .header("x-profile-id", &profile_id)
        .json(&list_transactions_request())
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let admin_authorization = format!("admin-api-key={ADMIN_API_KEY}");
    let bearer_key = format!("Bearer {plaintext}");
    for (authorization, profile) in [
        // The profile is mandatory, must exist and must belong to the key's merchant.
        (Some(authorization.as_str()), None),
        (
            Some(authorization.as_str()),
            Some(other_profile_id.as_str()),
        ),
        (Some(authorization.as_str()), Some("profile_does_not_exist")),
        // Unknown, empty and unprefixed keys.
        (
            Some("api-key=dev_not_a_real_key"),
            Some(profile_id.as_str()),
        ),
        (Some("api-key="), Some(profile_id.as_str())),
        (Some(plaintext.as_str()), Some(profile_id.as_str())),
        // A `Bearer` value is only ever read as a JWT.
        (Some(bearer_key.as_str()), Some(profile_id.as_str())),
        (
            Some(admin_authorization.as_str()),
            Some(profile_id.as_str()),
        ),
        (None, Some(profile_id.as_str())),
    ] {
        let mut builder = client.post("/transactions/v2/list");
        if let Some(authorization) = authorization {
            builder = builder.header("Authorization", authorization);
        }
        if let Some(profile) = profile {
            builder = builder.header("x-profile-id", profile);
        }
        let response = builder.json(&list_transactions_request()).send().await;
        // The key itself is never printed.
        assert_eq!(
            response.status(),
            401,
            "authorization set: {}, profile: {profile:?}",
            authorization.is_some()
        );
    }

    // The old `api-key` header is not read.
    let response = client
        .post("/transactions/v2/list")
        .header("api-key", &plaintext)
        .header("x-profile-id", &profile_id)
        .json(&list_transactions_request())
        .send()
        .await;
    assert_eq!(response.status(), 401);

    // A revoked key stops working on the next request.
    let response = client
        .put(&format!("/api_keys/{}", key.api_key_details.id))
        .with_auth(&token)
        .json(&json!({ "update_type": "revoke_update" }))
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let response = client
        .post("/transactions/v2/list")
        .header("Authorization", &authorization)
        .header("x-profile-id", &profile_id)
        .json(&list_transactions_request())
        .send()
        .await;
    assert_eq!(response.status(), 401);
}
