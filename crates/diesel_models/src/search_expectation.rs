use common_enums::ExpectationStatus;
use common_types::field_path_key::FieldPathKey;
use common_utils::{
    date_time, encryption::Encryption, id_generation::generate_id_with_default_len,
    id_type::MerchantId,
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::search_expectation;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = search_expectation)]
#[diesel(primary_key(id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchExpectation {
    pub id: String,
    pub rule_id: String,
    pub target_account_id: String,
    pub target_key: FieldPathKey,
    pub status: ExpectationStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub encrypted_source_value: Encryption,
    pub hashed_source_value: Secret<String>,
    pub merchant_id: MerchantId,
    pub primary_transaction_id: String,
}

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = search_expectation)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchExpectationNew {
    pub id: String,
    pub rule_id: String,
    pub target_account_id: String,
    pub target_key: FieldPathKey,
    pub status: ExpectationStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub encrypted_source_value: Encryption,
    pub hashed_source_value: Secret<String>,
    pub merchant_id: MerchantId,
    pub primary_transaction_id: String,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = search_expectation)]
pub struct SearchExpectationUpdateInternal {
    pub status: ExpectationStatus,
    pub last_modified_at: PrimitiveDateTime,
}

/// Enum representing different types of search expectation updates
#[derive(Debug, Clone)]
pub enum SearchExpectationUpdate {
    /// Update only the status
    StatusUpdate { status: ExpectationStatus },
}

impl From<SearchExpectationUpdate> for SearchExpectationUpdateInternal {
    fn from(search_expectation_update: SearchExpectationUpdate) -> Self {
        let last_modified_at = date_time::now();

        match search_expectation_update {
            SearchExpectationUpdate::StatusUpdate { status } => Self {
                status,
                last_modified_at,
            },
        }
    }
}

impl SearchExpectationNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rule_id: String,
        target_account_id: String,
        encrypted_source_value: Encryption,
        hashed_source_value: Secret<String>,
        target_key: FieldPathKey,
        status: ExpectationStatus,
        merchant_id: MerchantId,
        primary_transaction_id: String,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("sea_exp");
        Self {
            id,
            rule_id,
            target_account_id,
            target_key,
            status,
            created_at: now,
            last_modified_at: now,
            encrypted_source_value,
            hashed_source_value,
            merchant_id,
            primary_transaction_id,
        }
    }
}
