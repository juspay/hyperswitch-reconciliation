//! Data-layer tests for the `api_key` table. These pin the rules the auth layer will rely on:
//! revoked keys are invisible, keys never cross merchants, and a revoked key can't be revived.

use common_enums::ApiKeyStatus;
use common_utils::{
    date_time,
    id_generation::generate_id_with_default_len,
    id_type::{GenerateId, MerchantId},
};
use diesel_models::{
    api_key::{ApiKey, ApiKeyNew, ApiKeyUpdate},
    errors::DatabaseError,
};
use masking::{PeekInterface, Secret};
use pretty_assertions::assert_eq;
use recon::db::api_key::ApiKeyInterface;
use time::Duration;

use crate::common::test_store;

fn new_api_key(merchant_id: &MerchantId) -> ApiKeyNew {
    let now = date_time::now();

    ApiKeyNew {
        id: generate_id_with_default_len("apikey"),
        merchant_id: merchant_id.clone(),
        name: "Nightly upload".to_string(),
        description: Some("Used by the finance export job".to_string()),
        // Unique per key, as a real keyed hash would be.
        hashed_api_key: Secret::new(generate_id_with_default_len("hash")),
        prefix: "recon_dev_ab12cd".to_string(),
        created_by: "user_abc".to_string(),
        created_at: now,
        expires_at: Some(now + Duration::days(30)),
        last_modified_at: now,
        status: ApiKeyStatus::Active,
    }
}

fn hash_of(api_key: &ApiKey) -> String {
    api_key.hashed_api_key.peek().clone()
}

#[actix_web::test]
async fn inserted_key_is_found_by_hash_with_all_fields() {
    let store = test_store().await;
    let merchant_id = MerchantId::generate();
    let new_key = new_api_key(&merchant_id);

    let inserted = store.insert_api_key(new_key).await.unwrap();
    let found = store
        .find_active_api_key_by_hashed_api_key(&hash_of(&inserted))
        .await
        .unwrap()
        .expect("key should be found by its hash");

    assert_eq!(found.id, inserted.id);
    assert_eq!(found.merchant_id, merchant_id);
    assert_eq!(found.created_by, "user_abc");
    assert_eq!(found.status, ApiKeyStatus::Active);
}

#[actix_web::test]
async fn unknown_and_revoked_keys_are_not_found_by_hash() {
    let store = test_store().await;
    let merchant_id = MerchantId::generate();
    let key = store
        .insert_api_key(new_api_key(&merchant_id))
        .await
        .unwrap();

    assert!(store
        .find_active_api_key_by_hashed_api_key("no_such_hash")
        .await
        .unwrap()
        .is_none());

    store
        .update_api_key(&merchant_id, &key.id, ApiKeyUpdate::Revoke)
        .await
        .unwrap();

    assert!(store
        .find_active_api_key_by_hashed_api_key(&hash_of(&key))
        .await
        .unwrap()
        .is_none());
}

#[actix_web::test]
async fn duplicate_hash_is_rejected() {
    let store = test_store().await;
    let merchant_id = MerchantId::generate();
    let first = store
        .insert_api_key(new_api_key(&merchant_id))
        .await
        .unwrap();

    let mut duplicate = new_api_key(&merchant_id);
    duplicate.hashed_api_key = first.hashed_api_key.clone();
    let error = store.insert_api_key(duplicate).await.unwrap_err();

    assert!(matches!(
        error.current_context(),
        DatabaseError::UniqueViolation
    ));
}

#[actix_web::test]
async fn keys_never_cross_merchants() {
    let store = test_store().await;
    let merchant_a = MerchantId::generate();
    let merchant_b = MerchantId::generate();
    let key = store
        .insert_api_key(new_api_key(&merchant_a))
        .await
        .unwrap();

    let not_found = |error: error_stack::Report<DatabaseError>| {
        matches!(error.current_context(), DatabaseError::NotFound)
    };

    assert!(store
        .find_active_api_key_by_merchant_id_and_id(&merchant_b, &key.id)
        .await
        .map_err(not_found)
        .unwrap_err());
    assert!(store
        .update_api_key(&merchant_b, &key.id, ApiKeyUpdate::Revoke)
        .await
        .map_err(not_found)
        .unwrap_err());
    assert!(store
        .list_active_api_keys_by_merchant_id(&merchant_b)
        .await
        .unwrap()
        .is_empty());

    // Merchant A's key is untouched by merchant B's attempts.
    store
        .find_active_api_key_by_merchant_id_and_id(&merchant_a, &key.id)
        .await
        .unwrap();
}

#[actix_web::test]
async fn list_returns_only_active_keys_newest_first() {
    let store = test_store().await;
    let merchant_id = MerchantId::generate();
    let now = date_time::now();

    let mut oldest = new_api_key(&merchant_id);
    oldest.created_at = now - Duration::hours(2);
    let mut newest = new_api_key(&merchant_id);
    newest.created_at = now - Duration::hours(1);
    let revoked = new_api_key(&merchant_id);

    let oldest = store.insert_api_key(oldest).await.unwrap();
    let newest = store.insert_api_key(newest).await.unwrap();
    let revoked = store.insert_api_key(revoked).await.unwrap();
    store
        .update_api_key(&merchant_id, &revoked.id, ApiKeyUpdate::Revoke)
        .await
        .unwrap();

    let listed: Vec<String> = store
        .list_active_api_keys_by_merchant_id(&merchant_id)
        .await
        .unwrap()
        .into_iter()
        .map(|key| key.id)
        .collect();

    assert_eq!(listed, vec![newest.id, oldest.id]);
}

#[actix_web::test]
async fn update_changes_only_requested_fields_and_can_clear_nullable_ones() {
    let store = test_store().await;
    let merchant_id = MerchantId::generate();
    let key = store
        .insert_api_key(new_api_key(&merchant_id))
        .await
        .unwrap();

    let updated = store
        .update_api_key(
            &merchant_id,
            &key.id,
            ApiKeyUpdate::Update {
                name: Some("Renamed".to_string()),
                description: Some(None),
                expires_at: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.description, None);
    // Not requested, so untouched.
    assert_eq!(updated.expires_at, key.expires_at);
    assert_eq!(hash_of(&updated), hash_of(&key));
}

#[actix_web::test]
async fn revoked_key_cannot_be_updated_or_revived() {
    let store = test_store().await;
    let merchant_id = MerchantId::generate();
    let key = store
        .insert_api_key(new_api_key(&merchant_id))
        .await
        .unwrap();

    let revoked = store
        .update_api_key(&merchant_id, &key.id, ApiKeyUpdate::Revoke)
        .await
        .unwrap();
    assert_eq!(revoked.status, ApiKeyStatus::Revoked);
    assert!(revoked.last_modified_at >= key.last_modified_at);

    for update in [
        ApiKeyUpdate::Revoke,
        ApiKeyUpdate::Update {
            name: Some("Back from the dead".to_string()),
            description: None,
            expires_at: Some(None),
        },
    ] {
        let error = store
            .update_api_key(&merchant_id, &key.id, update)
            .await
            .unwrap_err();
        assert!(matches!(error.current_context(), DatabaseError::NotFound));
    }

    assert!(store
        .find_active_api_key_by_merchant_id_and_id(&merchant_id, &key.id)
        .await
        .is_err());
}
