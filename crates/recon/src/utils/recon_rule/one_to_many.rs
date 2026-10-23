use api_models::recon_rule::{
    OneToManySingleSingle as OneToManySingleSingleApi,
    OneToManySingleSingleSource as OneToManySingleSingleSourceApi,
    OneToManySingleSingleTarget as OneToManySingleSingleTargetApi,
    OneToManySingleSingleTargetType as OneToManySingleSingleTargetTypeApi,
    OneToManyStrategy as OneToManyStrategyApi,
};
use common_enums::ReconRuleAccountType;
use common_types::recon_rule::strategies::one_to_many_strategy::{
    OneToManySingleSingle, OneToManySingleSingleSource, OneToManySingleSingleTarget,
    OneToManySingleSingleTargetType, OneToManySingleSingleV1, OneToManyStrategy,
    OneToManyStrategyV1,
};

use crate::{
    errors::ReconRuleResult,
    utils::recon_rule::{
        AppState, ReconStrategyApiExt, ReconStrategyExt, RuleAccountExt, RuleValidator,
        SearchIdentifierRuleValidator, TriggerRuleValidator,
    },
};

// ===========> ReconStrategyExt impls <=============

impl ReconStrategyExt for OneToManyStrategy {
    type ReturnType = OneToManyStrategyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(one_to_many_v1) => one_to_many_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToManyStrategyV1 {
    type ReturnType = OneToManyStrategyApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::SingleSingle(single_single) => {
                Self::ReturnType::SingleSingle(single_single.to_api_response())
            }
        }
    }
}

impl ReconStrategyExt for OneToManySingleSingle {
    type ReturnType = OneToManySingleSingleApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::V1(single_single_v1) => single_single_v1.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToManySingleSingleV1 {
    type ReturnType = OneToManySingleSingleApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            source_account: self.source_account.to_api_response(),
            target_accounts: self.target_accounts.to_api_response(),
        }
    }
}

impl ReconStrategyExt for OneToManySingleSingleSource {
    type ReturnType = OneToManySingleSingleSourceApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            trigger: self.trigger,
        }
    }
}

impl ReconStrategyExt for OneToManySingleSingleTargetType {
    type ReturnType = OneToManySingleSingleTargetTypeApi;
    fn to_api_response(self) -> Self::ReturnType {
        match self {
            Self::Percentage { targets } => Self::ReturnType::Percentage {
                targets: targets
                    .into_iter()
                    .map(|(target, split)| (target.to_api_response(), split))
                    .collect(),
            },
            Self::Fixed { targets } => Self::ReturnType::Fixed {
                targets: targets
                    .into_iter()
                    .map(|(target, split)| (target.to_api_response(), split))
                    .collect(),
            },
        }
    }
}

impl ReconStrategyExt for OneToManySingleSingleTarget {
    type ReturnType = OneToManySingleSingleTargetApi;
    fn to_api_response(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            match_rules: self.match_rules,
            search_identifier: self.search_identifier,
        }
    }
}

// ==========> ReconStrategyApiExt impls <=============

impl ReconStrategyApiExt for OneToManyStrategyApi {
    type ReturnType = OneToManyStrategy;
    fn to_diesel_model(self) -> Self::ReturnType {
        match self {
            Self::SingleSingle(single_single) => OneToManyStrategy::V1(
                OneToManyStrategyV1::SingleSingle(single_single.to_diesel_model()),
            ),
        }
    }
}

impl ReconStrategyApiExt for OneToManySingleSingleApi {
    type ReturnType = OneToManySingleSingle;
    fn to_diesel_model(self) -> Self::ReturnType {
        OneToManySingleSingle::V1(OneToManySingleSingleV1 {
            source_account: self.source_account.to_diesel_model(),
            target_accounts: self.target_accounts.to_diesel_model(),
        })
    }
}

impl ReconStrategyApiExt for OneToManySingleSingleTargetTypeApi {
    type ReturnType = OneToManySingleSingleTargetType;
    fn to_diesel_model(self) -> Self::ReturnType {
        match self {
            Self::Percentage { targets } => OneToManySingleSingleTargetType::Percentage {
                targets: targets
                    .into_iter()
                    .map(|(target, split)| (target.to_diesel_model(), split))
                    .collect(),
            },
            Self::Fixed { targets } => OneToManySingleSingleTargetType::Fixed {
                targets: targets
                    .into_iter()
                    .map(|(target, split)| (target.to_diesel_model(), split))
                    .collect(),
            },
        }
    }
}

impl ReconStrategyApiExt for OneToManySingleSingleSourceApi {
    type ReturnType = OneToManySingleSingleSource;
    fn to_diesel_model(self) -> Self::ReturnType {
        Self::ReturnType {
            trigger: self.trigger,
            account_id: self.account_id,
        }
    }
}

impl ReconStrategyApiExt for OneToManySingleSingleTargetApi {
    type ReturnType = OneToManySingleSingleTarget;
    fn to_diesel_model(self) -> Self::ReturnType {
        Self::ReturnType {
            account_id: self.account_id,
            match_rules: self.match_rules,
            search_identifier: self.search_identifier,
        }
    }
}

// ==============> RuleAccountExt impls <===============

impl RuleAccountExt for OneToManySingleSingleV1 {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)> {
        self.target_accounts
            .get_rule_accounts()
            .into_iter()
            .chain(std::iter::once((
                self.source_account.account_id.clone(),
                ReconRuleAccountType::Source,
            )))
            .collect()
    }
}

impl RuleAccountExt for OneToManySingleSingleTargetType {
    fn get_rule_accounts(&self) -> Vec<(String, ReconRuleAccountType)> {
        match self {
            Self::Percentage { targets } => targets
                .iter()
                .map(|(target, _)| (target.account_id.clone(), ReconRuleAccountType::Target))
                .collect(),
            Self::Fixed { targets } => targets
                .iter()
                .map(|(target, _)| (target.account_id.clone(), ReconRuleAccountType::Target))
                .collect(),
        }
    }
}

// ===============> RuleValidator impls <==============

#[async_trait::async_trait]
impl RuleValidator for OneToManySingleSingleV1 {
    async fn validate(&self, state: &AppState, profile_id: &str) -> ReconRuleResult<()> {
        Self::default_validations(state, self.clone(), profile_id).await?;
        self.source_account.trigger.validate_for_rule()?;
        for target_account in self.target_accounts.get_target_accounts() {
            target_account.validate(state, profile_id).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl RuleValidator for OneToManySingleSingleTarget {
    async fn validate(&self, _: &AppState, _: &str) -> ReconRuleResult<()> {
        self.search_identifier.validate_for_rule()?;
        Ok(())
    }
}
