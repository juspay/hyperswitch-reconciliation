use api_models::recon_rule::{
    GetReconRuleResponse, LinkWithRuleUpdate, ReconRuleCreateRequest, ReconRuleUpdateRequest,
};
use common_utils::auth::AuthContext;
use diesel_models::recon_rule::{ReconRuleNew, ReconRuleUpdate};
use error_stack::ResultExt;
use recon_domain_models::{behaviour::Conversion, recon_rule_account::DomainReconRuleAccount};

use crate::{
    db::recon_rule::ReconRuleInterface,
    domain::recon_engine::flows::ReconStrategyExt,
    errors::{
        conversions::StorageErrorExt,
        recon_rule::{ReconRuleError, ReconRuleResponse},
    },
    types::ApplicationResponse,
    utils::recon_rule::{
        update_strategy_with_link, AgingConfigApiRequestExt, ReconRuleExt, ReconStrategyApiExt,
        RuleAccountExt, RuleValidator,
    },
    AppState,
};

pub async fn get_recon_rule_v2_by_id(
    state: &AppState,
    rule_id: &str,
    auth_context: AuthContext,
) -> ReconRuleResponse<GetReconRuleResponse> {
    // Find the recon rule by profile ID and rule ID (ensures profile isolation)
    let recon_rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(&auth_context.profile_id, rule_id)
        .await
        .to_not_found_response(|| ReconRuleError::RuleNotFound {
            rule_id: rule_id.to_string(),
        })?;

    Ok(ApplicationResponse::json(recon_rule.to_api_response_v2()))
}

pub async fn update_recon_rule(
    state: &AppState,
    rule_id: &str,
    request: ReconRuleUpdateRequest,
    auth_context: AuthContext,
) -> ReconRuleResponse<GetReconRuleResponse> {
    // Validate rule_name if provided
    if let Some(ref name) = request.rule_name {
        if name.trim().is_empty() {
            return Err(ReconRuleError::InvalidRuleName.into());
        }
    }

    // Validate rule_description if provided
    if let Some(ref description) = request.rule_description {
        if description.trim().is_empty() {
            return Err(ReconRuleError::InvalidRuleDescription.into());
        }
    }

    // Validate strategy if provided
    let diesel_strategy = request.strategy.map(|strategy| strategy.to_diesel_model());
    if let Some(ref strategy) = diesel_strategy {
        let recon_engine_type = strategy.get_recon_engine_type();
        recon_engine_type
            .validate(state, &auth_context.profile_id)
            .await?;
    }

    let existing_rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(&auth_context.profile_id, rule_id)
        .await
        .to_not_found_response(|| ReconRuleError::RuleNotFound {
            rule_id: rule_id.to_string(),
        })?;

    let updated_rule = if request.rule_name.is_some() || request.rule_description.is_some() {
        let recon_rule_update = ReconRuleUpdate::DetailsUpdate {
            rule_name: request.rule_name,
            rule_description: request.rule_description,
        };
        state
            .store
            .update_recon_rule_by_id(rule_id, recon_rule_update)
            .await
            .change_context(ReconRuleError::InternalServerError)?
    } else {
        existing_rule
    };

    let updated_rule = if let Some(strategy) = diesel_strategy {
        let recon_rule_update = ReconRuleUpdate::StrategyUpdate {
            strategy: Box::new(strategy),
        };

        state
            .store
            .update_recon_rule_by_id(rule_id, recon_rule_update)
            .await
            .change_context(ReconRuleError::InternalServerError)?
    } else {
        updated_rule
    };

    let updated_rule = if let Some(link_with_rule) = request.link_with_rule {
        let update_data = match link_with_rule {
            LinkWithRuleUpdate::Unset => None,
            LinkWithRuleUpdate::Set { data } => Some(data),
        };
        let updated_strategy = update_strategy_with_link(updated_rule.strategy, update_data)?;

        let recon_rule_update = ReconRuleUpdate::StrategyUpdate {
            strategy: Box::new(updated_strategy),
        };

        state
            .store
            .update_recon_rule_by_id(rule_id, recon_rule_update)
            .await
            .change_context(ReconRuleError::InternalServerError)?
    } else {
        updated_rule
    };

    let updated_rule = if let Some(aging_config) = request.aging_config {
        let recon_rule_update = ReconRuleUpdate::AgingConfigUpdate {
            aging_config: aging_config.to_diesel_model(),
        };

        state
            .store
            .update_recon_rule_by_id(rule_id, recon_rule_update)
            .await
            .change_context(ReconRuleError::InternalServerError)?
    } else {
        updated_rule
    };

    let updated_rule = if let Some(priority) = request.priority {
        let recon_rule_update = ReconRuleUpdate::PriorityUpdate {
            priority: i16::from(priority),
        };
        state
            .store
            .update_recon_rule_by_id(rule_id, recon_rule_update)
            .await
            .change_context(ReconRuleError::InternalServerError)?
    } else {
        updated_rule
    };

    let updated_rule = if let Some(is_active) = request.is_active {
        let recon_rule_update = ReconRuleUpdate::ActiveStatusUpdate { is_active };
        state
            .store
            .update_recon_rule_by_id(rule_id, recon_rule_update)
            .await
            .change_context(ReconRuleError::InternalServerError)?
    } else {
        updated_rule
    };

    Ok(ApplicationResponse::json(updated_rule.to_api_response_v2()))
}

pub async fn create_recon_rule_v2(
    state: &AppState,
    request: ReconRuleCreateRequest,
    auth_context: AuthContext,
) -> ReconRuleResponse<GetReconRuleResponse> {
    // Validate request
    if request.rule_name.is_empty() {
        return Err(ReconRuleError::InvalidRuleName.into());
    }

    if request.rule_description.is_empty() {
        return Err(ReconRuleError::InvalidRuleDescription.into());
    }

    // Convert API strategy to domain strategy
    let strategy = request.strategy.to_diesel_model();

    let recon_engine_type = strategy.get_recon_engine_type();
    recon_engine_type
        .validate(state, &auth_context.profile_id)
        .await?;

    // Create main recon rule using constructor
    let recon_rule_new = ReconRuleNew::new(
        request.rule_name,
        request.rule_description,
        request.priority,
        auth_context.profile_id.clone(),
        strategy,
        request.aging_config.to_diesel_model(),
    );

    let rule_accounts_new = recon_engine_type
        .get_rule_accounts()
        .into_iter()
        .map(|(account_id, account_type)| {
            DomainReconRuleAccount::new(account_id, recon_rule_new.id.clone(), account_type)
                .construct_new()
        })
        .collect();

    // Insert only the recon_rule table entry (v2 only - no source/target/account tables)
    let (created_rule, _rule_accounts) = state
        .store
        .create_recon_rule_with_sources_and_targets(recon_rule_new, rule_accounts_new)
        .await
        .change_context(ReconRuleError::InternalServerError)?;

    Ok(ApplicationResponse::json_created(
        created_rule.to_api_response_v2(),
    ))
}

pub async fn list_recon_rules_by_profile_id_v2(
    state: &AppState,
    profile_id: &str,
) -> ReconRuleResponse<Vec<GetReconRuleResponse>> {
    // Get all recon rules for the profile
    let recon_rules = state
        .store
        .list_recon_rules_by_profile_id(profile_id, None)
        .await
        .change_context(ReconRuleError::InternalServerError)?;

    Ok(ApplicationResponse::json(
        recon_rules
            .into_iter()
            .map(|recon_rule| recon_rule.to_api_response_v2())
            .collect(),
    ))
}
