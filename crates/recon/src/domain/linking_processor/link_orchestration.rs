use common_enums::EntryLinkSide;
use diesel_models::link_config::LinkConfig;
use error_stack::{report, ResultExt};
use recon_domain_models::{entry::DomainEntry, transaction::DomainTransaction};

use storage_impl::link_config::LinkConfigInterface;

use crate::{
    domain::linking_processor::{
        implementations::link_trigger::LinkTriggerExt, LinkProcessingResult, LinkingProcessingError,
    },
    routes::AppState,
};

pub struct LinkProcessingContext {
    pub link_config: LinkConfig,
    pub entry_link_side: EntryLinkSide,
}

pub async fn link_orchestration(
    state: &AppState,
    domain_entry: &DomainEntry,
    domain_transaction: DomainTransaction,
    entry_link_side: EntryLinkSide,
) -> LinkProcessingResult<LinkProcessingContext> {
    let active_link_configs = state
        .store
        .list_active_link_configs_by_acc_rule_profile(
            &domain_entry.account_id,
            &domain_transaction.rule_id,
            &domain_transaction.profile_id,
            &entry_link_side,
        )
        .await
        .change_context(LinkingProcessingError::DatabaseError)?;

    let link_config = active_link_configs
        .into_iter()
        .filter(|link_config| match entry_link_side {
            EntryLinkSide::Transaction => link_config.transaction_trigger.validate(domain_entry),
            EntryLinkSide::Confirmation => link_config.confirmation_trigger.validate(domain_entry),
        })
        .max_by_key(|link_config| link_config.priority)
        .ok_or(report!(LinkingProcessingError::NoLinkConfigFound))?;

    Ok(LinkProcessingContext {
        link_config,
        entry_link_side,
    })
}
