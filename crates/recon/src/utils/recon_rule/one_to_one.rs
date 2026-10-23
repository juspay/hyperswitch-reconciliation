use crate::{
    db::account::AccountInterface,
    errors::{ReconRuleError, ReconRuleResult},
    utils::recon_rule::{
        validate_source_field_sets, AppState, GroupingFieldValidator, ReconStrategyApiExt,
        ReconStrategyExt, RuleAccountExt, RuleValidator, SearchIdentifierRuleValidator,
        TriggerRuleValidator,
    },
};

use api_models::recon_rule::{
    OneToOneManyMany as OneToOneManyManyApi, OneToOneManyManySource as OneToOneManyManySourceApi,
    OneToOneManyManyTarget as OneToOneManyManyTargetApi,
    OneToOneManySingle as OneToOneManySingleApi,
    OneToOneManySingleSource as OneToOneManySingleSourceApi,
    OneToOneManySingleTarget as OneToOneManySingleTargetApi,
    OneToOneSingleMany as OneToOneSingleManyApi,
    OneToOneSingleManySource as OneToOneSingleManySourceApi,
    OneToOneSingleManyTarget as OneToOneSingleManyTargetApi,
    OneToOneSingleSingle as OneToOneSingleSingleApi,
    OneToOneSingleSingleSource as OneToOneSingleSingleSourceApi,
    OneToOneSingleSingleTarget as OneToOneSingleSingleTargetApi,
    OneToOneStrategy as OneToOneStrategyApi,
};
use common_enums::ReconRuleAccountType;
use common_types::recon_rule::strategies::one_to_one_strategy::{
    OneToOneManyMany, OneToOneManyManySource, OneToOneManyManyTarget, OneToOneManyManyV1,
    OneToOneManySingle, OneToOneManySingleSource, OneToOneManySingleTarget, OneToOneManySingleV1,
    OneToOneSingleMany, OneToOneSingleManySource, OneToOneSingleManyTarget, OneToOneSingleManyV1,
    OneToOneSingleSingle, OneToOneSingleSingleSource, OneToOneSingleSingleTarget,
    OneToOneSingleSingleV1, OneToOneStrategy, OneToOneStrategyV1,
};
use error_stack::ResultExt;

// ===========> ReconStrategyExt impls <=============
impl ReconStrategyExt for OneToOneStrategy {
    type ReturnType = OneToOneStrategyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(one_to_one_v1) => one_to_one_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneStrategyV1 {
    type ReturnType = OneToOneStrategyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::SingleSingle(single_single) => {
                Self::ReturnType::SingleSingle(single_single.to_api_response())
            }
            Self::SingleMany(single_many) => {
                Self::ReturnType::SingleMany(single_many.to_api_response())
            }
            Self::ManySingle(many_single) => {
                Self::ReturnType::ManySingle(many_single.to_api_response())
            }
            Self::ManyMany(many_many) => Self::ReturnType::ManyMany(many_many.to_api_response()),
        }
    }
}

impl ReconStrategyExt for OneToOneSingleSingle {
    type ReturnType = OneToOneSingleSingleApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(single_single_v1) => single_single_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneSingleSingleV1 {
    type ReturnType = OneToOneSingleSingleApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_api_response(),
            target_account: self.target_account.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneSingleSingleSource {
    type ReturnType = OneToOneSingleSingleSourceApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            trigger: self.trigger,
        }
    }
}

impl ReconStrategyExt for OneToOneSingleSingleTarget {
    type ReturnType = OneToOneSingleSingleTargetApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            tolerance_config: self.tolerance_config,
        }
    }
}

impl ReconStrategyExt for OneToOneSingleMany {
    type ReturnType = OneToOneSingleManyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(single_many_v1) => single_many_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneSingleManyV1 {
    type ReturnType = OneToOneSingleManyApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_api_response(),
            target_account: self.target_account.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneSingleManySource {
    type ReturnType = OneToOneSingleManySourceApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            trigger: self.trigger,
        }
    }
}

impl ReconStrategyExt for OneToOneSingleManyTarget {
    type ReturnType = OneToOneSingleManyTargetApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
        }
    }
}

impl ReconStrategyExt for OneToOneManySingle {
    type ReturnType = OneToOneManySingleApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(many_single_v1) => many_single_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneManySingleV1 {
    type ReturnType = OneToOneManySingleApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_api_response(),
            target_account: self.target_account.to_api_response(),
            source_accumulation_mode: self.source_accumulation_mode,
        }
    }
}

impl ReconStrategyExt for OneToOneManySingleSource {
    type ReturnType = OneToOneManySingleSourceApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            trigger: self.trigger,
            grouping_field: self.grouping_field,
        }
    }
}

impl ReconStrategyExt for OneToOneManySingleTarget {
    type ReturnType = OneToOneManySingleTargetApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
        }
    }
}

impl ReconStrategyExt for OneToOneManyMany {
    type ReturnType = OneToOneManyManyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(many_many_v1) => many_many_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneManyManyV1 {
    type ReturnType = OneToOneManyManyApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_api_response(),
            target_account: self.target_account.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToOneManyManySource {
    type ReturnType = OneToOneManyManySourceApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            trigger: self.trigger,
            grouping_field: self.grouping_field,
        }
    }
}

impl ReconStrategyExt for OneToOneManyManyTarget {
    type ReturnType = OneToOneManyManyTargetApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
        }
    }
}

// ==========> ReconStrategyApiExt impls <=============

impl ReconStrategyApiExt for OneToOneStrategyApi {
    type ReturnType = OneToOneStrategy;
    fn to_diesel_model(self) -> Self::ReturnType {
        match self {
            Self::SingleSingle(single_single) => OneToOneStrategy::V1(
                OneToOneStrategyV1::SingleSingle(single_single.to_diesel_model()),
            ),
            Self::SingleMany(singe_many) => {
                OneToOneStrategy::V1(OneToOneStrategyV1::SingleMany(singe_many.to_diesel_model()))
            }
            Self::ManySingle(many_single) => OneToOneStrategy::V1(OneToOneStrategyV1::ManySingle(
                many_single.to_diesel_model(),
            )),
            Self::ManyMany(many_many) => {
                OneToOneStrategy::V1(OneToOneStrategyV1::ManyMany(many_many.to_diesel_model()))
            }
        }
    }
}

impl ReconStrategyApiExt for OneToOneSingleSingleApi {
    type ReturnType = OneToOneSingleSingle;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneSingleSingle::V1(OneToOneSingleSingleV1 {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_diesel_model(),
            target_account: self.target_account.to_diesel_model(),
        })
    }
}

impl ReconStrategyApiExt for OneToOneSingleSingleSourceApi {
    type ReturnType = OneToOneSingleSingleSource;
    fn to_diesel_model(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            trigger: self.trigger,
        }
    }
}

impl ReconStrategyApiExt for OneToOneSingleSingleTargetApi {
    type ReturnType = OneToOneSingleSingleTarget;
    fn to_diesel_model(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            tolerance_config: self.tolerance_config,
        }
    }
}

impl ReconStrategyApiExt for OneToOneSingleManyApi {
    type ReturnType = OneToOneSingleMany;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneSingleMany::V1(OneToOneSingleManyV1 {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_diesel_model(),
            target_account: self.target_account.to_diesel_model(),
        })
    }
}

impl ReconStrategyApiExt for OneToOneSingleManySourceApi {
    type ReturnType = OneToOneSingleManySource;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneSingleManySource {
            account_id: self.account_id,
            trigger: self.trigger,
        }
    }
}

impl ReconStrategyApiExt for OneToOneSingleManyTargetApi {
    type ReturnType = OneToOneSingleManyTarget;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneSingleManyTarget {
            account_id: self.account_id,
        }
    }
}

impl ReconStrategyApiExt for OneToOneManySingleApi {
    type ReturnType = OneToOneManySingle;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneManySingle::V1(OneToOneManySingleV1 {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_diesel_model(),
            target_account: self.target_account.to_diesel_model(),
            source_accumulation_mode: self.source_accumulation_mode,
        })
    }
}

impl ReconStrategyApiExt for OneToOneManySingleSourceApi {
    type ReturnType = OneToOneManySingleSource;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneManySingleSource {
            account_id: self.account_id,
            trigger: self.trigger,
            grouping_field: self.grouping_field,
        }
    }
}

impl ReconStrategyApiExt for OneToOneManySingleTargetApi {
    type ReturnType = OneToOneManySingleTarget;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneManySingleTarget {
            account_id: self.account_id,
        }
    }
}

impl ReconStrategyApiExt for OneToOneManyManyApi {
    type ReturnType = OneToOneManyMany;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneManyMany::V1(OneToOneManyManyV1 {
            search_identifier: self.search_identifier,
            match_rules: self.match_rules,
            source_account: self.source_account.to_diesel_model(),
            target_account: self.target_account.to_diesel_model(),
        })
    }
}

impl ReconStrategyApiExt for OneToOneManyManySourceApi {
    type ReturnType = OneToOneManyManySource;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneManyManySource {
            account_id: self.account_id,
            trigger: self.trigger,
            grouping_field: self.grouping_field,
        }
    }
}

impl ReconStrategyApiExt for OneToOneManyManyTargetApi {
    type ReturnType = OneToOneManyManyTarget;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToOneManyManyTarget {
            account_id: self.account_id,
        }
    }
}

// ==============> RuleAccountExt impls <===============

impl RuleAccountExt for OneToOneSingleSingleV1 {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)> {
        vec![
            (
                self.source_account.account_id.clone(),
                ReconRuleAccountType::Source,
            ),
            (
                self.target_account.account_id.clone(),
                ReconRuleAccountType::Target,
            ),
        ]
    }
}

impl RuleAccountExt for OneToOneSingleManyV1 {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)> {
        vec![
            (
                self.source_account.account_id.clone(),
                ReconRuleAccountType::Source,
            ),
            (
                self.target_account.account_id.clone(),
                ReconRuleAccountType::Target,
            ),
        ]
    }
}

impl RuleAccountExt for OneToOneManySingleV1 {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)> {
        vec![
            (
                self.source_account.account_id.clone(),
                ReconRuleAccountType::Source,
            ),
            (
                self.target_account.account_id.clone(),
                ReconRuleAccountType::Target,
            ),
        ]
    }
}

impl RuleAccountExt for OneToOneManyManyV1 {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)> {
        vec![
            (
                self.source_account.account_id.clone(),
                ReconRuleAccountType::Source,
            ),
            (
                self.target_account.account_id.clone(),
                ReconRuleAccountType::Target,
            ),
        ]
    }
}

// ===============> RuleValidator impls <==============

#[async_trait::async_trait]
impl RuleValidator for OneToOneSingleSingleV1 {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()> {
        Self::default_validations(state, self.clone(), profile_id).await?;
        self.source_account.trigger.validate_for_rule()?;
        self.target_account.validate(state, profile_id).await?;
        self.search_identifier.validate_for_rule()?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RuleValidator for OneToOneSingleSingleTarget {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()> {
        let target_account = state
            .store
            .find_account_by_profile_id_and_account_id(profile_id, &self.account_id)
            .await
            .change_context(ReconRuleError::InvalidReconRule {
                message: format!("Target account with id {} does not exist", self.account_id),
            })?;

        super::validate_tolerance_config(
            state,
            profile_id,
            &self.tolerance_config,
            target_account.currency,
        )
        .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl RuleValidator for OneToOneSingleManyV1 {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()> {
        Self::default_validations(state, self.clone(), profile_id).await?;
        self.source_account.trigger.validate_for_rule()?;
        self.search_identifier.validate_for_rule()?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RuleValidator for OneToOneManySingleV1 {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()> {
        Self::default_validations(state, self.clone(), profile_id).await?;
        self.source_account.trigger.validate_for_rule()?;

        self.search_identifier.validate_for_rule()?;
        self.source_account.grouping_field.validate_for_rule()?;
        validate_source_field_sets(
            &self.search_identifier,
            &self.source_account.grouping_field,
            &self.match_rules,
        )?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl RuleValidator for OneToOneManyManyV1 {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()> {
        Self::default_validations(state, self.clone(), profile_id).await?;
        self.source_account.trigger.validate_for_rule()?;

        self.search_identifier.validate_for_rule()?;
        self.source_account.grouping_field.validate_for_rule()?;
        validate_source_field_sets(
            &self.search_identifier,
            &self.source_account.grouping_field,
            &self.match_rules,
        )?;

        Ok(())
    }
}
