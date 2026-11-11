use api_models::api_key::{ApiKeyResponse, CreateApiKeyResponse, ListApiKeysResponse};
use common_enums::ApiKeyStatus;
use common_utils::date_time;
use masking::PeekInterface;
use serde_json::{json, Value};
use time::Duration;

use crate::common::TestApp;

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
        (create_body("past", -1), "RE_1705"),
        (create_body("too long", 366), "RE_1706"),
        (create_body("   ", 30), "RE_1702"),
        (create_body(&"n".repeat(65), 30), "RE_1703"),
        (long_description, "RE_1704"),
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

    // Unparsable, "never", null and missing expiry are all rejected: expiry is mandatory.
    for body in [
        json!({ "name": "bad", "expires_at": "forever" }),
        json!({ "name": "bad", "expires_at": "never" }),
        json!({ "name": "bad", "expires_at": null }),
        json!({ "name": "no expiry" }),
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
async fn expiry_is_capped_at_one_year() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("ak_cap_merchant", "ak_cap_profile").await;

    // Just inside the cap (a day of slack for clock drift between test and server).
    let response = client
        .post("/api_keys")
        .with_auth(&token)
        .json(&create_body("almost a year", 364))
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let created: CreateApiKeyResponse = response.json().await.unwrap();

    let response = client
        .get(&format!("/api_keys/{}", created.api_key_details.id))
        .with_auth(&token)
        .send()
        .await;
    let fetched: Value = client.expect_success(response).await;
    assert!(fetched["expires_at"].is_string());

    let response = client
        .post("/api_keys")
        .with_auth(&token)
        .json(&create_body("two years", 730))
        .send()
        .await;
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn management_requires_a_jwt() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, _, _) = client
        .bootstrap("ak_cred_merchant", "ak_cred_profile")
        .await;

    let response = client.get("/api_keys").send().await;
    assert_eq!(response.status(), 401);

    // The admin API key is not accepted on these endpoints, with or without a merchant header.
    let response = client.get("/api_keys").with_admin_key().send().await;
    assert_eq!(response.status(), 401);

    let response = client
        .post("/api_keys")
        .with_admin_key()
        .header("x-merchant-id", merchant_id.get_string_repr())
        .json(&create_body("by admin", 30))
        .send()
        .await;
    assert_eq!(response.status(), 401);

    // Invalid bearer token.
    let response = client.get("/api_keys").with_auth("not-a-jwt").send().await;
    assert_eq!(response.status(), 401);
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
        .json(&json!({ "name": " renamed " }))
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
        .json(&json!({ "name": "after" }))
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
        .json(&json!({ "description": "new description" }))
        .send()
        .await;
    let updated: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(updated.name, "after");
    assert_eq!(updated.description.as_deref(), Some("new description"));

    // `expires_at` can't be sent: extending a key's life is a new key.
    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({ "expires_at": expires_in_days(300) }))
        .send()
        .await;
    assert_eq!(response.status(), 400);

    // An empty update is rejected.
    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), 400);
    assert_eq!(error_code(response).await, "RE_1708");

    // Invalid name and description.
    for (body, code) in [
        (json!({ "name": "  " }), "RE_1702"),
        (json!({ "name": "n".repeat(65) }), "RE_1703"),
        (json!({ "description": "d".repeat(257) }), "RE_1704"),
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

    // Reactivation is not a thing: only `revoked` is accepted.
    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .json(&json!({ "status": "active" }))
        .send()
        .await;
    // Not a variant of the update request, so serde rejects it before the handler.
    assert_eq!(response.status(), 400);

    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .json(&json!({ "status": "revoked" }))
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
    for body in [
        json!({ "status": "revoked" }),
        json!({ "status": "active" }),
        json!({ "name": "revive" }),
    ] {
        let response = client
            .put(&format!("/api_keys/{id}"))
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        // `active` is rejected while parsing, everything else matches no active key.
        let expected = if body["status"] == "active" { 400 } else { 404 };
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
        json!({ "name": "final name", "status": "revoked" }),
        json!({ "description": "final description", "status": "revoked" }),
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
        .json(&json!({ "name": "stolen" }))
        .send()
        .await;
    assert_eq!(response.status(), 404);

    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_auth(&token_b)
        .json(&json!({ "status": "revoked" }))
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
