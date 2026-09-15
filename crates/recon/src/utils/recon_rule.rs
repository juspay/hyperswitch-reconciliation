use std::collections::HashSet;

use crate::domain::recon_engine::flows::ReconEngineType;
use crate::errors::{conversions::StorageErrorExt, ReconRuleError};
use crate::routes::AppState;
use crate::{db::account::AccountInterface, errors::recon_rule::ReconRuleResult};
use api_models::recon_rule::{
    aging_config::AgingConfigApiResponse, AgingConfigApiRequest, GetReconRuleResponse,
    ReconStrategy as ReconStrategyApi, ThresholdApiRequest, ThresholdApiResponse,
};
use common_enums::{Currency, ReconRuleAccountType};
use common_types::recon_rule::entry_field::EntryField;
use common_types::recon_rule::search_identifier::SearchIdentifierKey;
use common_types::recon_rule::{
    aging_config::{
        AgingConfig, AgingConfigV1, AgingConfigWithThreshold, AgingConfigWithThresholdV1,
    },
    grouping_field::{CompositeGroupingField, GroupingField, GroupingFieldV1, SingleGroupingField},
    link_with_rule::LinkWithRule,
    match_rules::{MatchRules, MatchRulesV1},
    search_identifier::{
        CompositeSearchIdentifier, SearchIdentifier, SearchIdentifierV1, SearchIdentifierV2,
        SingleSearchIdentifier,
    },
    strategies::{
        one_to_many_strategy::{OneToManySingleSingle, OneToManyStrategy, OneToManyStrategyV1},
        one_to_one_strategy::{
            OneToOneManyMany, OneToOneManySingle, OneToOneSingleMany, OneToOneSingleSingle,
            OneToOneStrategy, OneToOneStrategyV1,
        },
    },
    tolerance_config::{ToleranceConfig, ToleranceConfigV1},
    trigger::Trigger,
    ReconStrategy, ReconStrategyV1,
};
use diesel_models::recon_rule::ReconRule;
use futures::future;
pub mod one_to_many;
pub mod one_to_one;

// Trait for converting ReconRuleSource to ReconRuleSourceResponse

pub trait ReconRuleExt {
    fn to_api_response_v2(self) -> GetReconRuleResponse;
}

impl ReconRuleExt for ReconRule {
    fn to_api_response_v2(self) -> GetReconRuleResponse {
        GetReconRuleResponse {
            rule_id: self.id,
            rule_name: self.name,
            rule_description: self.description,
            priority: self.priority,
            is_active: self.is_active,
            profile_id: self.profile_id,
            strategy: self.strategy.to_api_response(),
            aging_config: self.aging_config.to_api_response(),
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}

pub trait AgingConfigExt {
    type ReturnType;
    fn to_api_response(self) -> Self::ReturnType;
}

impl AgingConfigExt for AgingConfig {
    type ReturnType = AgingConfigApiResponse;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(config_v1) => config_v1.to_api_response(),
        }
    }
}

impl AgingConfigExt for AgingConfigV1 {
    type ReturnType = AgingConfigApiResponse;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::NoAging => AgingConfigApiResponse::NoAging,
            Self::WithThreshold(with_threshold) => with_threshold.to_api_response(),
        }
    }
}

impl AgingConfigExt for AgingConfigWithThreshold {
    type ReturnType = AgingConfigApiResponse;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(with_threshold) => with_threshold.to_api_response(),
        }
    }
}

impl AgingConfigExt for AgingConfigWithThresholdV1 {
    type ReturnType = AgingConfigApiResponse;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::WeekDaysV1 { value } => AgingConfigApiResponse::WithThreshold {
                threshold: ThresholdApiResponse::WeekDays { value },
            },
        }
    }
}

pub trait AgingConfigApiRequestExt {
    type ReturnType;
    fn to_diesel_model(self) -> Self::ReturnType;
}

impl AgingConfigApiRequestExt for AgingConfigApiRequest {
    type ReturnType = AgingConfig;
    fn to_diesel_model(self) -> Self::ReturnType {
        match self {
            Self::NoAging => AgingConfig::V1(AgingConfigV1::NoAging),
            Self::WithThreshold { threshold } => AgingConfig::V1(AgingConfigV1::WithThreshold(
                AgingConfigWithThreshold::V1(threshold.to_diesel_model()),
            )),
        }
    }
}

impl AgingConfigApiRequestExt for ThresholdApiRequest {
    type ReturnType = AgingConfigWithThresholdV1;
    fn to_diesel_model(self) -> Self::ReturnType {
        match self {
            Self::WeekDays { value } => AgingConfigWithThresholdV1::WeekDaysV1 { value },
        }
    }
}

pub trait ReconStrategyExt {
    type ReturnType;
    fn to_api_response(self) -> Self::ReturnType;
}

impl ReconStrategyExt for ReconStrategy {
    type ReturnType = ReconStrategyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(strategy_v1) => strategy_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for ReconStrategyV1 {
    type ReturnType = ReconStrategyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::OneToOne(one_to_one) => Self::ReturnType::OneToOne(one_to_one.to_api_response()),
            Self::OneToMany(one_to_many) => {
                Self::ReturnType::OneToMany(one_to_many.to_api_response())
            }
        }
    }
}

pub trait ReconStrategyApiExt {
    type ReturnType;
    fn to_diesel_model(self) -> Self::ReturnType;
}

impl ReconStrategyApiExt for ReconStrategyApi {
    type ReturnType = ReconStrategy;
    fn to_diesel_model(self) -> Self::ReturnType {
        match self {
            Self::OneToOne(one_to_one_strategy) => ReconStrategy::V1(ReconStrategyV1::OneToOne(
                one_to_one_strategy.to_diesel_model(),
            )),
            Self::OneToMany(one_to_many_strategy) => ReconStrategy::V1(ReconStrategyV1::OneToMany(
                one_to_many_strategy.to_diesel_model(),
            )),
        }
    }
}

pub trait RuleAccountExt {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)>;
}

impl RuleAccountExt for ReconEngineType {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)> {
        match self {
            Self::OneToOneSingleSingleV1(engine) => engine.get_rule_accounts(),
            Self::OneToOneSingleManyV1(engine) => engine.get_rule_accounts(),
            Self::OneToOneManySingleV1(engine) => engine.get_rule_accounts(),
            Self::OneToOneManyManyV1(engine) => engine.get_rule_accounts(),
            Self::OneToManySingleSingleV1(engine) => engine.get_rule_accounts(),
        }
    }
}

pub trait RuleLinkExt {
    fn get_link_with_rule(&self) -> Option<LinkWithRule>;
}

impl RuleLinkExt for ReconEngineType {
    fn get_link_with_rule(&self) -> Option<LinkWithRule> {
        match self {
            Self::OneToOneSingleSingleV1(engine) => engine.link_with_rule.clone(),
            Self::OneToOneSingleManyV1(engine) => engine.link_with_rule.clone(),
            Self::OneToOneManySingleV1(engine) => engine.link_with_rule.clone(),
            Self::OneToOneManyManyV1(engine) => engine.link_with_rule.clone(),
            Self::OneToManySingleSingleV1(engine) => engine.link_with_rule.clone(),
        }
    }
}

pub trait TriggerRuleValidator {
    fn validate_for_rule(&self) -> ReconRuleResult<()>;
}

impl TriggerRuleValidator for Trigger {
    fn validate_for_rule(&self) -> ReconRuleResult<()> {
        match self {
            Self::V1(_) => Ok(()),
            Self::V2(trigger) if trigger.conditions.is_empty() => {
                Err(ReconRuleError::InvalidReconRule {
                    message: "Composite trigger must contain at least one condition".to_string(),
                }
                .into())
            }
            Self::V2(_) => Ok(()),
        }
    }
}

#[async_trait::async_trait]
pub trait RuleValidator {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()>;

    async fn default_validations(
        state: &AppState,
        engine: impl RuleAccountExt + Send,
        profile_id: &str,
    ) -> ReconRuleResult<()> {
        let rule_accounts = engine.get_rule_accounts();
        // 1. Verify all accounts exist
        let accounts = future::try_join_all(rule_accounts.iter().map(|(account_id, _)| {
            let store = &state.store;
            let account_id = account_id.clone();
            async move {
                store
                    .find_account_by_profile_id_and_account_id(profile_id, &account_id)
                    .await
                    .to_not_found_response(|| ReconRuleError::AccountNotFound { account_id })
            }
        }))
        .await?;
        // 2. Check all accounts share currency
        if let Some(expected) = accounts.first().map(|a| &a.currency) {
            if accounts.iter().any(|a| a.currency != *expected) {
                return Err(ReconRuleError::InvalidReconRule {
                    message: "All accounts must have the same currency".into(),
                }
                .into());
            }
        }

        // 3. Check unique account IDs
        let rule_accounts_set = rule_accounts
            .iter()
            .map(|(account_id, _)| account_id)
            .collect::<HashSet<&String>>();

        if rule_accounts_set.len() != rule_accounts.len() {
            return Err(ReconRuleError::InvalidReconRule {
                message: "Account IDs in the Recon Rule must be unique".into(),
            }
            .into());
        }

        // 4. Must have at least one source and one target
        let has_source = rule_accounts
            .iter()
            .any(|(_, acc_type)| matches!(acc_type, ReconRuleAccountType::Source));

        let has_target = rule_accounts
            .iter()
            .any(|(_, acc_type)| matches!(acc_type, ReconRuleAccountType::Target));

        if !has_source || !has_target {
            return Err(ReconRuleError::InvalidReconRule {
                message: "Recon Rule must have at least one Source and one Target account".into(),
            }
            .into());
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl RuleValidator for ReconEngineType {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()> {
        match self {
            Self::OneToOneSingleSingleV1(engine) => engine.validate(state, profile_id).await,
            Self::OneToOneSingleManyV1(engine) => engine.validate(state, profile_id).await,
            Self::OneToOneManySingleV1(engine) => engine.validate(state, profile_id).await,
            Self::OneToOneManyManyV1(engine) => engine.validate(state, profile_id).await,
            Self::OneToManySingleSingleV1(engine) => engine.validate(state, profile_id).await,
        }
    }
}

pub fn update_strategy_with_link(
    strategy: ReconStrategy,
    link_with_rule: Option<LinkWithRule>,
) -> Result<ReconStrategy, error_stack::Report<ReconRuleError>> {
    match strategy {
        ReconStrategy::V1(ReconStrategyV1::OneToOne(OneToOneStrategy::V1(
            one_to_one_strategy_v1,
        ))) => {
            let updated_one_to_one = match one_to_one_strategy_v1 {
                OneToOneStrategyV1::SingleSingle(OneToOneSingleSingle::V1(mut v1)) => {
                    v1.link_with_rule = link_with_rule;
                    OneToOneStrategyV1::SingleSingle(OneToOneSingleSingle::V1(v1))
                }
                OneToOneStrategyV1::SingleMany(OneToOneSingleMany::V1(mut v1)) => {
                    v1.link_with_rule = link_with_rule;
                    OneToOneStrategyV1::SingleMany(OneToOneSingleMany::V1(v1))
                }
                OneToOneStrategyV1::ManySingle(OneToOneManySingle::V1(mut v1)) => {
                    v1.link_with_rule = link_with_rule;
                    OneToOneStrategyV1::ManySingle(OneToOneManySingle::V1(v1))
                }
                OneToOneStrategyV1::ManyMany(OneToOneManyMany::V1(mut v1)) => {
                    v1.link_with_rule = link_with_rule;
                    OneToOneStrategyV1::ManyMany(OneToOneManyMany::V1(v1))
                }
            };

            Ok(ReconStrategy::V1(ReconStrategyV1::OneToOne(
                OneToOneStrategy::V1(updated_one_to_one),
            )))
        }
        ReconStrategy::V1(ReconStrategyV1::OneToMany(OneToManyStrategy::V1(one_to_many_v1))) => {
            let updated_one_to_many = match one_to_many_v1 {
                OneToManyStrategyV1::SingleSingle(OneToManySingleSingle::V1(mut v1)) => {
                    v1.link_with_rule = link_with_rule;
                    OneToManyStrategyV1::SingleSingle(OneToManySingleSingle::V1(v1))
                }
            };

            Ok(ReconStrategy::V1(ReconStrategyV1::OneToMany(
                OneToManyStrategy::V1(updated_one_to_many),
            )))
        }
    }
}

// ===============> Tolerance Config Validation <==============

/// Validates tolerance config if present:
/// - lower_tolerance must be >= 0
/// - upper_tolerance must be >= 0
/// - tolerance_account_id must exist and belong to the same profile
/// - tolerance account must have the same currency as the other accounts in the rule
async fn validate_tolerance_config(
    state: &AppState,
    profile_id: &str,
    tolerance_config: &Option<ToleranceConfig>,
    other_account_currency: Currency,
) -> ReconRuleResult<()> {
    let Some(tolerance_config) = tolerance_config else {
        return Ok(());
    };

    let ToleranceConfig::V1(ToleranceConfigV1 {
        lower_tolerance,
        upper_tolerance,
        tolerance_account_id,
    }) = tolerance_config;

    // Validate lower_tolerance is non-negative
    if lower_tolerance.get_amount_as_i64() < 0 {
        return Err(ReconRuleError::InvalidReconRule {
            message: "lower_tolerance must be non-negative".to_string(),
        }
        .into());
    }

    // Validate upper_tolerance is non-negative
    if upper_tolerance.get_amount_as_i64() < 0 {
        return Err(ReconRuleError::InvalidReconRule {
            message: "upper_tolerance must be non-negative".to_string(),
        }
        .into());
    }

    // Validate tolerance_account_id exists and belongs to the same profile
    let account = state
        .store
        .find_account_by_profile_id_and_account_id(profile_id, tolerance_account_id)
        .await
        .to_not_found_response(|| ReconRuleError::InvalidReconRule {
            message: format!(
                "tolerance_account_id '{}' not found or does not belong to the profile",
                tolerance_account_id
            ),
        })?;

    // Validate tolerance account has the same currency as the other accounts in the rule
    if account.currency != other_account_currency {
        return Err(ReconRuleError::InvalidReconRule {
            message: format!(
                "tolerance_account_id '{}' must have the same currency as the other accounts in the rule",
                tolerance_account_id
            ),
        }
        .into());
    }

    Ok(())
}

// ===============> Search Identifier Validation <==============

pub trait SearchIdentifierRuleValidator {
    fn validate_for_rule(&self) -> ReconRuleResult<()>;
}

impl SearchIdentifierRuleValidator for SearchIdentifier {
    fn validate_for_rule(&self) -> ReconRuleResult<()> {
        match self {
            Self::V1(v1) => v1.validate_for_rule(),
            Self::V2(v2) => v2.validate_for_rule(),
        }
    }
}

impl SearchIdentifierRuleValidator for SearchIdentifierV1 {
    fn validate_for_rule(&self) -> ReconRuleResult<()> {
        Ok(())
    }
}

impl SearchIdentifierRuleValidator for SearchIdentifierV2 {
    fn validate_for_rule(&self) -> ReconRuleResult<()> {
        match self {
            Self::Single { .. } => Ok(()),
            Self::Composite(CompositeSearchIdentifier { keys, .. }) => {
                validate_composite_search_keys(keys)
            }
        }
    }
}

fn validate_composite_search_keys(keys: &[SearchIdentifierKey]) -> ReconRuleResult<()> {
    if keys.is_empty() {
        return Err(ReconRuleError::InvalidSearchIdentifier {
            message: "Composite search identifier must contain at least one key".to_string(),
        }
        .into());
    }

    let mut seen_source: HashSet<String> = HashSet::new();
    let mut seen_target: HashSet<String> = HashSet::new();
    for k in keys {
        if !seen_source.insert(k.source_field.to_string()) {
            return Err(ReconRuleError::InvalidSearchIdentifier {
                message: format!(
                    "Search identifier has duplicate source_field {}",
                    k.source_field
                ),
            }
            .into());
        }

        if !seen_target.insert(k.target_field.to_string()) {
            return Err(ReconRuleError::InvalidSearchIdentifier {
                message: format!(
                    "Search identifier has duplicate target_field {}",
                    k.target_field
                ),
            }
            .into());
        }
    }

    Ok(())
}

// ===============> Grouping Field Validation <==============

pub trait GroupingFieldValidator {
    fn validate_for_rule(&self) -> ReconRuleResult<()>;
}

impl GroupingFieldValidator for GroupingField {
    fn validate_for_rule(&self) -> ReconRuleResult<()> {
        match self {
            Self::V1(v1) => v1.validate_for_rule(),
        }
    }
}

impl GroupingFieldValidator for GroupingFieldV1 {
    fn validate_for_rule(&self) -> ReconRuleResult<()> {
        match self {
            Self::Single { .. } => Ok(()),
            Self::Composite(CompositeGroupingField { fields, .. }) => {
                validate_composite_grouping_fields(fields)
            }
        }
    }
}

fn validate_composite_grouping_fields(fields: &[EntryField]) -> ReconRuleResult<()> {
    if fields.is_empty() {
        return Err(ReconRuleError::InvalidGroupingField {
            message: "Composite grouping field must contain at least one field".to_string(),
        }
        .into());
    }

    let mut seen: HashSet<String> = HashSet::new();
    for f in fields {
        if !seen.insert(f.to_string()) {
            return Err(ReconRuleError::InvalidGroupingField {
                message: format!("Grouping field has duplicate field {}", f),
            }
            .into());
        }
    }

    Ok(())
}

fn validate_source_field_sets(
    search_identifier: &SearchIdentifier,
    grouping_field: &GroupingField,
    match_rules: &MatchRules,
) -> ReconRuleResult<()> {
    let grouping_set: HashSet<String> = match grouping_field {
        GroupingField::V1(GroupingFieldV1::Single(SingleGroupingField { field })) => {
            std::iter::once(field.to_string()).collect()
        }
        GroupingField::V1(GroupingFieldV1::Composite(CompositeGroupingField {
            fields, ..
        })) => fields.iter().map(|f| f.to_string()).collect(),
    };

    let search_set: HashSet<String> = match search_identifier {
        SearchIdentifier::V1(SearchIdentifierV1 { source_field, .. }) => {
            std::iter::once(source_field.to_string()).collect()
        }
        SearchIdentifier::V2(SearchIdentifierV2::Single(SingleSearchIdentifier { key })) => {
            std::iter::once(key.source_field.to_string()).collect()
        }
        SearchIdentifier::V2(SearchIdentifierV2::Composite(CompositeSearchIdentifier {
            keys,
            ..
        })) => keys.iter().map(|k| k.source_field.to_string()).collect(),
    };

    // Note: When we have equal values: search_identifier ⊆ (grouping_set U equal_values)
    if !search_set.is_subset(&grouping_set) {
        return Err(ReconRuleError::InvalidSearchIdentifier {
            message:
                "Search identifier source fields must be a subset of source account's grouping fields"
                    .to_string(),
        }
        .into());
    }

    let rules_source_set: HashSet<String> = match match_rules {
        MatchRules::V1(MatchRulesV1 { rules }) => rules
            .iter()
            .map(|rule| rule.source_field.to_string())
            .collect(),
    };

    if !rules_source_set.is_subset(&grouping_set) {
        return Err(ReconRuleError::InvalidMatchRules {
            message: "Match rules source field must be one of source account's grouping fields"
                .to_string(),
        }
        .into());
    }

    Ok(())
}
