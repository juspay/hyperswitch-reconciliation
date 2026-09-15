use base64::{engine::general_purpose::STANDARD as BASE64_ENGINE, Engine};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, Queryable},
    expression::AsExpression,
    serialize::ToSql,
    sql_types,
};
use masking::{ExposeInterface, Secret};

use crate::{crypto::Encryptable, pii::EncryptionStrategy};

/// Version prefix for local encryption to match keymanager format
const LOCAL_ENCRYPTION_VERSION: &str = "v1";

impl<DB> FromSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStrategy>: FromSql<sql_types::Binary, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        <Secret<Vec<u8>, EncryptionStrategy>>::from_sql(bytes).map(Self::new)
    }
}

impl<DB> ToSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStrategy>: ToSql<sql_types::Binary, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.get_inner().to_sql(out)
    }
}

impl<DB> Queryable<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStrategy>: FromSql<sql_types::Binary, DB>,
{
    type Row = Secret<Vec<u8>, EncryptionStrategy>;
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(Self { inner: row })
    }
}

#[derive(Debug, AsExpression, Clone, Eq, PartialEq)]
#[diesel(sql_type = sql_types::Binary)]
#[repr(transparent)]
pub struct Encryption {
    inner: Secret<Vec<u8>, EncryptionStrategy>,
}

impl<T: Clone> From<Encryptable<T>> for Encryption {
    fn from(value: Encryptable<T>) -> Self {
        Self::new(value.into_encrypted())
    }
}

impl serde::Serialize for Encryption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Store the exposed value in a binding to extend its lifetime
        let exposed_data = self.inner.clone().expose();
        let data_str = std::str::from_utf8(&exposed_data).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(data_str)
    }
}

impl<'de> serde::Deserialize<'de> for Encryption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data_str = String::deserialize(deserializer)?;
        Ok(Self {
            inner: Secret::new(data_str.into_bytes()),
        })
    }
}

impl Encryption {
    pub fn new(item: Secret<Vec<u8>, EncryptionStrategy>) -> Self {
        Self { inner: item }
    }

    /// Create an Encryption from raw bytes (for local encryption)
    /// Formats the data as "v1:base64-encoded-data"
    pub fn from_raw_bytes(raw_bytes: Vec<u8>) -> Self {
        let base64_encoded = BASE64_ENGINE.encode(&raw_bytes);
        let formatted = format!("{LOCAL_ENCRYPTION_VERSION}:{base64_encoded}");
        Self {
            inner: Secret::new(formatted.into_bytes()),
        }
    }

    /// Extract raw bytes from the encryption data
    /// Handles both versioned format ("v1:base64-data") and raw bytes
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>, String> {
        let data = self.inner.clone().expose();

        // Try to parse as string first (versioned format)
        if let Ok(data_str) = std::str::from_utf8(&data) {
            if let Some((version, base64_data)) = data_str.split_once(':') {
                if version == LOCAL_ENCRYPTION_VERSION {
                    // Decode base64 data
                    return BASE64_ENGINE
                        .decode(base64_data)
                        .map_err(|e| format!("Failed to decode base64: {e}"));
                }
            }
        }

        // If not versioned format, return raw bytes as-is
        Ok(data)
    }

    #[inline]
    pub fn into_inner(self) -> Secret<Vec<u8>, EncryptionStrategy> {
        self.inner
    }

    #[inline]
    pub fn get_inner(&self) -> &Secret<Vec<u8>, EncryptionStrategy> {
        &self.inner
    }
}
