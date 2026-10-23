use api_models::link_config::{
    link_matching_key::{LinkIdentifierKey, SingleLinkIdentifier},
    link_trigger::LinkTriggerGroup,
    GetLinkConfigResponse, LinkConfigCreateRequest, LinkSearchIdentifier, LinkTrigger,
};
use common_enums::BalanceDirection;
use common_types::recon_rule::entry_field::EntryField;
use serde_json::{json, Value};

use crate::{
    api::{
        account_test::create_test_account,
        recon_engine_test::types::{ReconRuleCustomConfig, ReconRuleEngine},
        recon_rule_test::create_test_recon_rule,
    },
    common::{TestApp, TestClient},
};

async fn config_request(client: &TestClient, token: &str) -> LinkConfigCreateRequest {
    let account_id = create_test_account(client, token, "PSP", BalanceDirection::Debit).await;
    let target_id = create_test_account(client, token, "Bank", BalanceDirection::Credit).await;
    let mut rule_ids = Vec::new();
    for name in ["Transaction rule", "Confirmation rule"] {
        let rule = create_test_recon_rule(
            client,
            token,
            ReconRuleEngine::OneToOneSingleSingleV1(("PSP".to_owned(), "Bank".to_owned())),
            Some(name.to_owned()),
            None,
            None,
            ReconRuleCustomConfig::default(),
            vec![account_id.clone()],
            vec![target_id.clone()],
        )
        .await;
        rule_ids.push(rule.rule_id);
    }
    let trigger = LinkTrigger {
        any: vec![LinkTriggerGroup { all: vec![] }],
    };
    LinkConfigCreateRequest {
        account_id,
        transaction_rule_id: rule_ids[0].clone(),
        confirmation_rule_id: rule_ids[1].clone(),
        transaction_trigger: trigger.clone(),
        confirmation_trigger: trigger,
        link_search_identifier: LinkSearchIdentifier::Single(SingleLinkIdentifier {
            key: LinkIdentifierKey {
                transaction_field: EntryField::OrderId,
                confirmation_field: EntryField::Metadata("order_id".to_owned()),
            },
        }),
        priority: 10,
    }
}

async fn create_config(
    client: &TestClient,
    token: &str,
    request: &LinkConfigCreateRequest,
) -> Value {
    let response = client
        .post("/link_configs")
        .with_auth(token)
        .json(request)
        .send()
        .await;
    assert_eq!(response.status(), 201);
    response.json().await.unwrap()
}

async fn update_config(client: &TestClient, token: &str, id: &str, update: &Value) -> Value {
    let response = client
        .put(&format!("/link_configs/{id}"))
        .with_auth(token)
        .json(update)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[actix_web::test]
async fn link_config_create_and_update_preserve_omitted_fields() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, profile_id, token) = client.bootstrap("Link merchant", "Link profile").await;
    let request = config_request(&client, &token).await;
    let created = create_config(&client, &token, &request).await;
    let typed: GetLinkConfigResponse = serde_json::from_value(created.clone()).unwrap();
    assert_eq!(typed.profile_id, profile_id);
    assert!(typed.is_active);
    assert_eq!(typed.priority, 10);
    assert_eq!(typed.created_at, typed.last_modified_at);
    assert!(typed.link_config_id.starts_with("link_config_"));
    for (field, value) in serde_json::to_value(&request).unwrap().as_object().unwrap() {
        assert_eq!(&created[field], value);
    }

    let id = &typed.link_config_id;
    assert_eq!(
        update_config(&client, &token, id, &json!({})).await,
        created
    );
    let updated = update_config(
        &client,
        &token,
        id,
        &json!({"priority": 0, "is_active": false}),
    )
    .await;
    let mut expected = created.clone();
    expected["priority"] = json!(0);
    expected["is_active"] = json!(false);
    expected["last_modified_at"] = updated["last_modified_at"].clone();
    assert_eq!(updated, expected);
    let typed_update: GetLinkConfigResponse = serde_json::from_value(updated.clone()).unwrap();
    assert!(typed_update.last_modified_at > typed.last_modified_at);
    assert_eq!(
        update_config(&client, &token, id, &json!({})).await,
        updated
    );

    let replacement = config_request(&client, &token).await;
    let mut full_update = serde_json::to_value(replacement).unwrap();
    full_update.as_object_mut().unwrap().remove("account_id");
    full_update["priority"] = json!(255);
    full_update["is_active"] = json!(true);
    full_update["transaction_trigger"] = json!({
        "any": [{"all": [
            {"field": "currency", "operator": "equals", "value": "USD"},
            {"field": "metadata.type", "operator": "not_equals", "value": "refund"}
        ]}]
    });
    full_update["confirmation_trigger"] = json!({"any": []});
    full_update["link_search_identifier"] = json!({
        "matching_key_type": "composite",
        "keys": [
            {"transaction_field": "order_id", "confirmation_field": "metadata.order_id"},
            {"transaction_field": "currency", "confirmation_field": "currency"}
        ],
        "delimiter": "pipe"
    });
    let updated = update_config(&client, &token, id, &full_update).await;
    for (field, value) in full_update.as_object().unwrap() {
        assert_eq!(&updated[field], value);
    }
    for field in ["link_config_id", "profile_id", "account_id", "created_at"] {
        assert_eq!(updated[field], created[field]);
    }
    assert_eq!(
        update_config(&client, &token, id, &json!({})).await,
        updated
    );
}

#[actix_web::test]
async fn link_config_rejects_missing_and_foreign_profile_resources() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Owner", "Owner profile").await;
    let (_, _, other_token) = client.bootstrap("Other", "Other profile").await;
    let request = config_request(&client, &token).await;
    let foreign = serde_json::to_value(config_request(&client, &other_token).await).unwrap();
    let created = create_config(&client, &token, &request).await;
    let id = created["link_config_id"].as_str().unwrap();

    for (auth, config_id) in [(&other_token, id), (&token, "missing_config")] {
        let response = client
            .put(&format!("/link_configs/{config_id}"))
            .with_auth(auth)
            .json(&json!({"is_active": false}))
            .send()
            .await;
        client.expect_error(response, 404).await;
    }

    for field in ["account_id", "transaction_rule_id", "confirmation_rule_id"] {
        for invalid_value in [json!("missing_resource"), foreign[field].clone()] {
            let mut invalid_create = serde_json::to_value(&request).unwrap();
            invalid_create[field] = invalid_value;
            let response = client
                .post("/link_configs")
                .with_auth(&token)
                .json(&invalid_create)
                .send()
                .await;
            client.expect_error(response, 404).await;
        }
    }

    for field in ["transaction_rule_id", "confirmation_rule_id"] {
        for invalid_value in [json!("missing_resource"), foreign[field].clone()] {
            let mut invalid_update = json!({"priority": 99, "is_active": false});
            invalid_update[field] = invalid_value;
            let response = client
                .put(&format!("/link_configs/{id}"))
                .with_auth(&token)
                .json(&invalid_update)
                .send()
                .await;
            client.expect_error(response, 404).await;
            assert_eq!(
                update_config(&client, &token, id, &json!({})).await,
                created
            );
        }
    }
}

#[actix_web::test]
async fn link_config_rejects_invalid_configuration_and_missing_auth() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Validation", "Validation profile").await;
    let request = config_request(&client, &token).await;
    let created = create_config(&client, &token, &request).await;
    let id = created["link_config_id"].as_str().unwrap();

    let mut invalid_account = request.clone();
    invalid_account.account_id = " ".to_owned();
    let response = client
        .post("/link_configs")
        .with_auth(&token)
        .json(&invalid_account)
        .send()
        .await;
    assert_eq!(response.status(), 404);

    for (invalid, expected_code) in [
        (json!({"transaction_rule_id": ""}), 404),
        (json!({"confirmation_rule_id": " "}), 404),
        (
            json!({"link_search_identifier": {
                "matching_key_type": "composite",
                "keys": [], "delimiter": "pipe"
            }}),
            400,
        ),
        (json!({"transaction_trigger": {"any": "invalid"}}), 400),
        (json!({"priority": 256}), 400),
        (json!({"priority": -1}), 400),
    ] {
        let mut invalid_create = serde_json::to_value(&request).unwrap();
        invalid_create
            .as_object_mut()
            .unwrap()
            .extend(invalid.as_object().unwrap().clone());
        let response = client
            .post("/link_configs")
            .with_auth(&token)
            .json(&invalid_create)
            .send()
            .await;
        assert_eq!(response.status(), expected_code);

        let response = client
            .put(&format!("/link_configs/{id}"))
            .with_auth(&token)
            .json(&invalid)
            .send()
            .await;
        assert_eq!(response.status(), expected_code);
        assert_eq!(
            update_config(&client, &token, id, &json!({})).await,
            created
        );
    }

    let response = client.post("/link_configs").json(&request).send().await;
    client.expect_error(response, 401).await;
    let response = client
        .put(&format!("/link_configs/{id}"))
        .json(&json!({"is_active": false}))
        .send()
        .await;
    client.expect_error(response, 401).await;
    let response = client
        .post("/link_configs")
        .with_auth("invalid-token")
        .json(&request)
        .send()
        .await;
    client.expect_error(response, 401).await;
}

#[actix_web::test]
async fn link_config_get_and_list_are_profile_scoped() {
    let app = TestApp::new().await;
    let client = app.api_client();
    let (_, _, token) = client.bootstrap("Read owner", "Read owner profile").await;
    let (_, _, other_token) = client.bootstrap("Read other", "Read other profile").await;
    let mut request = config_request(&client, &token).await;
    let created = create_config(&client, &token, &request).await;
    let id = created["link_config_id"].as_str().unwrap();

    let response = client
        .get(&format!("/link_configs/{id}"))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.json::<Value>().await.unwrap(), created);

    let response = client
        .get("/link_configs")
        .with_auth(&other_token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.json::<Value>().await.unwrap(), json!([]));

    for (auth, config_id) in [(&other_token, id), (&token, "missing_config")] {
        let response = client
            .get(&format!("/link_configs/{config_id}"))
            .with_auth(auth)
            .send()
            .await;
        client.expect_error(response, 404).await;
    }

    let inactive = update_config(&client, &token, id, &json!({"is_active": false})).await;
    let response = client
        .get(&format!("/link_configs/{id}"))
        .with_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.json::<Value>().await.unwrap(), inactive);

    request.priority = 1;
    let active = create_config(&client, &token, &request).await;
    let foreign_request = config_request(&client, &other_token).await;
    let foreign = create_config(&client, &other_token, &foreign_request).await;
    for (auth, expected) in [
        (&token, json!([active, inactive])),
        (&other_token, json!([foreign])),
    ] {
        let response = client.get("/link_configs").with_auth(auth).send().await;
        assert_eq!(response.status(), 200);
        assert_eq!(response.json::<Value>().await.unwrap(), expected);
    }

    for path in ["/link_configs".to_owned(), format!("/link_configs/{id}")] {
        let response = client.get(&path).send().await;
        client.expect_error(response, 401).await;
        let response = client.get(&path).with_auth("invalid-token").send().await;
        client.expect_error(response, 401).await;
    }
}

#[test]
fn link_config_openapi_documents_routes_and_models() {
    let document = serde_json::to_value(recon::openapi::create_openapi()).unwrap();
    assert!(document["paths"]["/link_configs"]["post"].is_object());
    assert!(document["paths"]["/link_configs/{link_config_id}"]["put"].is_object());
    assert!(document["paths"]["/link_configs"]["get"].is_object());
    assert!(document["paths"]["/link_configs/{link_config_id}"]["get"].is_object());
    for schema in [
        "LinkConfigCreateRequest",
        "LinkConfigUpdateRequest",
        "GetLinkConfigResponse",
        "LinkTrigger",
        "LinkSearchIdentifier",
    ] {
        assert!(document["components"]["schemas"][schema].is_object());
    }
}
