use diesel::ExpressionMethods;

use crate::{
    merchant_account::{MerchantAccount, MerchantAccountNew},
    query::generics,
    schema::{merchant_account, merchant_account::dsl},
    PgPooledConn, StorageResult,
};

impl MerchantAccount {
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<merchant_account::table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }
}

impl MerchantAccountNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantAccount> {
        generics::generic_insert(conn, self).await
    }
}
