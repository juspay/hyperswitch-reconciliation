use crate::errors::business_profile::BusinessProfileError;
use crate::errors::merchant_account::MerchantAccountError;
use crate::{
    db::{business_profile::BusinessProfileInterface, merchant_account::MerchantAccountInterface},
    errors::ReconAppError,
    routes::AppState,
    types::{ApplicationResponse, ReconResponse},
};
use actix_web::web;
use api_models::auth::{
    TestMerchantSetupRequest, TestMerchantSetupResponse, TestMerchantSetupWithIdsRequest,
};
use common_utils::{
    date_time, id_generation::generate_id_with_default_len, id_type::MerchantId,
    types::keymanager::Identifier, GenerateId,
};
use diesel_models::{
    business_profile::BusinessProfileNew,
    merchant_account::MerchantAccountNew as DieselMerchantAccountNew,
};
use error_stack::{report, ResultExt};
use recon_domain_models::ApplicationEncryptionInterface;

pub async fn bootstrap(
    state: AppState,
    request: web::Json<TestMerchantSetupRequest>,
) -> ReconResponse<TestMerchantSetupResponse> {
    // 1. Create merchant account
    if request.merchant_name.is_empty() {
        return Err(report!(ReconAppError::from(
            MerchantAccountError::MerchantNameEmpty
        )));
    }

    let merchant_id = MerchantId::generate();
    state
        .encryption_client
        .setup_key_for_identifier(&Identifier::ReconMerchant(merchant_id.clone()))
        .await
        .change_context(ReconAppError::InternalServerError)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id.clone());

    let diesel_merchant_account_new = DieselMerchantAccountNew {
        merchant_id,
        merchant_name: merchant_encryptor
            .encrypt_string(request.merchant_name.clone())
            .await
            .change_context(ReconAppError::InternalServerError)?
            .into(),
        created_at: date_time::now(),
        last_modified_at: date_time::now(),
    };

    let merchant_account = state
        .store
        .insert_merchant_account(diesel_merchant_account_new)
        .await
        .change_context(MerchantAccountError::InternalServerError.into())?;

    // 2. Create business profile
    if request.profile_name.is_empty() {
        return Err(report!(ReconAppError::from(
            BusinessProfileError::ProfileNameEmpty
        )));
    }

    let profile_id = generate_id_with_default_len("profile");

    let diesel_business_profile_new = BusinessProfileNew {
        profile_id,
        profile_name: merchant_encryptor
            .encrypt_string(request.profile_name.clone())
            .await
            .change_context(ReconAppError::InternalServerError)?
            .into(),
        merchant_id: merchant_account.merchant_id.clone(),
        created_at: date_time::now(),
        last_modified_at: date_time::now(),
    };

    let business_profile = state
        .store
        .insert_business_profile(diesel_business_profile_new)
        .await
        .change_context(BusinessProfileError::InternalServerError.into())?;

    Ok(ApplicationResponse::json(TestMerchantSetupResponse {
        merchant_id: merchant_account.merchant_id,
        profile_id: business_profile.profile_id,
    }))
}

pub async fn bootstrap_with_merchant_id_profile_id(
    state: AppState,
    request: web::Json<TestMerchantSetupWithIdsRequest>,
) -> ReconResponse<TestMerchantSetupResponse> {
    // 1. Validate input fields
    if request.merchant_name.is_empty() {
        return Err(report!(ReconAppError::from(
            MerchantAccountError::MerchantNameEmpty
        )));
    }

    if request.profile_name.is_empty() {
        return Err(report!(ReconAppError::from(
            BusinessProfileError::ProfileNameEmpty
        )));
    }

    if request.profile_id.is_empty() {
        return Err(report!(ReconAppError::from(
            BusinessProfileError::ProfileIdEmpty
        )));
    }

    let merchant_id = request.merchant_id.clone();

    // Create both merchant and profile
    state
        .encryption_client
        .setup_key_for_identifier(&Identifier::ReconMerchant(merchant_id.clone()))
        .await
        .change_context(ReconAppError::InternalServerError)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id.clone());

    // 2. Check if merchant_id already exists
    let merchant_result = state.store.find_merchant_account_by_id(&merchant_id).await;

    // 3. Check if profile_id already exists
    let profile_result = state
        .store
        .find_business_profile_by_id(&request.profile_id)
        .await;

    // 4. Handle existence checks
    let merchant_account = if let Ok(merchant) = merchant_result {
        // Merchant exists, check if profile exists
        if let Ok(_profile) = profile_result {
            // Both exist - throw error
            return Err(report!(ReconAppError::ResourceConflict(format!(
                "Both merchant '{}' and profile '{}' already exist",
                merchant_id.get_string_repr(),
                request.profile_id
            ))));
        }
        // Merchant exists, profile doesn't - return existing merchant
        merchant
    } else {
        // Merchant doesn't exist, check if profile exists (edge case)
        if let Ok(_profile) = profile_result {
            return Err(report!(ReconAppError::ResourceConflict(format!(
                "Profile '{}' already exists but merchant '{}' does not",
                request.profile_id,
                merchant_id.get_string_repr()
            ))));
        }

        let diesel_merchant_account_new = DieselMerchantAccountNew {
            merchant_id: merchant_id.clone(),
            merchant_name: merchant_encryptor
                .encrypt_string(request.merchant_name.clone())
                .await
                .change_context(ReconAppError::InternalServerError)?
                .into(),
            created_at: date_time::now(),
            last_modified_at: date_time::now(),
        };

        state
            .store
            .insert_merchant_account(diesel_merchant_account_new)
            .await
            .change_context(MerchantAccountError::InternalServerError.into())?
    };

    // Get merchant encryptor (works for both existing and new merchants)
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id);

    // 5. Create business profile with provided profile_id
    let diesel_business_profile_new = BusinessProfileNew {
        profile_id: request.profile_id.clone(),
        profile_name: merchant_encryptor
            .encrypt_string(request.profile_name.clone())
            .await
            .change_context(ReconAppError::InternalServerError)?
            .into(),
        merchant_id: merchant_account.merchant_id.clone(),
        created_at: date_time::now(),
        last_modified_at: date_time::now(),
    };

    let business_profile = state
        .store
        .insert_business_profile(diesel_business_profile_new)
        .await
        .change_context(BusinessProfileError::InternalServerError.into())?;

    Ok(ApplicationResponse::json(TestMerchantSetupResponse {
        merchant_id: merchant_account.merchant_id,
        profile_id: business_profile.profile_id,
    }))
}
