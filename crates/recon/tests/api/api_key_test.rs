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
        "scopes": ["ProfileReconTransactionRead"],
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

#[tokio::test]
async fn create_rejects_invalid_expiry_and_name() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_invalid_merchant", "ak_invalid_profile")
        .await;

    for body in [
        create_body("past", -1),
        json!({ "name": "bad", "expires_at": "forever" }),
        json!({ "name": "bad", "expires_at": "never" }),
        json!({ "name": "bad", "expires_at": null }),
        create_body("too long", 366),
        create_body("   ", 30),
        create_body(&"n".repeat(65), 30),
    ] {
        let response = client
            .post("/api_keys")
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        assert_eq!(response.status(), 400, "{body}");
    }

    // Expiry is mandatory.
    let response = client
        .post("/api_keys")
        .with_auth(&token)
        .json(&json!({ "name": "no expiry" }))
        .send()
        .await;
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn create_validates_scopes() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_scopes_merchant", "ak_scopes_profile")
        .await;

    let read = "ProfileReconTransactionRead";
    for scopes in [
        json!([]),
        json!([read, read]),
        json!(["NotAPermission"]),
        // Real permissions that are not in the allowlist.
        json!(["ProfileReconTransactionWrite"]),
        json!(["ProfileReconIngestionRead"]),
        json!(["ProfileReconStagingEntryRead"]),
        // Keys can't mint keys.
        json!(["MerchantReconApiKeyWrite"]),
        json!(["MerchantReconApiKeyRead"]),
    ] {
        let mut body = create_body("scoped", 30);
        body["scopes"] = scopes.clone();
        let response = client
            .post("/api_keys")
            .with_auth(&token)
            .json(&body)
            .send()
            .await;
        assert_eq!(response.status(), 400, "{scopes}");
    }

    // Staging entry create is allowlisted.
    let mut body = create_body("staging writer", 30);
    body["scopes"] = json!([read, "ProfileReconStagingEntryWrite"]);
    let response = client
        .post("/api_keys")
        .with_auth(&token)
        .json(&body)
        .send()
        .await;
    assert_eq!(response.status(), 201);

    // Scopes are required.
    let mut body = create_body("no scopes", 30);
    body.as_object_mut().unwrap().remove("scopes");
    let response = client
        .post("/api_keys")
        .with_auth(&token)
        .json(&body)
        .send()
        .await;
    assert_eq!(response.status(), 400);

    // A valid scope is stored and returned by create and get.
    let created = create_key(&client, &token, "valid").await;
    assert_eq!(created.api_key_details.scopes, vec![read.to_string()]);
    let fetched: ApiKeyResponse = client
        .get(&format!("/api_keys/{}", created.api_key_details.id))
        .with_auth(&token)
        .send()
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(fetched.scopes, vec![read.to_string()]);
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
async fn management_requires_a_credential() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, _, token) = client
        .bootstrap("ak_cred_merchant", "ak_cred_profile")
        .await;

    let response = client.get("/api_keys").send().await;
    assert_eq!(response.status(), 401);

    // Wrong admin key.
    let response = client
        .get("/api_keys")
        .header("Authorization", "admin-api-key=wrong")
        .header("x-merchant-id", merchant_id.get_string_repr())
        .send()
        .await;
    assert_eq!(response.status(), 401);

    let response = client
        .get("/api_keys")
        .header(
            "Authorization",
            "admin-api-key=test_admin_12345678901234567890,api-key=dev_abc",
        )
        .header("x-merchant-id", merchant_id.get_string_repr())
        .send()
        .await;
    assert_eq!(response.status(), 401);

    let response = client
        .get("/api_keys")
        .header("Authorization", &format!("Bearer {token}, admin-api-key=x"))
        .send()
        .await;
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn credentials_are_only_read_from_the_authorization_header() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_id, _, token) = client.bootstrap("ak_hdr_merchant", "ak_hdr_profile").await;
    let admin_key = "test_admin_12345678901234567890";

    let response = client
        .get("/api_keys")
        .header("Authorization", &format!("admin-api-key={admin_key}"))
        .header("x-merchant-id", merchant_id.get_string_repr())
        .send()
        .await;
    assert_eq!(response.status(), 200);

    let response = client
        .get("/api_keys")
        .header("api-key", admin_key)
        .header("x-merchant-id", merchant_id.get_string_repr())
        .send()
        .await;
    assert_eq!(response.status(), 401);

    for authorization in ["admin-api-key=wrong", "api-key=dev_not_a_real_key"] {
        let response = client
            .get("/api_keys")
            .header("Authorization", authorization)
            .header("x-merchant-id", merchant_id.get_string_repr())
            .send()
            .await;
        assert_eq!(response.status(), 401, "{authorization}");
    }

    let response = client.get("/api_keys").with_auth(&token).send().await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn admin_key_manages_keys_for_the_merchant_in_the_header() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (merchant_a, _, token_a) = client
        .bootstrap("ak_admin_merchant_a", "ak_admin_profile_a")
        .await;
    let (merchant_b, _, token_b) = client
        .bootstrap("ak_admin_merchant_b", "ak_admin_profile_b")
        .await;
    let a = merchant_a.get_string_repr().to_string();

    // The merchant header is mandatory.
    let response = client
        .post("/api_keys")
        .with_admin_key()
        .json(&create_body("no header", 30))
        .send()
        .await;
    assert_eq!(response.status(), 401);

    // Unknown merchant.
    let response = client
        .post("/api_keys")
        .with_admin_key()
        .header("x-merchant-id", "no_such_merchant")
        .json(&create_body("unknown", 30))
        .send()
        .await;
    assert_eq!(response.status(), 401);

    let response = client
        .post("/api_keys")
        .with_admin_key()
        .header("x-merchant-id", &a)
        .json(&create_body("by admin", 30))
        .send()
        .await;
    assert_eq!(response.status(), 201);
    let created: CreateApiKeyResponse = response.json().await.unwrap();
    assert_eq!(created.api_key_details.merchant_id, merchant_a);
    assert_eq!(created.api_key_details.created_by, None);
    let id = created.api_key_details.id;

    // Visible to the merchant's own JWT, not to another merchant.
    let response = client
        .get(&format!("/api_keys/{id}"))
        .with_auth(&token_a)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    let response = client
        .get(&format!("/api_keys/{id}"))
        .with_auth(&token_b)
        .send()
        .await;
    assert_eq!(response.status(), 404);

    // The header scopes the admin key too: merchant B's header can't reach A's key.
    let response = client
        .delete(&format!("/api_keys/{id}"))
        .with_admin_key()
        .header("x-merchant-id", merchant_b.get_string_repr())
        .send()
        .await;
    assert_eq!(response.status(), 404);

    let response = client
        .put(&format!("/api_keys/{id}"))
        .with_admin_key()
        .header("x-merchant-id", &a)
        .json(&json!({ "name": "renamed" }))
        .send()
        .await;
    let updated: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(updated.name, "renamed");

    let list: ListApiKeysResponse = client
        .expect_success(
            client
                .get("/api_keys")
                .with_admin_key()
                .header("x-merchant-id", &a)
                .send()
                .await,
        )
        .await;
    assert_eq!(list.api_keys.len(), 1);

    let response = client
        .delete(&format!("/api_keys/{id}"))
        .with_admin_key()
        .header("x-merchant-id", &a)
        .send()
        .await;
    let revoked: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(revoked.status, ApiKeyStatus::Revoked);
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
    assert_eq!(updated.expires_at, created.expires_at);
    assert!(updated.last_modified_at >= created.last_modified_at);

    // `expires_at` can't be sent: extending a key's life is a new key.
    let response = client
        .put(&format!("/api_keys/{}", created.id))
        .with_auth(&token)
        .json(&json!({ "expires_at": expires_in_days(300) }))
        .send()
        .await;
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn revoke_removes_key_and_cannot_repeat() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client
        .bootstrap("ak_revoke_merchant", "ak_revoke_profile")
        .await;
    let id = create_key(&client, &token, "to revoke")
        .await
        .api_key_details
        .id;

    let response = client
        .delete(&format!("/api_keys/{id}"))
        .with_auth(&token)
        .send()
        .await;
    let revoked: ApiKeyResponse = client.expect_success(response).await;
    assert_eq!(revoked.status, ApiKeyStatus::Revoked);

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

    for response in [
        client
            .delete(&format!("/api_keys/{id}"))
            .with_auth(&token)
            .send()
            .await,
        client
            .put(&format!("/api_keys/{id}"))
            .with_auth(&token)
            .json(&json!({ "name": "revive" }))
            .send()
            .await,
    ] {
        assert_eq!(response.status(), 404);
    }
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
        .delete(&format!("/api_keys/{id}"))
        .with_auth(&token_b)
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
    assert_eq!(response.status(), 200);
}
