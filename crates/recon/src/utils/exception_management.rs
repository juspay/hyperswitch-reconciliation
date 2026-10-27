use std::collections::{HashMap, HashSet};

use api_models::transaction::{EntryOp, NewEntry};
use common_enums::{Currency, EntryStatus};
use common_types::amount::AmountType;
use common_utils::auth::AuthContext;
use diesel_models::{recon_rule::ReconRule, transaction::Transaction};
use error_stack::ResultExt;
use storage_impl::account::AccountInterface;

use crate::{
    domain::{
        exception_management::manual_reconciliation::{ExistingEntry, PreparedEntry},
        recon_engine::flows::ReconStrategyExt,
        transaction::MAX_ENTRIES_PER_TRANSACTION,
    },
    errors::transaction::{TransactionError, TransactionResult},
    routes::AppState,
    utils::recon_rule::RuleAccountExt,
};

pub trait EntryOpsVecExt {
    fn validate_payload(&self, transaction: &Transaction) -> TransactionResult<()>;

    fn validate_with_old_entries(
        &self,
        old_entries: &HashMap<String, ExistingEntry>,
    ) -> TransactionResult<()>;

    async fn validate_referenced_accounts_from_db(
        &self,
        auth: &AuthContext,
        state: &AppState,
        transaction_currency: Currency,
    ) -> TransactionResult<()>;

    fn linked_staging_entry_ids(&self) -> Vec<&str>;

    fn referenced_config_ids(
        &self,
        old_entries: &HashMap<String, ExistingEntry>,
    ) -> HashSet<String>;

    fn transformations_to_validate<'a>(
        &'a self,
        old_entries: &'a HashMap<String, ExistingEntry>,
    ) -> TransactionResult<Vec<(&'a str, &'a str)>>;
}

impl EntryOpsVecExt for [EntryOp] {
    fn validate_payload(&self, transaction: &Transaction) -> TransactionResult<()> {
        if self.is_empty() {
            return Err(TransactionError::EmptyEntryOps.into());
        }

        let mut seen = HashSet::with_capacity(self.len());
        let duplicate = self
            .iter()
            .flat_map(|op| match op {
                EntryOp::UpdateEntry { entry_id, .. }
                | EntryOp::MarkReceived { entry_id, .. }
                | EntryOp::RemoveEntry { entry_id }
                | EntryOp::CreateWithStagingEntry {
                    staging_entry_id: entry_id,
                    ..
                } => vec![entry_id.as_str()],
                EntryOp::ReplaceWithStagingEntry {
                    entry_id,
                    staging_entry_id,
                    ..
                } => vec![entry_id.as_str(), staging_entry_id.as_str()],
                EntryOp::CreateEntry { .. } => Vec::new(),
            })
            .find(|id| !seen.insert(*id));

        if let Some(target_id) = duplicate {
            return Err(TransactionError::DuplicateEntryOpTarget {
                target_id: target_id.to_string(),
            }
            .into());
        }

        let removed_entry_ids: HashSet<&str> = self
            .iter()
            .filter_map(|op| match op {
                EntryOp::RemoveEntry { entry_id } => Some(entry_id.as_str()),
                _ => None,
            })
            .collect();
        let added_entry_count = self
            .iter()
            .filter(|op| {
                matches!(
                    op,
                    EntryOp::CreateEntry { .. } | EntryOp::CreateWithStagingEntry { .. }
                )
            })
            .count();
        let resulting_entry_count = transaction
            .entry_ids
            .iter()
            .filter(|id| !removed_entry_ids.contains(id.as_str()))
            .count()
            + added_entry_count;

        if resulting_entry_count > MAX_ENTRIES_PER_TRANSACTION {
            return Err(TransactionError::TooManyEntries {
                count: resulting_entry_count,
                limit: MAX_ENTRIES_PER_TRANSACTION,
            }
            .into());
        }

        let amounts = self.iter().filter_map(|op| match op {
            EntryOp::CreateEntry { entry } => match entry {
                NewEntry::Direct { amount, .. } | NewEntry::Expected { amount, .. } => {
                    Some(*amount)
                }
            },
            EntryOp::UpdateEntry { overrides, .. } | EntryOp::MarkReceived { overrides, .. } => {
                overrides.amount
            }
            EntryOp::CreateWithStagingEntry { .. }
            | EntryOp::ReplaceWithStagingEntry { .. }
            | EntryOp::RemoveEntry { .. } => None,
        });

        for amount in amounts {
            let amount = AmountType::from_major_unit(amount, transaction.currency).change_context(
                TransactionError::InvalidAmount {
                    details: "Invalid amount provided".to_string(),
                },
            )?;

            if !amount.is_positive() {
                return Err(TransactionError::InvalidAmount {
                    details: "Amount must be greater than zero in minor units".to_string(),
                }
                .into());
            }
        }

        Ok(())
    }

    async fn validate_referenced_accounts_from_db(
        &self,
        auth: &AuthContext,
        state: &AppState,
        transaction_currency: Currency,
    ) -> TransactionResult<()> {
        let referenced_account_ids: Vec<String> =
            self.iter()
                .filter_map(|op| match op {
                    EntryOp::CreateEntry { entry } => match entry {
                        NewEntry::Direct { account_id, .. }
                        | NewEntry::Expected { account_id, .. } => Some(account_id.clone()),
                    },
                    EntryOp::UpdateEntry { .. }
                    | EntryOp::MarkReceived { .. }
                    | EntryOp::CreateWithStagingEntry { .. }
                    | EntryOp::ReplaceWithStagingEntry { .. }
                    | EntryOp::RemoveEntry { .. } => None,
                })
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();

        if referenced_account_ids.is_empty() {
            return Ok(());
        }

        let accounts = state
            .store
            .list_accounts_by_profile_id_and_account_ids(&auth.profile_id, &referenced_account_ids)
            .await
            .change_context(TransactionError::InternalServerError)?;

        let accounts_by_id: HashMap<&str, &_> = accounts
            .iter()
            .map(|account| (account.id.as_str(), account))
            .collect();

        for account_id in &referenced_account_ids {
            let account = accounts_by_id.get(account_id.as_str()).ok_or_else(|| {
                TransactionError::AccountNotFound {
                    account_id: account_id.clone(),
                }
            })?;

            if account.currency != transaction_currency {
                return Err(TransactionError::EntryCurrencyMismatch.into());
            }
        }

        Ok(())
    }

    fn validate_with_old_entries(
        &self,
        old_entries: &HashMap<String, ExistingEntry>,
    ) -> TransactionResult<()> {
        let touched_entry_ids = self.iter().filter_map(|op| match op {
            EntryOp::UpdateEntry { entry_id, .. }
            | EntryOp::MarkReceived { entry_id, .. }
            | EntryOp::ReplaceWithStagingEntry { entry_id, .. }
            | EntryOp::RemoveEntry { entry_id } => Some(entry_id.as_str()),
            EntryOp::CreateEntry { .. } | EntryOp::CreateWithStagingEntry { .. } => None,
        });

        for entry_id in touched_entry_ids {
            if !old_entries.contains_key(entry_id) {
                return Err(TransactionError::EntryNotFound {
                    entry_id: entry_id.to_string(),
                }
                .into());
            }
        }

        for op in self {
            if let EntryOp::MarkReceived { entry_id, .. } = op {
                let op_entry = old_entries
                    .get(entry_id.as_str())
                    .map(|existing| &existing.entry)
                    .ok_or(TransactionError::InternalServerError)?;

                if !matches!(op_entry.status, EntryStatus::Expected) {
                    return Err(TransactionError::InvalidEntryStatus {
                        entry_id: op_entry.id.clone(),
                        status: op_entry.status,
                        expected_statuses: [EntryStatus::Expected].to_vec(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    fn linked_staging_entry_ids(&self) -> Vec<&str> {
        self.iter()
            .filter_map(|op| match op {
                EntryOp::CreateWithStagingEntry {
                    staging_entry_id, ..
                }
                | EntryOp::ReplaceWithStagingEntry {
                    staging_entry_id, ..
                } => Some(staging_entry_id.as_str()),
                EntryOp::UpdateEntry { .. }
                | EntryOp::MarkReceived { .. }
                | EntryOp::RemoveEntry { .. }
                | EntryOp::CreateEntry { .. } => None,
            })
            .collect()
    }

    fn referenced_config_ids(
        &self,
        old_entries: &HashMap<String, ExistingEntry>,
    ) -> HashSet<String> {
        self.iter()
            .filter_map(|op| match op {
                EntryOp::CreateEntry {
                    entry:
                        NewEntry::Direct {
                            transformation_id, ..
                        },
                } => transformation_id.clone(),
                EntryOp::UpdateEntry {
                    entry_id,
                    overrides,
                }
                | EntryOp::MarkReceived {
                    entry_id,
                    overrides,
                } => {
                    if overrides.transformation_id.is_none() && overrides.metadata.is_none() {
                        return None;
                    }

                    overrides.transformation_id.clone().or_else(|| {
                        old_entries
                            .get(entry_id)
                            .and_then(|existing| existing.entry.transformation_config_id.clone())
                    })
                }
                EntryOp::CreateEntry { .. }
                | EntryOp::RemoveEntry { .. }
                | EntryOp::CreateWithStagingEntry { .. }
                | EntryOp::ReplaceWithStagingEntry { .. } => None,
            })
            .collect()
    }

    fn transformations_to_validate<'a>(
        &'a self,
        old_entries: &'a HashMap<String, ExistingEntry>,
    ) -> TransactionResult<Vec<(&'a str, &'a str)>> {
        self.iter()
            .filter_map(|op| match op {
                EntryOp::CreateEntry {
                    entry:
                        NewEntry::Direct {
                            account_id,
                            transformation_id: Some(config_id),
                            ..
                        },
                } => Some(Ok((config_id.as_str(), account_id.as_str()))),
                EntryOp::UpdateEntry {
                    entry_id,
                    overrides,
                }
                | EntryOp::MarkReceived {
                    entry_id,
                    overrides,
                } => {
                    let config_id = overrides.transformation_id.as_deref()?;

                    Some(
                        old_entries
                            .get(entry_id)
                            .map(|existing| (config_id, existing.entry.account_id.as_str()))
                            .ok_or_else(|| TransactionError::InternalServerError.into()),
                    )
                }
                EntryOp::CreateEntry { .. }
                | EntryOp::RemoveEntry { .. }
                | EntryOp::CreateWithStagingEntry { .. }
                | EntryOp::ReplaceWithStagingEntry { .. } => None,
            })
            .collect()
    }
}

pub trait PreparedEntryVecExt {
    fn validate_rule_account_coverage(&self, recon_rule: &ReconRule) -> TransactionResult<()>;
}

impl PreparedEntryVecExt for [PreparedEntry] {
    fn validate_rule_account_coverage(&self, recon_rule: &ReconRule) -> TransactionResult<()> {
        let covered: HashSet<&str> = self
            .iter()
            .map(|prepared| prepared.entry.account_id.as_str())
            .collect();

        let missing: Vec<String> = recon_rule
            .strategy
            .get_recon_engine_type()
            .get_rule_accounts()
            .into_iter()
            .map(|(account_id, _)| account_id)
            .filter(|account_id| !covered.contains(account_id.as_str()))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if missing.is_empty() {
            return Ok(());
        }

        Err(TransactionError::MissingRuleAccountEntries {
            account_ids: missing,
        }
        .into())
    }
}
