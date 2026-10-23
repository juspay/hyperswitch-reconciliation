use api_models::link_config::{
    GetLinkConfigResponse, LinkConfigCreateRequest, LinkConfigUpdateRequest,
};
use common_utils::auth::AuthContext;
use diesel_models::link_config::{LinkConfigNew, LinkConfigUpdate};
use error_stack::ResultExt;

use crate::{
    db::link_config::LinkConfigInterface,
    errors::{
        conversions::StorageErrorExt,
        link_config::{LinkConfigError, LinkConfigResponse},
    },
    types::ApplicationResponse,
    utils::link_config::{
        LinkConfigExt, LinkConfigValidator, LinkSearchIdentifierApiExt, LinkTriggerApiExt,
    },
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
    request
        .validate_with_db(state, &auth_context.profile_id)
        .await?;

    let new_config = LinkConfigNew::new(
        auth_context.profile_id,
        request.account_id,
        request.transaction_rule_id,
        request.confirmation_rule_id,
        request.transaction_trigger.to_diesel_model(),
        request.confirmation_trigger.to_diesel_model(),
        request.link_search_identifier.to_diesel_model(),
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

    request
        .validate_with_db(state, &auth_context.profile_id)
        .await?;

    let config = if request.transaction_rule_id.is_some() || request.confirmation_rule_id.is_some()
    {
        let update = LinkConfigUpdate::RuleIdsUpdate {
            transaction_rule_id: request.transaction_rule_id,
            confirmation_rule_id: request.confirmation_rule_id,
        };
        state
            .store
            .update_link_config_by_profile_id_and_link_config_id(
                &auth_context.profile_id,
                link_config_id,
                update,
            )
            .await
            .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
                link_config_id: link_config_id.to_owned(),
            })?
    } else {
        existing
    };

    let config = if request.transaction_trigger.is_some() || request.confirmation_trigger.is_some()
    {
        let update = LinkConfigUpdate::TriggerUpdate {
            transaction_trigger: request
                .transaction_trigger
                .map(|value| value.to_diesel_model()),
            confirmation_trigger: request
                .confirmation_trigger
                .map(|value| value.to_diesel_model()),
        };
        state
            .store
            .update_link_config_by_profile_id_and_link_config_id(
                &auth_context.profile_id,
                link_config_id,
                update,
            )
            .await
            .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
                link_config_id: link_config_id.to_owned(),
            })?
    } else {
        config
    };

    let config = if let Some(link_search_identifier) = request.link_search_identifier {
        let update = LinkConfigUpdate::SearchIdentifierUpdate {
            link_search_identifier: link_search_identifier.to_diesel_model(),
        };
        state
            .store
            .update_link_config_by_profile_id_and_link_config_id(
                &auth_context.profile_id,
                link_config_id,
                update,
            )
            .await
            .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
                link_config_id: link_config_id.to_owned(),
            })?
    } else {
        config
    };

    let config = if let Some(priority) = request.priority {
        let update = LinkConfigUpdate::PriorityUpdate {
            priority: i16::from(priority),
        };
        state
            .store
            .update_link_config_by_profile_id_and_link_config_id(
                &auth_context.profile_id,
                link_config_id,
                update,
            )
            .await
            .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
                link_config_id: link_config_id.to_owned(),
            })?
    } else {
        config
    };

    let config = if let Some(is_active) = request.is_active {
        let update = LinkConfigUpdate::ActiveUpdate { is_active };
        state
            .store
            .update_link_config_by_profile_id_and_link_config_id(
                &auth_context.profile_id,
                link_config_id,
                update,
            )
            .await
            .to_not_found_response(|| LinkConfigError::LinkConfigNotFound {
                link_config_id: link_config_id.to_owned(),
            })?
    } else {
        config
    };

    Ok(ApplicationResponse::json(config.to_api_response()))
}
