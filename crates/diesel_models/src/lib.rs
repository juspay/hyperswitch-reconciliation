pub mod account;
pub mod api_key;
pub mod business_profile;
pub mod entry;
pub mod errors;
pub mod ingestion_config;
pub mod ingestion_history;
pub mod merchant_account;
pub mod metadata_schema;
pub mod query;
pub mod recon_rule;
pub mod recon_rule_account;
mod schema;
pub mod search_duplicate;
pub mod search_entry_key;
pub mod search_expectation;
pub mod search_group;
pub mod staging_entry;
pub mod transaction;
pub mod transformation_config;
pub mod transformation_history;

pub type PgPooledConn = async_bb8_diesel::Connection<diesel::PgConnection>;
pub type StorageResult<T> = error_stack::Result<T, errors::DatabaseError>;

pub(crate) mod diesel_impl {
    use diesel::{
        deserialize::FromSql,
        pg::Pg,
        sql_types::{Array, Nullable},
        Queryable,
    };

    pub struct DieselArray<T>(Vec<Option<T>>);

    impl<T> From<DieselArray<T>> for Vec<T> {
        fn from(array: DieselArray<T>) -> Self {
            array.0.into_iter().flatten().collect()
        }
    }

    impl<T, U> Queryable<Array<Nullable<U>>, Pg> for DieselArray<T>
    where
        T: FromSql<U, Pg>,
        Vec<Option<T>>: FromSql<Array<Nullable<U>>, Pg>,
    {
        type Row = Vec<Option<T>>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            Ok(Self(row))
        }
    }

    pub struct OptionalDieselArray<T>(Option<Vec<Option<T>>>);

    impl<T> From<OptionalDieselArray<T>> for Option<Vec<T>> {
        fn from(option_array: OptionalDieselArray<T>) -> Self {
            option_array
                .0
                .map(|array| array.into_iter().flatten().collect())
        }
    }

    impl<T, U> Queryable<Nullable<Array<Nullable<U>>>, Pg> for OptionalDieselArray<T>
    where
        Option<Vec<Option<T>>>: FromSql<Nullable<Array<Nullable<U>>>, Pg>,
    {
        type Row = Option<Vec<Option<T>>>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            Ok(Self(row))
        }
    }
}
