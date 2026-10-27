use api_models::business_profile as api;
use api_models::business_profile::ReconStatusResponse;
use common_utils::{
    auth::AuthContext, id_generation::generate_id_with_default_len, id_type::MerchantId,
};
use diesel_models::business_profile::{BusinessProfileNew, BusinessProfileUpdate};
use error_stack::ResultExt;
use futures::future;
use masking::PeekInterface;
use recon_domain_models::{behaviour::ReverseConversion, business_profile::DomainBusinessProfile};
use storage_impl::business_profile::BusinessProfileInterface;

use crate::{
    errors::{
        business_profile::{BusinessProfileError, BusinessProfileResult},
        conversions::StorageErrorExt,
    },
    routes::AppState,
    types::ApplicationResponse,
    utils::business_profile::BusinessProfileExt,
};

pub async fn list_business_profiles_by_merchant_id(
    state: AppState,
    merchant_id: MerchantId,
) -> BusinessProfileResult<Vec<api::BusinessProfileResponse>> {
    let business_profiles = state
        .store
        .list_business_profiles_by_merchant_id(merchant_id.clone())
        .await
        .change_context(BusinessProfileError::InternalServerError)?;

    // Get encryptor once instead of per profile
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id);

    let business_profiles_futures = business_profiles
        .into_iter()
        .map(|business_profile| business_profile.convert_back(&merchant_encryptor));

    let domain_business_profiles: Vec<DomainBusinessProfile> =
        future::try_join_all(business_profiles_futures)
            .await
            .change_context(BusinessProfileError::InternalServerError)?;

    let response = domain_business_profiles
        .into_iter()
        .map(BusinessProfileExt::to_api_response)
        .collect::<Vec<api::BusinessProfileResponse>>();

    Ok(ApplicationResponse::json(response))
}

pub async fn create_business_profile(
    state: AppState,
    merchant_id: MerchantId,
    request: api::BusinessProfileCreateRequest,
) -> BusinessProfileResult<api::BusinessProfileResponse> {
    let profile_name = request.profile_name.peek().trim().to_owned();
    if profile_name.is_empty() {
        return Err(BusinessProfileError::ProfileNameEmpty.into());
    }

    let profile_id = generate_id_with_default_len("profile");

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id.clone());

    let diesel_business_profile_new = BusinessProfileNew {
        profile_id: profile_id.clone(),
        profile_name: merchant_encryptor
            .encrypt_string(profile_name)
            .await
            .change_context(BusinessProfileError::InternalServerError)?
            .into(),
        merchant_id: merchant_id.clone(),
        created_at: common_utils::date_time::now(),
        last_modified_at: common_utils::date_time::now(),
        timezone: request.timezone,
    };

    let diesel_business_profile = state
        .store
        .insert_business_profile(diesel_business_profile_new)
        .await
        .change_context(BusinessProfileError::InternalServerError)?;

    let domain_business_profile: DomainBusinessProfile = diesel_business_profile
        .convert_back(&merchant_encryptor)
        .await
        .change_context(BusinessProfileError::InternalServerError)?;

    Ok(ApplicationResponse::json_created(
        domain_business_profile.to_api_response(),
    ))
}

pub async fn get_business_profile(
    state: AppState,
    profile_id: String,
) -> BusinessProfileResult<api::BusinessProfileResponse> {
    let business_profile = state
        .store
        .find_business_profile_by_id(&profile_id)
        .await
        .change_context(BusinessProfileError::InternalServerError)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(business_profile.merchant_id.clone());

    let domain_business_profile: DomainBusinessProfile = business_profile
        .convert_back(&merchant_encryptor)
        .await
        .change_context(BusinessProfileError::InternalServerError)?;

    Ok(ApplicationResponse::json(
        domain_business_profile.to_api_response(),
    ))
}

pub async fn update_business_profile(
    state: AppState,
    merchant_id: MerchantId,
    profile_id: String,
    request: api::BusinessProfileUpdateRequest,
) -> BusinessProfileResult<api::BusinessProfileResponse> {
    let new_profile_name = request
        .profile_name
        .map(|profile_name| profile_name.peek().trim().to_owned());

    if new_profile_name.as_deref() == Some("") {
        return Err(BusinessProfileError::ProfileNameEmpty.into());
    }

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id);

    let existing_business_profile = state
        .store
        .find_business_profile_by_id(&profile_id)
        .await
        .to_not_found_response(|| BusinessProfileError::ProfileNotFound {
            profile_id: profile_id.clone(),
        })?;

    let business_profile = match new_profile_name {
        Some(profile_name) => {
            let encrypted_name = merchant_encryptor
                .encrypt_string(profile_name)
                .await
                .change_context(BusinessProfileError::InternalServerError)?
                .into();

            state
                .store
                .update_business_profile(
                    &profile_id,
                    BusinessProfileUpdate::NameUpdate {
                        profile_name: encrypted_name,
                    },
                )
                .await
                .change_context(BusinessProfileError::InternalServerError)?
        }
        None => existing_business_profile,
    };

    let business_profile = match request.timezone {
        Some(timezone) => state
            .store
            .update_business_profile(
                &profile_id,
                BusinessProfileUpdate::TimezoneUpdate { timezone },
            )
            .await
            .change_context(BusinessProfileError::InternalServerError)?,
        None => business_profile,
    };

    let domain_business_profile: DomainBusinessProfile = business_profile
        .convert_back(&merchant_encryptor)
        .await
        .change_context(BusinessProfileError::InternalServerError)?;

    Ok(ApplicationResponse::json(
        domain_business_profile.to_api_response(),
    ))
}

pub async fn get_recon_status(
    state: AppState,
    auth_context: AuthContext,
) -> BusinessProfileResult<ReconStatusResponse> {
    let processor_status = *state.recon_processor_status.borrow();

    let count = state
        .store
        .count_pending_staging_entries_by_profile(&auth_context.profile_id)
        .await
        .change_context(BusinessProfileError::InternalServerError)?;
    let pending_staging_entries =
        u64::try_from(count).change_context(BusinessProfileError::InternalServerError)?;

    Ok(ApplicationResponse::json(ReconStatusResponse {
        processor_status,
        pending_staging_entries,
    }))
}
