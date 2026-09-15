/// Implements the `ToSql` and `FromSql` traits on a type to allow it to be serialized/deserialized
/// to/from JSON data in the database.
#[macro_export]
macro_rules! impl_to_sql_from_sql_json {
    ($type:ty, $diesel_type:ty) => {
        #[allow(unused_qualifications)]
        impl diesel::serialize::ToSql<$diesel_type, diesel::pg::Pg> for $type {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                let value = serde_json::to_value(self)?;

                // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
                // please refer to the diesel migration blog:
                // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
                <serde_json::Value as diesel::serialize::ToSql<
                                                                $diesel_type,
                                                                diesel::pg::Pg,
                                                            >>::to_sql(&value, &mut out.reborrow())
            }
        }

        #[allow(unused_qualifications)]
        impl diesel::deserialize::FromSql<$diesel_type, diesel::pg::Pg> for $type {
            fn from_sql(
                bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                let value = <serde_json::Value as diesel::deserialize::FromSql<
                    $diesel_type,
                    diesel::pg::Pg,
                >>::from_sql(bytes)?;
                Ok(serde_json::from_value(value)?)
            }
        }
    };
    ($type: ty) => {
        $crate::impl_to_sql_from_sql_json!($type, diesel::sql_types::Json);
        $crate::impl_to_sql_from_sql_json!($type, diesel::sql_types::Jsonb);
    };
}

/// Implements the `ToSql` and `FromSql` traits on a type to allow it to be serialized/deserialized
/// to/from `TEXT` data in the database. The type must implement `Display` and `FromStr`,
/// whose `Err` is `Send + Sync + std::error::Error + 'static`.
#[macro_export]
macro_rules! impl_to_sql_from_sql_text {
    ($type:ty) => {
        #[allow(unused_qualifications)]
        impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::pg::Pg> for $type {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                use std::io::Write;
                out.write_all(self.to_string().as_bytes())?;
                Ok(diesel::serialize::IsNull::No)
            }
        }

        #[allow(unused_qualifications)]
        impl diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::pg::Pg> for $type {
            fn from_sql(
                bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                let s = std::str::from_utf8(bytes.as_bytes())?;
                <Self as std::str::FromStr>::from_str(s).map_err(Into::into)
            }
        }
    };
}

mod id_type {
    /// Defines an ID type.
    #[macro_export]
    macro_rules! id_type {
        ($type:ident, $doc:literal, $diesel_type:ty, $max_length:expr, $min_length:expr) => {
            #[doc = $doc]
            #[derive(
                Clone,
                Hash,
                PartialEq,
                Eq,
                serde::Serialize,
                serde::Deserialize,
                diesel::expression::AsExpression,
                utoipa::ToSchema,
            )]
            #[diesel(sql_type = $diesel_type)]
            #[schema(value_type = String)]
            pub struct $type($crate::id_type::LengthId<$max_length, $min_length>);
        };
        ($type:ident, $doc:literal) => {
            $crate::id_type!(
                $type,
                $doc,
                diesel::sql_types::Text,
                { $crate::consts::MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH },
                { $crate::consts::MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH }
            );
        };
    }

    /// Implements common methods on the specified ID type.
    #[macro_export]
    macro_rules! impl_id_type_methods {
        ($type:ty, $field_name:literal) => {
            impl $type {
                /// Get the string representation of the ID type.
                pub fn get_string_repr(&self) -> &str {
                    &self.0 .0 .0
                }
            }
        };
    }

    /// Implements the `Debug` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_debug_id_type {
        ($type:ty) => {
            impl core::fmt::Debug for $type {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_tuple(stringify!($type))
                        .field(&self.0 .0 .0)
                        .finish()
                }
            }
        };
    }

    /// Implements the `TryFrom<Cow<'static, str>>` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_try_from_cow_str_id_type {
        ($type:ty, $field_name:literal) => {
            impl TryFrom<std::borrow::Cow<'static, str>> for $type {
                type Error = error_stack::Report<$crate::errors::ValidationError>;

                fn try_from(value: std::borrow::Cow<'static, str>) -> Result<Self, Self::Error> {
                    use error_stack::ResultExt;

                    let merchant_ref_id = $crate::id_type::LengthId::from(value).change_context(
                        $crate::errors::ValidationError::IncorrectValueProvided {
                            field_name: $field_name,
                        },
                    )?;

                    Ok(Self(merchant_ref_id))
                }
            }
        };
    }

    /// Implements the `Default` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_default_id_type {
        ($type:ty, $prefix:literal) => {
            impl Default for $type {
                fn default() -> Self {
                    Self($crate::generate_ref_id_with_default_length($prefix))
                }
            }
        };
    }

    /// Implements the `GenerateId` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_generate_id_id_type {
        ($type:ty, $prefix:literal) => {
            impl $crate::id_type::GenerateId for $type {
                fn generate() -> Self {
                    Self($crate::generate_ref_id_with_default_length($prefix))
                }
            }
        };
    }

    /// Implements the `SerializableSecret` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_serializable_secret_id_type {
        ($type:ty) => {
            impl masking::SerializableSecret for $type {}
        };
    }

    /// Implements the `ToSql` and `FromSql` traits on the specified ID type.
    #[macro_export]
    macro_rules! impl_to_sql_from_sql_id_type {
        ($type:ty, $diesel_type:ty, $max_length:expr, $min_length:expr) => {
            impl<DB> diesel::serialize::ToSql<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                $crate::id_type::LengthId<$max_length, $min_length>:
                    diesel::serialize::ToSql<$diesel_type, DB>,
            {
                fn to_sql<'b>(
                    &'b self,
                    out: &mut diesel::serialize::Output<'b, '_, DB>,
                ) -> diesel::serialize::Result {
                    self.0.to_sql(out)
                }
            }

            impl<DB> diesel::deserialize::FromSql<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                $crate::id_type::LengthId<$max_length, $min_length>:
                    diesel::deserialize::FromSql<$diesel_type, DB>,
            {
                fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
                    $crate::id_type::LengthId::<$max_length, $min_length>::from_sql(value).map(Self)
                }
            }
        };
        ($type:ty) => {
            $crate::impl_to_sql_from_sql_id_type!(
                $type,
                diesel::sql_types::Text,
                { $crate::consts::MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH },
                { $crate::consts::MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH }
            );
        };
    }

    /// Implements the `Queryable` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_queryable_id_type {
        ($type:ty, $diesel_type:ty) => {
            impl<DB> diesel::Queryable<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                Self: diesel::deserialize::FromSql<$diesel_type, DB>,
            {
                type Row = Self;

                fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
                    Ok(row)
                }
            }
        };
        ($type:ty) => {
            $crate::impl_queryable_id_type!($type, diesel::sql_types::Text);
        };
    }
}
