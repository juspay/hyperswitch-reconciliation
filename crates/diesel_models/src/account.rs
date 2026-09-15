use crate::schema::account;
use common_enums::{BalanceDirection, Currency};
use common_types::units::MinorUnit;
use common_utils::date_time;
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = account)]
pub struct Account {
    pub id: String,
    pub balance_direction: BalanceDirection,
    pub name: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub initial_balance: MinorUnit,
    pub profile_id: String,
    pub currency: Currency,
    pub matched_debits: MinorUnit,
    pub matched_credits: MinorUnit,
    pub pending_debits: MinorUnit,
    pub pending_credits: MinorUnit,
    pub expected_debits: MinorUnit,
    pub expected_credits: MinorUnit,
    pub mismatched_debits: MinorUnit,
    pub mismatched_credits: MinorUnit,
    pub posted_debits: MinorUnit,
    pub posted_credits: MinorUnit,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = account)]
pub struct AccountNew {
    pub id: String,
    pub balance_direction: BalanceDirection,
    pub name: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub initial_balance: MinorUnit,
    pub profile_id: String,
    pub currency: Currency,
    pub matched_debits: MinorUnit,
    pub matched_credits: MinorUnit,
    pub pending_debits: MinorUnit,
    pub pending_credits: MinorUnit,
    pub expected_debits: MinorUnit,
    pub expected_credits: MinorUnit,
    pub mismatched_debits: MinorUnit,
    pub mismatched_credits: MinorUnit,
    pub posted_debits: MinorUnit,
    pub posted_credits: MinorUnit,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = account)]
pub struct AccountUpdateInternal {
    pub name: Option<String>,
    pub last_modified_at: PrimitiveDateTime,
}

/// Enum representing different types of account updates
#[derive(Debug, Clone)]
pub enum AccountUpdate {
    /// Update only the account name
    NameUpdate { name: String },
}

impl From<AccountUpdate> for AccountUpdateInternal {
    fn from(account_update: AccountUpdate) -> Self {
        let last_modified_at = date_time::now();

        match account_update {
            AccountUpdate::NameUpdate { name } => Self {
                name: Some(name),
                last_modified_at,
            },
        }
    }
}
