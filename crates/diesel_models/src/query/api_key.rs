use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::ApiKeyStatus;
use common_utils::{id_type::MerchantId, logging};
use diesel::{
    debug_query, pg::Pg, prelude::*, result::Error as DieselError, BoolExpressionMethods,
    ExpressionMethods,
};
use error_stack::{report, ResultExt};

use crate::{
    api_key::{ApiKey, ApiKeyNew, ApiKeyUpdateInternal},
    errors,
    query::generics,
    schema::api_key::{self, dsl},
    PgPooledConn, StorageResult,
};

impl ApiKeyNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ApiKey> {
        generics::generic_insert(conn, self).await
    }
}

impl ApiKey {
    /// Returns `None` for an unknown or revoked key. Expiry is not checked here.
    pub async fn find_active_by_hashed_api_key(
        conn: &PgPooledConn,
        hashed_api_key: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<api_key::table, _, _>(
            conn,
            dsl::hashed_api_key
                .eq(hashed_api_key.to_owned())
                .and(dsl::status.eq(ApiKeyStatus::Active)),
        )
        .await
    }

    pub async fn find_active_by_merchant_id_and_api_key_id(
        conn: &PgPooledConn,
        merchant_id: &MerchantId,
        api_key_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<api_key::table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::id.eq(api_key_id.to_owned()))
                .and(dsl::status.eq(ApiKeyStatus::Active)),
        )
        .await
    }

    pub async fn list_active_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &MerchantId,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::api_key
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .filter(dsl::status.eq(ApiKeyStatus::Active))
            .order(dsl::created_at.desc());

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        match generics::db_metrics::track_database_call::<api_key::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                DieselError::NotFound => {
                    Err(report!(err)).change_context(errors::DatabaseError::NotFound)
                }
                _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
            },
        }
        .attach_printable_lazy(|| {
            format!(
                "Error while listing API keys for merchant_id: {}",
                merchant_id.get_string_repr()
            )
        })
    }

    /// Only an active key can be updated, so a revoked key can never be modified or revived.
    pub async fn update_active_by_merchant_id_and_api_key_id(
        conn: &PgPooledConn,
        merchant_id: &MerchantId,
        api_key_id: &str,
        api_key_update: ApiKeyUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<api_key::table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::id.eq(api_key_id.to_owned()))
                .and(dsl::status.eq(ApiKeyStatus::Active)),
            api_key_update,
        )
        .await
    }
}
