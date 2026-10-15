use crate::schema::recon_rule;
use common_types::recon_rule::{aging_config::AgingConfig, ReconStrategy};
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = recon_rule)]
pub struct ReconRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub is_active: bool,
    pub priority: i16,
    pub profile_id: String,
    pub strategy: ReconStrategy,
    pub aging_config: AgingConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = recon_rule)]
pub struct ReconRuleNew {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub is_active: bool,
    pub priority: i16,
    pub profile_id: String,
    pub strategy: ReconStrategy,
    pub aging_config: AgingConfig,
}

impl ReconRuleNew {
    pub fn new(
        rule_name: String,
        rule_description: String,
        priority: u8,
        profile_id: String,
        strategy: ReconStrategy,
        aging_config: AgingConfig,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("rule");
        Self {
            id,
            name: rule_name,
            description: rule_description,
            created_at: now,
            last_modified_at: now,
            is_active: true, // Default to active when creating
            priority: i16::from(priority),
            profile_id,
            strategy,
            aging_config,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = recon_rule)]
pub struct ReconRuleUpdateInternal {
    pub name: Option<String>,
    pub description: Option<String>,
    pub strategy: Option<ReconStrategy>,
    pub aging_config: Option<AgingConfig>,
    pub priority: Option<i16>,
    pub is_active: Option<bool>,
    pub last_modified_at: PrimitiveDateTime,
}

/// Enum representing different types of recon rule updates
#[derive(Debug, Clone)]
pub enum ReconRuleUpdate {
    /// Update rule name and/or description
    DetailsUpdate {
        rule_name: Option<String>,
        rule_description: Option<String>,
    },
    /// Update strategy
    StrategyUpdate {
        strategy: Box<ReconStrategy>,
    },
    /// Update aging configuration
    AgingConfigUpdate {
        aging_config: AgingConfig,
    },
    PriorityUpdate {
        priority: i16,
    },
    ActiveStatusUpdate {
        is_active: bool,
    },
}

impl From<ReconRuleUpdate> for ReconRuleUpdateInternal {
    fn from(recon_rule_update: ReconRuleUpdate) -> Self {
        let last_modified_at = date_time::now();

        match recon_rule_update {
            ReconRuleUpdate::DetailsUpdate {
                rule_name,
                rule_description,
            } => Self {
                name: rule_name,
                description: rule_description,
                strategy: None,
                aging_config: None,
                priority: None,
                is_active: None,
                last_modified_at,
            },
            ReconRuleUpdate::StrategyUpdate { strategy } => Self {
                name: None,
                description: None,
                strategy: Some(*strategy),
                aging_config: None,
                priority: None,
                is_active: None,
                last_modified_at,
            },
            ReconRuleUpdate::AgingConfigUpdate { aging_config } => Self {
                name: None,
                description: None,
                strategy: None,
                aging_config: Some(aging_config),
                priority: None,
                is_active: None,
                last_modified_at,
            },
            ReconRuleUpdate::PriorityUpdate { priority } => Self {
                name: None,
                description: None,
                strategy: None,
                aging_config: None,
                priority: Some(priority),
                is_active: None,
                last_modified_at,
            },
            ReconRuleUpdate::ActiveStatusUpdate { is_active } => Self {
                name: None,
                description: None,
                strategy: None,
                aging_config: None,
                priority: None,
                is_active: Some(is_active),
                last_modified_at,
            },
        }
    }
}
