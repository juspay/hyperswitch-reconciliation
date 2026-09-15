use api_models::business_profile as api;
use api_models::business_profile::ReconStatusResponse;
use common_utils::{
    auth::AuthContext, id_generation::generate_id_with_default_len, id_type::MerchantId,
};
use diesel_models::business_profile::BusinessProfileNew;
use error_stack::ResultExt;
use futures::future;
use masking::{ExposeInterface, PeekInterface};
use recon_domain_models::{behaviour::ReverseConversion, business_profile::DomainBusinessProfile};

use crate::{
    db::business_profile::BusinessProfileInterface,
    errors::business_profile::{BusinessProfileError, BusinessProfileResult},
    routes::AppState,
    types::ApplicationResponse,
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

    let domain_entries = future::try_join_all(business_profiles_futures)
        .await
        .change_context(BusinessProfileError::InternalServerError)?;

    let response =
        domain_entries
            .into_iter()
            .map(
                |domain_business_profile: DomainBusinessProfile| -> Result<
                    api::BusinessProfileResponse,
                    error_stack::Report<BusinessProfileError>,
                > {
                    Ok(api::BusinessProfileResponse {
                        profile_id: domain_business_profile.profile_id,
                        merchant_id: domain_business_profile.merchant_id,
                        profile_name: domain_business_profile.profile_name,
                    })
                },
            )
            .collect::<Result<Vec<api::BusinessProfileResponse>, _>>()?;

    Ok(ApplicationResponse::json(response))
}

pub async fn create_business_profile(
    state: AppState,
    merchant_id: MerchantId,
    request: api::BusinessProfileCreateRequest,
) -> BusinessProfileResult<api::BusinessProfileResponse> {
    if request.profile_name.peek().is_empty() {
        return Err(BusinessProfileError::ProfileNameEmpty.into());
    }

    let profile_id = generate_id_with_default_len("profile");

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id.clone());

    let diesel_business_profile_new = BusinessProfileNew {
        profile_id: profile_id.clone(),
        profile_name: merchant_encryptor
            .encrypt_string(request.profile_name.expose())
            .await
            .change_context(BusinessProfileError::InternalServerError)?
            .into(),
        merchant_id: merchant_id.clone(),
        created_at: common_utils::date_time::now(),
        last_modified_at: common_utils::date_time::now(),
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
        api::BusinessProfileResponse {
            profile_id: domain_business_profile.profile_id,
            merchant_id: domain_business_profile.merchant_id,
            profile_name: domain_business_profile.profile_name,
        },
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

    Ok(ApplicationResponse::json(api::BusinessProfileResponse {
        profile_id: domain_business_profile.profile_id,
        merchant_id: domain_business_profile.merchant_id,
        profile_name: domain_business_profile.profile_name,
    }))
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
