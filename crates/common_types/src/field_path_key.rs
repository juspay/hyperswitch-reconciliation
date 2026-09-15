use core::fmt;
use std::{convert::Infallible, str::FromStr};

use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Text};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Hash, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
#[schema(value_type = String)]
pub struct FieldPathKey(String);

impl FieldPathKey {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for FieldPathKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for FieldPathKey {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl AsRef<str> for FieldPathKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for FieldPathKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for FieldPathKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

common_utils::impl_to_sql_from_sql_text!(FieldPathKey);
