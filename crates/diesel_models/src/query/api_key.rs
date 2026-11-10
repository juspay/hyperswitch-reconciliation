use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::ApiKeyStatus;
use common_utils::{id_type::MerchantId, logging};
use diesel::{debug_query, pg::Pg, prelude::*, BoolExpressionMethods, ExpressionMethods};
use error_stack::ResultExt;

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

    pub async fn update_active_by_merchant_id_and_api_key_id(
        conn: &PgPooledConn,
        merchant_id: &MerchantId,
        api_key_id: &str,
        update: ApiKeyUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<api_key::table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::id.eq(api_key_id.to_owned()))
                .and(dsl::status.eq(ApiKeyStatus::Active)),
            update,
        )
        .await
        .attach_printable_lazy(|| format!("Error updating API key: id={api_key_id}"))
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

        generics::db_metrics::track_database_call::<api_key::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|err| error_stack::Report::new(err))
        .change_context(errors::DatabaseError::Others)
        .attach_with(|| {
            format!(
                "Error while listing API keys for merchant_id: {}",
                merchant_id.get_string_repr()
            )
        })
    }
}
