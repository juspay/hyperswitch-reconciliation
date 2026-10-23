use common_types::recon_rule::{
    grouping_field::GroupingField,
    match_rules::MatchRules,
    search_identifier::SearchIdentifier,
    strategies::{
        one_to_many_strategy::{FixedSplitValue, PercentageSplitValue},
        one_to_one_strategy::SourceAccumulationMode,
    },
    tolerance_config::ToleranceConfig,
    trigger::Trigger,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

pub mod aging_config;

pub use aging_config::{
    AgingConfigApiRequest, AgingConfigApiResponse, ThresholdApiRequest, ThresholdApiResponse,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RuleData {
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
    #[schema(example = "Payment Processing Rule")]
    pub rule_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GetReconRuleResponse {
    pub rule_id: String,
    pub rule_name: String,
    pub rule_description: String,
    pub priority: i16,
    pub is_active: bool,
    pub profile_id: String,
    pub strategy: ReconStrategy,
    pub aging_config: AgingConfigApiResponse,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "recon_strategy_type", rename_all = "snake_case")]
pub enum ReconStrategy {
    OneToOne(OneToOneStrategy),
    OneToMany(OneToManyStrategy),
}

// =============> One To One <============

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_one_type", rename_all = "snake_case")]
pub enum OneToOneStrategy {
    SingleSingle(OneToOneSingleSingle),
    SingleMany(OneToOneSingleMany),
    ManySingle(OneToOneManySingle),
    ManyMany(OneToOneManyMany),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleSingle {
    pub search_identifier: SearchIdentifier,
    pub match_rules: MatchRules,
    pub source_account: OneToOneSingleSingleSource,
    pub target_account: OneToOneSingleSingleTarget,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleSingleSource {
    pub account_id: String,
    pub trigger: Trigger,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleSingleTarget {
    pub account_id: String,
    pub tolerance_config: Option<ToleranceConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleMany {
    pub search_identifier: SearchIdentifier,
    pub match_rules: MatchRules,
    pub source_account: OneToOneSingleManySource,
    pub target_account: OneToOneSingleManyTarget,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleManySource {
    pub account_id: String,
    pub trigger: Trigger,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneSingleManyTarget {
    pub account_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManySingle {
    pub search_identifier: SearchIdentifier,
    pub match_rules: MatchRules,
    pub source_account: OneToOneManySingleSource,
    pub target_account: OneToOneManySingleTarget,
    #[serde(default)]
    pub source_accumulation_mode: SourceAccumulationMode,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManySingleSource {
    pub account_id: String,
    pub trigger: Trigger,
    pub grouping_field: GroupingField,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManySingleTarget {
    pub account_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManyMany {
    pub search_identifier: SearchIdentifier,
    pub match_rules: MatchRules,
    pub source_account: OneToOneManyManySource,
    pub target_account: OneToOneManyManyTarget,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManyManySource {
    pub account_id: String,
    pub trigger: Trigger,
    pub grouping_field: GroupingField,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToOneManyManyTarget {
    pub account_id: String,
}

// =============> One To Many <============

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "one_to_many_type", rename_all = "snake_case")]
pub enum OneToManyStrategy {
    SingleSingle(OneToManySingleSingle),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToManySingleSingle {
    pub source_account: OneToManySingleSingleSource,
    pub target_accounts: OneToManySingleSingleTargetType,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToManySingleSingleSource {
    pub account_id: String,
    pub trigger: Trigger,
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OneToManySingleSingleTarget {
    pub search_identifier: SearchIdentifier,
    pub match_rules: MatchRules,
    pub account_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconRuleUpdateRequest {
    #[schema(example = "Updated Rule Name")]
    pub rule_name: Option<String>,
    #[schema(example = "Updated rule description")]
    pub rule_description: Option<String>,
    pub aging_config: Option<AgingConfigApiRequest>,
    #[schema(example = 100)]
    pub priority: Option<u8>,
    pub is_active: Option<bool>,
    pub strategy: Option<ReconStrategy>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconRuleCreateRequest {
    pub rule_name: String,
    pub rule_description: String,
    #[schema(example = 100)]
    pub priority: u8,
    pub strategy: ReconStrategy,
    pub aging_config: AgingConfigApiRequest,
}
