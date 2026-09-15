use common_enums::{BalanceDirection, Currency, DieselTransactionStatus};
use common_types::{
    transaction::{status::DomainTransactionStatus, TransactionData, TransactionDataV3},
    units::MinorUnit,
};
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    account::Account, diesel_impl, entry::Entry, recon_rule::ReconRule, schema::transaction,
};

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = transaction, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct Transaction {
    pub id: String,
    pub transaction_id: String,
    #[diesel(deserialize_as = diesel_impl::DieselArray<String>)]
    pub entry_ids: Vec<String>,
    pub created_at: PrimitiveDateTime,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub last_modified_at: PrimitiveDateTime,
    pub discarded_status: Option<DieselTransactionStatus>,
    pub version: i32,
    pub profile_id: String,
    pub rule_id: String,
    pub credit_amount: MinorUnit,
    pub debit_amount: MinorUnit,
    pub currency: Currency,
    pub data: TransactionData,
    pub discarded_data: Option<TransactionData>,
    pub effective_at: PrimitiveDateTime,
    pub status: DieselTransactionStatus,
    pub modified_by: Option<String>,
}

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = transaction, check_for_backend(diesel::pg::Pg))]
pub struct TransactionNew {
    pub id: String,
    pub transaction_id: String,
    #[diesel(deserialize_as = diesel_impl::DieselArray<String>)]
    pub entry_ids: Vec<String>,
    pub created_at: PrimitiveDateTime,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub last_modified_at: PrimitiveDateTime,
    pub discarded_status: Option<DieselTransactionStatus>,
    pub version: i32,
    pub profile_id: String,
    pub rule_id: String,
    pub credit_amount: MinorUnit,
    pub debit_amount: MinorUnit,
    pub currency: Currency,
    pub data: TransactionData,
    pub discarded_data: Option<TransactionData>,
    pub effective_at: PrimitiveDateTime,
    pub status: DieselTransactionStatus,
    pub modified_by: Option<String>,
}

impl TransactionNew {
    pub fn new_versioned(
        previous: Transaction,
        update: TransactionVersionUpdate,
        modified_by: Option<String>,
    ) -> Self {
        let id = generate_id_with_default_len("txn");
        let now = date_time::now();

        let mut credit_amount = MinorUnit::new(0);
        let mut debit_amount = MinorUnit::new(0);

        for entry in &update.new_entries {
            match entry.balance_direction {
                BalanceDirection::Credit => {
                    credit_amount = MinorUnit::new(
                        credit_amount.get_amount_as_i64() + entry.amount.get_amount_as_i64(),
                    );
                }
                BalanceDirection::Debit => {
                    debit_amount = MinorUnit::new(
                        debit_amount.get_amount_as_i64() + entry.amount.get_amount_as_i64(),
                    );
                }
            }
        }

        // Find the minimum effective_at from all entries, fallback to current time
        let effective_at = update
            .new_entries
            .iter()
            .map(|entry| entry.effective_at)
            .min()
            .unwrap_or(now);

        Self {
            id,
            transaction_id: previous.transaction_id,
            entry_ids: update.new_entries.iter().map(|e| e.id.clone()).collect(),
            created_at: now,
            discarded_at: None,
            last_modified_at: now,
            discarded_status: None,
            version: previous.version + 1,
            profile_id: previous.profile_id,
            rule_id: previous.rule_id,
            credit_amount,
            debit_amount,
            currency: previous.currency,
            status: (DomainTransactionStatus::from(&update.data)).into(),
            data: update.data,
            discarded_data: None,
            effective_at,
            modified_by,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = transaction)]
pub struct TransactionUpdateInternal {
    pub status: DieselTransactionStatus,
    pub discarded_at: PrimitiveDateTime,
    pub discarded_status: DieselTransactionStatus,
    pub discarded_data: Option<TransactionData>,
    pub data: TransactionData,
    pub last_modified_at: PrimitiveDateTime,
}

/// Aggregate fields changed by a source-only append to an Expected transaction.
#[derive(Debug)]
pub struct ExpectedSourceAppendUpdateInternal {
    pub entry_ids: Vec<String>,
    pub credit_amount: MinorUnit,
    pub debit_amount: MinorUnit,
    pub effective_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

pub struct TransactionVersionUpdate {
    pub new_entries: Vec<Entry>,
    pub data: TransactionData,
}

/// Enum representing different types of transaction updates
#[derive(Debug, Clone)]
pub enum TransactionUpdate {
    /// Discard the transaction with specified data (status will be derived from data)
    DiscardUpdate { original_data: TransactionData },
}

impl From<TransactionUpdate> for TransactionUpdateInternal {
    fn from(transaction_update: TransactionUpdate) -> Self {
        let now = date_time::now();

        match transaction_update {
            TransactionUpdate::DiscardUpdate { original_data } => Self {
                status: DieselTransactionStatus::Archived,
                discarded_at: now,
                discarded_status: (DomainTransactionStatus::from(&original_data)).into(),
                discarded_data: Some(original_data),
                data: TransactionData::V3(TransactionDataV3::Archived),
                last_modified_at: now,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionOverviewRow {
    pub rule_id: String,
    pub status: DieselTransactionStatus,
    pub currency: Currency,
    pub total_count: i64,
    pub total_credit: MinorUnit,
    pub total_debit: MinorUnit,
    pub missing_count: i64,
    pub missing_credit: MinorUnit,
    pub missing_debit: MinorUnit,
}

#[derive(Debug, Clone)]
pub struct RuleAccountTransactionOverviewRow {
    pub rule_id: String,
    pub account_id: String,
    pub status: DieselTransactionStatus,
    pub currency: Currency,
    pub credit_count: i64,
    pub debit_count: i64,
    pub total_credit: MinorUnit,
    pub total_debit: MinorUnit,
    pub missing_credit_count: i64,
    pub missing_debit_count: i64,
    pub missing_credit: MinorUnit,
    pub missing_debit: MinorUnit,
}

#[derive(Debug, Clone)]
pub struct TransactionTimeBucketRow {
    pub bucket_start: PrimitiveDateTime,
    pub row: TransactionOverviewRow,
}

/// Combined view of transaction, entry, account, and recon rule data
/// Used for detailed transaction queries that need related data
#[derive(Debug, Clone)]
pub struct TransactionAccountView {
    pub transaction: Transaction,
    pub entry: Entry,
    pub account: Account,
    pub recon_rule: ReconRule,
}

#[derive(Debug, Clone)]
pub struct TransactionRuleView {
    pub transaction: Transaction,
    pub recon_rule: ReconRule,
}

#[derive(Debug, Clone)]
pub struct TransactionRuleViewPage {
    pub views: Vec<TransactionRuleView>,
    pub has_more: bool,
}
