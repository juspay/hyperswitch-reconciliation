use common_enums::{ReconRuleAccountType, StagingEntryProcessingMode};
use common_types::recon_rule::aging_config::AgingConfig;
use diesel_models::{
    metadata_schema::MetadataSchema, recon_rule::ReconRule, transaction::Transaction,
};
use error_stack::{Report, ResultExt};
use recon_domain_models::{
    behaviour::ReverseConversion, search_expectation::DomainSearchExpectation,
    staging_entry::DomainStagingEntry,
};
use storage_impl::{
    recon_rule::ReconRuleInterface, recon_rule_account::ReconRuleAccountInterface,
    search_expectation::SearchExpectationInterface,
};

use crate::{
    domain::recon_engine::{flows::ReconStrategyExt, ProcessingError, ProcessingResult},
    routes::AppState,
};

pub enum ProcessingDecision {
    ProcessAsTransaction {
        context: Box<TransactionContext>,
        recon_rule: ReconRule,
    },
    ProcessAsConfirmation {
        context: Box<ConfirmationContext>,
        recon_rule: ReconRule,
    },
}

pub struct ProcessingDecisionOutput {
    pub transaction: Transaction,
    pub aging_config: AgingConfig,
}

pub struct TransactionContext {
    pub rule_id: String,
    pub metadata_schema: MetadataSchema,
}

pub struct ConfirmationContext {
    pub rule_id: String,
    pub domain_search_expectation: DomainSearchExpectation,
    pub metadata_schema: MetadataSchema,
}

pub async fn rule_orchestration_v2(
    state: &AppState,
    staging_entry: &DomainStagingEntry,
    metadata_schema: MetadataSchema,
) -> ProcessingResult<ProcessingDecision> {
    match staging_entry.processing_mode {
        StagingEntryProcessingMode::Transaction => {
            // Find all the Rule Ids who has the account as source
            let rule_rule_accounts = state
                .store
                .list_recon_rule_accounts_by_account_id_and_type(
                    &staging_entry.account_id,
                    ReconRuleAccountType::Source,
                )
                .await
                .change_context(ProcessingError::DatabaseError)?;

            let rule_ids = rule_rule_accounts
                .into_iter()
                .map(|source| source.rule_id)
                .collect::<Vec<String>>();

            // Fetch the Recon Rules
            let recon_rules = state
                .store
                .list_recon_rules_by_ids(&staging_entry.profile_id, rule_ids)
                .await
                .change_context(ProcessingError::DatabaseError)?;

            // Filter the Recon Rules based on the trigger and get the highest priority rule
            let recon_rule = recon_rules
                .into_iter()
                .filter(|recon_rule| {
                    recon_rule
                        .strategy
                        .get_recon_engine_type()
                        .evaluate_trigger(staging_entry)
                })
                .max_by_key(|recon_rule| recon_rule.priority)
                .ok_or(Report::new(ProcessingError::NoRulesFound))?;

            Ok(ProcessingDecision::ProcessAsTransaction {
                context: Box::new(TransactionContext {
                    rule_id: recon_rule.id.to_string(),
                    metadata_schema,
                }),
                recon_rule,
            })
        }
        StagingEntryProcessingMode::Confirmation => {
            // Find all the Rule Ids who has the account as target
            let rule_rule_accounts = state
                .store
                .list_recon_rule_accounts_by_account_id_and_type(
                    &staging_entry.account_id,
                    ReconRuleAccountType::Target,
                )
                .await
                .change_context(ProcessingError::DatabaseError)?;

            let rule_ids = rule_rule_accounts
                .into_iter()
                .map(|target| target.rule_id)
                .collect::<Vec<String>>();

            // Fetch the Recon Rules
            let recon_rules = state
                .store
                .list_recon_rules_by_ids(&staging_entry.profile_id, rule_ids)
                .await
                .change_context(ProcessingError::DatabaseError)?;

            // Collect criteria with their associated rule priority
            let search_criteria: Vec<_> = recon_rules
                .iter()
                .filter_map(|rule| {
                    rule.strategy
                        .get_recon_engine_type()
                        .get_search_criteria(staging_entry, &rule.id)
                        .ok()
                })
                .flatten()
                .collect();

            let search_expectations = state
                .store
                .find_for_multiple_targets(&staging_entry.account_id, search_criteria)
                .await
                .change_context(ProcessingError::DatabaseError)?;

            let search_expectation = match search_expectations.as_slice() {
                [search_expectation] => search_expectation.to_owned(),
                [] => return Err(ProcessingError::NoExpectationEntryFound.into()),
                _ => {
                    return Err(ProcessingError::MultipleExpectedEntriesFound {
                        transaction_ids: search_expectations
                            .into_iter()
                            .map(|search_expectation| search_expectation.primary_transaction_id)
                            .collect(),
                    }
                    .into())
                }
            };

            let recon_rule = recon_rules
                .into_iter()
                .find(|rule| rule.id == search_expectation.rule_id)
                .ok_or(ProcessingError::NoExpectationEntryFound)?;

            let merchant_encryptor = state
                .encryption_client
                .get_encryptor_for_merchant(staging_entry.merchant_id.clone());

            let domain_search_expectation = search_expectation
                .convert_back(&merchant_encryptor)
                .await
                .change_context(ProcessingError::InternalError(
                    "Failed to decrypt staging_entry".to_string(),
                ))?;

            Ok(ProcessingDecision::ProcessAsConfirmation {
                context: Box::new(ConfirmationContext {
                    rule_id: recon_rule.id.to_string(),
                    domain_search_expectation,
                    metadata_schema,
                }),
                recon_rule,
            })
        }
    }
}
