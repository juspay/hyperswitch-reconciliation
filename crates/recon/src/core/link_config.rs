use api_models::link_config::{
    GetLinkConfigResponse, LinkConfigCreateRequest, LinkConfigUpdateRequest,
};
use common_types::link_config::link_matching_key::{LinkSearchIdentifier, LinkSearchIdentifierV1};
use common_utils::auth::AuthContext;
use diesel_models::link_config::{LinkConfigNew, LinkConfigUpdate};
use error_stack::ResultExt;

use crate::{
    db::{
        account::AccountInterface, link_config::LinkConfigInterface, recon_rule::ReconRuleInterface,
    },
    errors::{
        conversions::StorageErrorExt,
        link_config::{LinkConfigError, LinkConfigResponse, LinkConfigResult},
    },
    types::ApplicationResponse,
    utils::link_config::{LinkConfigExt, LinkSearchIdentifierApiExt, LinkTriggerApiExt},
    AppState,
};

pub async fn get_link_config_by_id(
    state: &AppState,
    link_config_id: &str,
    auth_context: AuthContext,
) -> LinkConfigResponse<GetLinkConfigResponse> {
    let config = state
        .store
        .find_link_config_by_profile_id_and_link_config_id(&auth_context.profile_id, link_config_id)
        .await
        .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
            link_config_id: link_config_id.to_owned(),
        })?;

    Ok(ApplicationResponse::json(config.to_api_response()))
}

pub async fn list_link_configs_by_profile_id(
    state: &AppState,
    profile_id: &str,
) -> LinkConfigResponse<Vec<GetLinkConfigResponse>> {
    let configs = state
        .store
        .list_link_configs_by_profile_id(profile_id)
        .await
        .change_context(LinkConfigError::InternalServerError)?;

    Ok(ApplicationResponse::json(
        configs
            .into_iter()
            .map(|config| config.to_api_response())
            .collect(),
    ))
}

pub async fn create_link_config(
    state: &AppState,
    request: LinkConfigCreateRequest,
    auth_context: AuthContext,
) -> LinkConfigResponse<GetLinkConfigResponse> {
    let link_search_identifier = request.link_search_identifier.to_diesel_model();
    validate_link_config(
        state,
        &auth_context.profile_id,
        &request.account_id,
        &request.transaction_rule_id,
        &request.confirmation_rule_id,
        &link_search_identifier,
    )
    .await?;

    let new_config = LinkConfigNew::new(
        auth_context.profile_id,
        request.account_id,
        request.transaction_rule_id,
        request.confirmation_rule_id,
        request.transaction_trigger.to_diesel_model(),
        request.confirmation_trigger.to_diesel_model(),
        link_search_identifier,
        request.priority,
    );
    let config = state
        .store
        .insert_link_config(new_config)
        .await
        .change_context(LinkConfigError::InternalServerError)?;

    Ok(ApplicationResponse::json_created(config.to_api_response()))
}

pub async fn update_link_config(
    state: &AppState,
    link_config_id: &str,
    request: LinkConfigUpdateRequest,
    auth_context: AuthContext,
) -> LinkConfigResponse<GetLinkConfigResponse> {
    let existing = state
        .store
        .find_link_config_by_profile_id_and_link_config_id(&auth_context.profile_id, link_config_id)
        .await
        .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
            link_config_id: link_config_id.to_owned(),
        })?;

    if request.account_id.is_none()
        && request.transaction_rule_id.is_none()
        && request.confirmation_rule_id.is_none()
        && request.transaction_trigger.is_none()
        && request.confirmation_trigger.is_none()
        && request.link_search_identifier.is_none()
        && request.priority.is_none()
        && request.is_active.is_none()
    {
        return Ok(ApplicationResponse::json(existing.to_api_response()));
    }

    let link_search_identifier = request
        .link_search_identifier
        .map(|value| value.to_diesel_model());
    validate_link_config(
        state,
        &auth_context.profile_id,
        request
            .account_id
            .as_deref()
            .unwrap_or(&existing.account_id),
        request
            .transaction_rule_id
            .as_deref()
            .unwrap_or(&existing.transaction_rule_id),
        request
            .confirmation_rule_id
            .as_deref()
            .unwrap_or(&existing.confirmation_rule_id),
        link_search_identifier
            .as_ref()
            .unwrap_or(&existing.link_search_identifier),
    )
    .await?;

    let update = LinkConfigUpdate::ConfigUpdate {
        account_id: request.account_id,
        transaction_rule_id: request.transaction_rule_id,
        confirmation_rule_id: request.confirmation_rule_id,
        transaction_trigger: request
            .transaction_trigger
            .map(|value| value.to_diesel_model()),
        confirmation_trigger: request
            .confirmation_trigger
            .map(|value| value.to_diesel_model()),
        link_search_identifier,
        priority: request.priority.map(i16::from),
        is_active: request.is_active,
    };
    let config = state
        .store
        .update_link_config_by_profile_id_and_link_config_id(
            &auth_context.profile_id,
            link_config_id,
            update,
        )
        .await
        .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
            link_config_id: link_config_id.to_owned(),
        })?;

    Ok(ApplicationResponse::json(config.to_api_response()))
}

async fn validate_link_config(
    state: &AppState,
    profile_id: &str,
    account_id: &str,
    transaction_rule_id: &str,
    confirmation_rule_id: &str,
    link_search_identifier: &LinkSearchIdentifier,
) -> LinkConfigResult<()> {
    for (field, value) in [
        ("account_id", account_id),
        ("transaction_rule_id", transaction_rule_id),
        ("confirmation_rule_id", confirmation_rule_id),
    ] {
        if value.trim().is_empty() {
            return Err(LinkConfigError::InvalidLinkConfig {
                message: format!("{field} cannot be empty"),
            }
            .into());
        }
    }
    if let LinkSearchIdentifier::V1(LinkSearchIdentifierV1::Composite(composite)) =
        link_search_identifier
    {
        if composite.keys.is_empty() {
            return Err(LinkConfigError::InvalidLinkConfig {
                message: "Composite matching identifier must contain at least one key".to_owned(),
            }
            .into());
        }
    }

    state
        .store
        .find_account_by_profile_id_and_account_id(profile_id, account_id)
        .await
        .to_not_found_response(|| LinkConfigError::AccountNotFound {
            account_id: account_id.to_owned(),
        })?;
    for rule_id in [transaction_rule_id, confirmation_rule_id] {
        state
            .store
            .find_recon_rule_by_profile_id_and_rule_id(profile_id, rule_id)
            .await
            .to_not_found_response(|| LinkConfigError::RuleNotFound {
                rule_id: rule_id.to_owned(),
            })?;
    }
    Ok(())
}
