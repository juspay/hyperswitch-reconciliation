use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::{
    grouping_field::GroupingField, link_with_rule::LinkWithRule, match_rules, search_identifier,
    tolerance_config::ToleranceConfig, trigger,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_one_strategy_version", rename_all = "snake_case")]
pub enum OneToOneStrategy {
    V1(OneToOneStrategyV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_one_strategy_type", rename_all = "snake_case")]
pub enum OneToOneStrategyV1 {
    SingleSingle(OneToOneSingleSingle),
    SingleMany(OneToOneSingleMany),
    ManySingle(OneToOneManySingle),
    ManyMany(OneToOneManyMany),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_one_single_single_version", rename_all = "snake_case")]
pub enum OneToOneSingleSingle {
    V1(OneToOneSingleSingleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleSingleV1 {
    pub search_identifier: search_identifier::SearchIdentifier,
    pub match_rules: match_rules::MatchRules,
    pub source_account: OneToOneSingleSingleSource,
    pub target_account: OneToOneSingleSingleTarget,
    #[serde(default)] // Need this because of backward compatibility
    pub link_with_rule: Option<LinkWithRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleSingleSource {
    pub account_id: String,
    pub trigger: trigger::Trigger,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleSingleTarget {
    pub account_id: String,
    #[serde(default)]
    pub tolerance_config: Option<ToleranceConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_one_single_many_version", rename_all = "snake_case")]
pub enum OneToOneSingleMany {
    V1(OneToOneSingleManyV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleManyV1 {
    pub search_identifier: search_identifier::SearchIdentifier,
    pub match_rules: match_rules::MatchRules,
    pub source_account: OneToOneSingleManySource,
    pub target_account: OneToOneSingleManyTarget,
    pub link_with_rule: Option<LinkWithRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleManySource {
    pub account_id: String,
    pub trigger: trigger::Trigger,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleManyTarget {
    pub account_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_one_many_single_version", rename_all = "snake_case")]
pub enum OneToOneManySingle {
    V1(OneToOneManySingleV1),
}

/// Controls how additional source entries evolve an existing grouped transaction.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceAccumulationMode {
    /// Rebuild the transaction as a new version for every source entry.
    #[default]
    Versioned,
    /// Append sources in place while the transaction is still awaiting its target.
    IncrementalWhileExpected,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManySingleV1 {
    pub search_identifier: search_identifier::SearchIdentifier,
    pub match_rules: match_rules::MatchRules,
    pub source_account: OneToOneManySingleSource,
    pub target_account: OneToOneManySingleTarget,
    pub link_with_rule: Option<LinkWithRule>,
    #[serde(default)]
    pub source_accumulation_mode: SourceAccumulationMode,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManySingleSource {
    pub account_id: String,
    pub trigger: trigger::Trigger,
    pub grouping_field: GroupingField,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManySingleTarget {
    pub account_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_one_many_many_version", rename_all = "snake_case")]
pub enum OneToOneManyMany {
    V1(OneToOneManyManyV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManyManyV1 {
    pub search_identifier: search_identifier::SearchIdentifier,
    pub match_rules: match_rules::MatchRules,
    pub source_account: OneToOneManyManySource,
    pub target_account: OneToOneManyManyTarget,
    pub link_with_rule: Option<LinkWithRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManyManySource {
    pub account_id: String,
    pub trigger: trigger::Trigger,
    pub grouping_field: GroupingField,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManyManyTarget {
    pub account_id: String,
}
