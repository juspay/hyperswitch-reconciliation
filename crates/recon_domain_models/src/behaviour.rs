use crate::{encryption::ApplicationEncryptionInterface, EncryptionHandler};
use common_utils::errors::{CustomResult, ValidationError};

/// Trait for converting domain types to storage models
#[async_trait::async_trait]
pub trait Conversion {
    type DstType;
    type NewDstType;

    /// Convert diesel model back to domain model (with decryption)
    async fn convert_back(
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized;

    /// Convert domain model to diesel New model for insertion (with encryption)
    fn construct_new(self) -> Self::NewDstType;

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType;
}

#[async_trait::async_trait]
pub trait ReverseConversion<SrcType: Conversion> {
    async fn convert_back(
        self,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> CustomResult<SrcType, ValidationError>;
}

#[async_trait::async_trait]
impl<T: Send, U: Conversion<DstType = T>> ReverseConversion<U> for T {
    async fn convert_back(
        self,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> CustomResult<U, ValidationError> {
        U::convert_back(encryptor, self).await
    }
}
