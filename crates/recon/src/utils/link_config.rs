use api_models::link_config::{
    link_matching_key as api_key, link_trigger as api_trigger, GetLinkConfigResponse,
    LinkConfigCreateRequest, LinkConfigUpdateRequest,
};
use common_types::link_config::{
    link_matching_key as storage_key, link_trigger as storage_trigger,
};
use diesel_models::link_config::LinkConfig;

use crate::{
    db::{account::AccountInterface, recon_rule::ReconRuleInterface},
    errors::{
        conversions::StorageErrorExt,
        link_config::{LinkConfigError, LinkConfigResult},
    },
    AppState,
};

pub trait LinkTriggerApiExt {
    fn to_diesel_model(self) -> storage_trigger::LinkTrigger;
}

impl LinkTriggerApiExt for api_trigger::LinkTrigger {
    fn to_diesel_model(self) -> storage_trigger::LinkTrigger {
        storage_trigger::LinkTrigger::V1(storage_trigger::LinkTriggerV1 {
            any: self
                .any
                .into_iter()
                .map(|group| storage_trigger::LinkTriggerGroup {
                    all: group
                        .all
                        .into_iter()
                        .map(|condition| storage_trigger::LinkTriggerCondition {
                            field: condition.field,
                            operator: match condition.operator {
                                api_trigger::LinkTriggerOperator::Equals => {
                                    storage_trigger::LinkTriggerOperator::Equals
                                }
                                api_trigger::LinkTriggerOperator::NotEquals => {
                                    storage_trigger::LinkTriggerOperator::NotEquals
                                }
                            },
                            value: condition.value,
                        })
                        .collect(),
                })
                .collect(),
        })
    }
}

pub trait LinkTriggerExt {
    fn to_api_response(self) -> api_trigger::LinkTrigger;
}

impl LinkTriggerExt for storage_trigger::LinkTrigger {
    fn to_api_response(self) -> api_trigger::LinkTrigger {
        match self {
            Self::V1(trigger) => api_trigger::LinkTrigger {
                any: trigger
                    .any
                    .into_iter()
                    .map(|group| api_trigger::LinkTriggerGroup {
                        all: group
                            .all
                            .into_iter()
                            .map(|condition| api_trigger::LinkTriggerCondition {
                                field: condition.field,
                                operator: match condition.operator {
                                    storage_trigger::LinkTriggerOperator::Equals => {
                                        api_trigger::LinkTriggerOperator::Equals
                                    }
                                    storage_trigger::LinkTriggerOperator::NotEquals => {
                                        api_trigger::LinkTriggerOperator::NotEquals
                                    }
                                },
                                value: condition.value,
                            })
                            .collect(),
                    })
                    .collect(),
            },
        }
    }
}

pub trait LinkSearchIdentifierApiExt {
    fn to_diesel_model(self) -> storage_key::LinkSearchIdentifier;
}

impl LinkSearchIdentifierApiExt for api_key::LinkSearchIdentifier {
    fn to_diesel_model(self) -> storage_key::LinkSearchIdentifier {
        let convert_key = |key: api_key::LinkIdentifierKey| storage_key::LinkIdentifierKey {
            transaction_field: key.transaction_field,
            confirmation_field: key.confirmation_field,
        };
        storage_key::LinkSearchIdentifier::V1(match self {
            Self::Single(single) => {
                storage_key::LinkSearchIdentifierV1::Single(storage_key::SingleLinkIdentifier {
                    key: convert_key(single.key),
                })
            }
            Self::Composite(composite) => storage_key::LinkSearchIdentifierV1::Composite(
                storage_key::CompositeLinkIdentifier {
                    keys: composite.keys.into_iter().map(convert_key).collect(),
                    delimiter: composite.delimiter,
                },
            ),
        })
    }
}

pub trait LinkSearchIdentifierExt {
    fn to_api_response(self) -> api_key::LinkSearchIdentifier;
}

impl LinkSearchIdentifierExt for storage_key::LinkSearchIdentifier {
    fn to_api_response(self) -> api_key::LinkSearchIdentifier {
        let convert_key = |key: storage_key::LinkIdentifierKey| api_key::LinkIdentifierKey {
            transaction_field: key.transaction_field,
            confirmation_field: key.confirmation_field,
        };
        match self {
            Self::V1(identifier) => match identifier {
                storage_key::LinkSearchIdentifierV1::Single(single) => {
                    api_key::LinkSearchIdentifier::Single(api_key::SingleLinkIdentifier {
                        key: convert_key(single.key),
                    })
                }
                storage_key::LinkSearchIdentifierV1::Composite(composite) => {
                    api_key::LinkSearchIdentifier::Composite(api_key::CompositeLinkIdentifier {
                        keys: composite.keys.into_iter().map(convert_key).collect(),
                        delimiter: composite.delimiter,
                    })
                }
            },
        }
    }
}

pub trait LinkConfigExt {
    fn to_api_response(self) -> GetLinkConfigResponse;
}

impl LinkConfigExt for LinkConfig {
    fn to_api_response(self) -> GetLinkConfigResponse {
        GetLinkConfigResponse {
            link_config_id: self.id,
            profile_id: self.profile_id,
            account_id: self.account_id,
            transaction_rule_id: self.transaction_rule_id,
            confirmation_rule_id: self.confirmation_rule_id,
            transaction_trigger: self.transaction_trigger.to_api_response(),
            confirmation_trigger: self.confirmation_trigger.to_api_response(),
            link_search_identifier: self.link_search_identifier.to_api_response(),
            priority: self.priority,
            is_active: self.is_active,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}

#[async_trait::async_trait]
pub trait LinkConfigValidator {
    async fn validate_with_db(&self, state: &AppState, profile_id: &str) -> LinkConfigResult<()>;
}

#[async_trait::async_trait]
impl LinkConfigValidator for LinkConfigCreateRequest {
    async fn validate_with_db(&self, state: &AppState, profile_id: &str) -> LinkConfigResult<()> {
        validate_link_search_identifier(&self.link_search_identifier)?;

        state
            .store
            .find_account_by_profile_id_and_account_id(profile_id, &self.account_id)
            .await
            .to_not_found_response(|| LinkConfigError::AccountNotFound {
                account_id: self.account_id.clone(),
            })?;
        validate_rule_ids(
            state,
            profile_id,
            &[&self.transaction_rule_id, &self.confirmation_rule_id],
        )
        .await
    }
}

#[async_trait::async_trait]
impl LinkConfigValidator for LinkConfigUpdateRequest {
    async fn validate_with_db(&self, state: &AppState, profile_id: &str) -> LinkConfigResult<()> {
        if let Some(identifier) = &self.link_search_identifier {
            validate_link_search_identifier(identifier)?;
        }

        let rule_ids: Vec<&str> = [
            self.transaction_rule_id.as_deref(),
            self.confirmation_rule_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();

        validate_rule_ids(state, profile_id, &rule_ids).await
    }
}

fn validate_link_search_identifier(
    identifier: &api_key::LinkSearchIdentifier,
) -> LinkConfigResult<()> {
    if let api_key::LinkSearchIdentifier::Composite(composite) = identifier {
        if composite.keys.is_empty() {
            return Err(LinkConfigError::InvalidLinkConfig {
                message: "Composite matching identifier must contain at least one key".to_owned(),
            }
            .into());
        }
    }
    Ok(())
}

async fn validate_rule_ids(
    state: &AppState,
    profile_id: &str,
    rule_ids: &[&str],
) -> LinkConfigResult<()> {
    for &rule_id in rule_ids {
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
