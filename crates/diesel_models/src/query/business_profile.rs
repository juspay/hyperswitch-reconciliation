use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::id_type::MerchantId;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*, result::Error as DieselError};
use error_stack::{report, ResultExt};

use crate::{
    business_profile::{BusinessProfile, BusinessProfileNew},
    errors,
    query::generics,
    schema::business_profile::{self, dsl},
    PgPooledConn, StorageResult,
};

impl BusinessProfile {
    pub async fn find_by_profile_id(conn: &PgPooledConn, profile_id: &str) -> StorageResult<Self> {
        generics::generic_find_by_id::<business_profile::table, _, _>(conn, profile_id.to_owned())
            .await
    }

    pub async fn list_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: MerchantId,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::business_profile.filter(dsl::merchant_id.eq(merchant_id.to_owned()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        match generics::db_metrics::track_database_call::<business_profile::table, _, _>(
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
        .attach_printable_lazy(move || {
            format!(
                "Error while listing business profiles for merchant_id: {}",
                merchant_id.get_string_repr()
            )
        })
    }
}

impl BusinessProfileNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BusinessProfile> {
        generics::generic_insert(conn, self).await
    }
}
