//! Contains the id type for merchant account

use crate::{
    errors::{CustomResult, ValidationError},
    id_type::{AlphaNumericId, LengthId},
};

crate::id_type!(
    MerchantId,
    "A type for merchant_id that can be used for merchant ids"
);
crate::impl_id_type_methods!(MerchantId, "merchant_id");

// This is to display the `MerchantId` as MerchantId(abcd)
crate::impl_debug_id_type!(MerchantId);
crate::impl_default_id_type!(MerchantId, "merchant");
crate::impl_try_from_cow_str_id_type!(MerchantId, "merchant_id");

crate::impl_generate_id_id_type!(MerchantId, "merchant");
crate::impl_serializable_secret_id_type!(MerchantId);
crate::impl_queryable_id_type!(MerchantId);
crate::impl_to_sql_from_sql_id_type!(MerchantId);

impl MerchantId {
    /// Create a MerchantId from String
    pub fn wrap(merchant_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(merchant_id))
    }

    /// Create a new MerchantId without any validations
    pub fn new_unchecked(merchant_id: String) -> Self {
        let alphanumeric_id = AlphaNumericId::new_unchecked(merchant_id);
        let length_id = LengthId::<64, 1>::new_unchecked(alphanumeric_id);
        Self(length_id)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::id_type::GenerateId;

    // Test 1: Serde roundtrip - JSON is plain string, not wrapped object
    #[test]
    fn test_serde_roundtrip() {
        let id = MerchantId::wrap("merchant_abc123".to_string()).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"merchant_abc123\""); // plain string, not {"MerchantId":"..."}
        let deserialized: MerchantId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    // Test 2: generate() has correct prefix
    #[test]
    fn test_generate_prefix() {
        let id = MerchantId::generate();
        assert!(id.get_string_repr().starts_with("merchant_"));
    }

    // Test 3: generate() produces unique IDs
    #[test]
    fn test_generate_uniqueness() {
        let id1 = MerchantId::generate();
        let id2 = MerchantId::generate();
        assert_ne!(id1, id2);
    }

    // Test 4: wrap() with valid ID succeeds
    #[test]
    fn test_wrap_valid() {
        let result = MerchantId::wrap("valid_merchant_123".to_string());
        assert!(result.is_ok());
    }

    // Test 5: wrap() with empty string fails (MIN_LENGTH=1)
    #[test]
    fn test_wrap_empty() {
        let result = MerchantId::wrap("".to_string());
        assert!(result.is_err());
    }

    // Test 6: wrap() with too long string fails (MAX_LENGTH=64)
    #[test]
    fn test_wrap_too_long() {
        let long_id = "a".repeat(65);
        let result = MerchantId::wrap(long_id);
        assert!(result.is_err());
    }

    // Test 7: wrap() with special characters fails (alphanumeric only)
    #[test]
    fn test_wrap_special_chars() {
        let result = MerchantId::wrap("invalid!@#".to_string());
        assert!(result.is_err());
    }

    // Test 8: wrap() with underscore and hyphen succeeds
    #[test]
    fn test_wrap_underscore_hyphen() {
        let result = MerchantId::wrap("valid-id_123".to_string());
        assert!(result.is_ok());
    }

    // Test 9: get_string_repr() returns original value
    #[test]
    fn test_get_string_repr() {
        let original = "test_merchant";
        let id = MerchantId::wrap(original.to_string()).unwrap();
        assert_eq!(id.get_string_repr(), original);
    }

    // Test 10: PartialEq works correctly
    #[test]
    fn test_partial_eq() {
        let id1 = MerchantId::wrap("same_id".to_string()).unwrap();
        let id2 = MerchantId::wrap("same_id".to_string()).unwrap();
        assert_eq!(id1, id2);
    }

    // Test 11: Clone works correctly
    #[test]
    fn test_clone() {
        let id = MerchantId::wrap("clone_test".to_string()).unwrap();
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    // Test 12: Debug format is MerchantId("value")
    #[test]
    fn test_debug_format() {
        let id = MerchantId::wrap("debug_test".to_string()).unwrap();
        let debug_str = format!("{:?}", id);
        assert!(debug_str.starts_with("MerchantId("));
        assert!(debug_str.contains("debug_test"));
    }
}
