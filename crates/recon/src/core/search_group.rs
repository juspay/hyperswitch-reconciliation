use api_models::search_group::SearchGroupResponse;
use common_utils::auth::AuthContext;
use error_stack::ResultExt;
use recon_domain_models::{behaviour::ReverseConversion, search_group::DomainSearchGroup};

use crate::{
    db::search_group::SearchGroupInterface,
    errors::{
        conversions::StorageErrorExt,
        search_group::{SearchGroupError, SearchGroupResult},
    },
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn get_search_group_by_id(
    state: AppState,
    id: &str,
    auth_context: AuthContext,
) -> SearchGroupResult<SearchGroupResponse> {
    let search_group = state
        .store
        .find_by_id_and_profile_id(id, &auth_context.profile_id)
        .await
        .change_context(SearchGroupError::SearchGroupNotFound)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(search_group.merchant_id.clone());

    let domain_search_group: DomainSearchGroup = search_group
        .convert_back(&merchant_encryptor)
        .await
        .change_context(SearchGroupError::InternalServerError)?;

    Ok(ApplicationResponse::json(SearchGroupResponse {
        id: domain_search_group.id,
        merchant_id: domain_search_group.merchant_id,
        rule_id: domain_search_group.rule_id,
        primary_transaction_id: domain_search_group.primary_transaction_id,
        encrypted_grouping_value: domain_search_group.encrypted_grouping_value,
        hashed_grouping_value: domain_search_group.hashed_grouping_value,
        profile_id: domain_search_group.profile_id,
        status: domain_search_group.status,
        created_at: domain_search_group.created_at,
        last_modified_at: domain_search_group.last_modified_at,
    }))
}

pub async fn get_search_group_by_primary_transaction_id(
    state: AppState,
    primary_transaction_id: &str,
    auth_context: AuthContext,
) -> SearchGroupResult<SearchGroupResponse> {
    let search_group = state
        .store
        .find_by_primary_transaction_id_and_profile_id(
            primary_transaction_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(|| SearchGroupError::SearchGroupNotFound)?
        .ok_or(SearchGroupError::SearchGroupNotFound)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(search_group.merchant_id.clone());

    let domain_search_group: DomainSearchGroup = search_group
        .convert_back(&merchant_encryptor)
        .await
        .change_context(SearchGroupError::InternalServerError)?;

    Ok(ApplicationResponse::json(SearchGroupResponse {
        id: domain_search_group.id,
        merchant_id: domain_search_group.merchant_id,
        rule_id: domain_search_group.rule_id,
        primary_transaction_id: domain_search_group.primary_transaction_id,
        encrypted_grouping_value: domain_search_group.encrypted_grouping_value,
        hashed_grouping_value: domain_search_group.hashed_grouping_value,
        profile_id: domain_search_group.profile_id,
        status: domain_search_group.status,
        created_at: domain_search_group.created_at,
        last_modified_at: domain_search_group.last_modified_at,
    }))
}
