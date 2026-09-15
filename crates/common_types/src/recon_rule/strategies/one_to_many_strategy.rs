use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    recon_rule::{link_with_rule::LinkWithRule, match_rules, search_identifier, trigger},
    units::MinorUnit,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_many_strategy_version", rename_all = "snake_case")]
pub enum OneToManyStrategy {
    V1(OneToManyStrategyV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_many_strategy_type", rename_all = "snake_case")]
pub enum OneToManyStrategyV1 {
    SingleSingle(OneToManySingleSingle),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_many_single_single_version", rename_all = "snake_case")]
pub enum OneToManySingleSingle {
    V1(OneToManySingleSingleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToManySingleSingleV1 {
    pub source_account: OneToManySingleSingleSource,
    pub target_accounts: OneToManySingleSingleTargetType,
    pub link_with_rule: Option<LinkWithRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToManySingleSingleSource {
    pub account_id: String,
    pub trigger: trigger::Trigger,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "split_type", rename_all = "snake_case")]
pub enum OneToManySingleSingleTargetType {
    Percentage {
        targets: Vec<(OneToManySingleSingleTarget, PercentageSplitValue)>,
    },
    Fixed {
        targets: Vec<(OneToManySingleSingleTarget, FixedSplitValue)>,
    },
}

impl OneToManySingleSingleTargetType {
    pub fn get_target_accounts(&self) -> Vec<OneToManySingleSingleTarget> {
        match self {
            Self::Fixed { targets } => targets.iter().map(|t| t.0.clone()).collect(),
            Self::Percentage { targets } => targets.iter().map(|t| t.0.clone()).collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToManySingleSingleTarget {
    pub account_id: String,
    pub search_identifier: search_identifier::SearchIdentifier,
    pub match_rules: match_rules::MatchRules,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PercentageSplitValue {
    pub value: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "fixed_split_type", rename_all = "snake_case")]
pub enum FixedSplitValue {
    Amount { value: MinorUnit },
    Remaining,
}
