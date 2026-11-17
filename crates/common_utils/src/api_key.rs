use error_stack::Report;
use masking::{PeekInterface, Secret, StrongSecret};

use crate::{
    crypto::generate_cryptographically_secure_random_string,
    errors::{CryptoError, CustomResult},
};

pub const API_KEY_RANDOM_LENGTH: usize = 64;
pub const API_KEY_DISPLAY_RANDOM_LENGTH: usize = 6;
pub const API_KEY_HASH_KEY_LENGTH: usize = 32;

/// Server-wide key for the keyed hash. There is no rotation: changing it invalidates every key.
#[derive(Debug, Clone)]
pub struct ApiKeyHashKey(StrongSecret<[u8; API_KEY_HASH_KEY_LENGTH]>);

impl ApiKeyHashKey {
    pub fn from_hex(hex_key: &str) -> CustomResult<Self, CryptoError> {
        let mut key = [0u8; API_KEY_HASH_KEY_LENGTH];
        hex::decode_to_slice(hex_key.trim(), &mut key).map_err(|e| {
            Report::new(match e {
                hex::FromHexError::InvalidStringLength => CryptoError::InvalidKeyLength,
                _ => CryptoError::DecodingFailed,
            })
        })?;

        Ok(Self(StrongSecret::new(key)))
    }
}

#[derive(Debug, Clone)]
pub struct PlaintextApiKey(StrongSecret<String>);

impl PlaintextApiKey {
    /// `env_prefix` must not contain `_`, which `display_prefix` splits on.
    pub fn generate(env_prefix: &str) -> Self {
        let random = generate_cryptographically_secure_random_string(API_KEY_RANDOM_LENGTH);

        Self(StrongSecret::new(format!("{env_prefix}_{random}")))
    }

    pub fn from_request(value: &str) -> Self {
        Self(StrongSecret::new(value.trim().to_string()))
    }

    pub fn display_prefix(&self) -> String {
        let key = self.0.peek();
        // A request key may not have a `_`: show nothing of it rather than panic.
        key.split_once('_')
            .map(|(env, random)| {
                let shown: String = random.chars().take(API_KEY_DISPLAY_RANDOM_LENGTH).collect();
                format!("{env}_{shown}")
            })
            .unwrap_or_default()
    }

    pub fn hash(&self, hash_key: &ApiKeyHashKey) -> HashedApiKey {
        HashedApiKey(Secret::new(
            blake3::keyed_hash(hash_key.0.peek(), self.0.peek().as_bytes())
                .to_hex()
                .to_string(),
        ))
    }

    pub fn expose(&self) -> &str {
        self.0.peek()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashedApiKey(Secret<String>);

impl HashedApiKey {
    pub fn peek(&self) -> &str {
        self.0.peek()
    }
}

impl From<HashedApiKey> for Secret<String> {
    fn from(hashed: HashedApiKey) -> Self {
        hashed.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const KEY_A: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const KEY_B: &str = "ff02030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f00";

    fn is_valid_format(key: &str, env: &str) -> bool {
        let prefix = format!("{env}_");
        key.strip_prefix(&prefix).is_some_and(|random| {
            random.len() == API_KEY_RANDOM_LENGTH
                && random.chars().all(|c| c.is_ascii_alphanumeric())
        })
    }

    #[test]
    fn generated_key_matches_format() {
        for name in ["dev", "sbx", "prd"] {
            let key = PlaintextApiKey::generate(name);
            assert!(is_valid_format(key.expose(), name), "{}", key.expose());
        }
    }

    #[test]
    fn generated_keys_are_unique() {
        let keys: HashSet<_> = (0..1000)
            .map(|_| PlaintextApiKey::generate("dev").expose().to_string())
            .collect();
        assert_eq!(keys.len(), 1000);
    }

    #[test]
    fn display_prefix_is_env_plus_six_random_chars() {
        let key = PlaintextApiKey::generate("sbx");
        let prefix = key.display_prefix();
        assert_eq!(prefix.len(), "sbx_".len() + API_KEY_DISPLAY_RANDOM_LENGTH);
        assert!(prefix.starts_with("sbx_"));
        assert!(key.expose().starts_with(&prefix));
        assert_eq!(
            PlaintextApiKey::from_request("no-separator").display_prefix(),
            ""
        );
    }

    #[test]
    fn hash_is_deterministic_and_hex() {
        let hash_key = ApiKeyHashKey::from_hex(KEY_A).unwrap();
        let key = PlaintextApiKey::generate("dev");

        let first = key.hash(&hash_key);
        assert_eq!(first, key.hash(&hash_key));
        assert_eq!(first.peek().len(), 64);
        assert!(first.peek().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn request_key_hashes_like_the_generated_one() {
        let hash_key = ApiKeyHashKey::from_hex(KEY_A).unwrap();
        let key = PlaintextApiKey::generate("dev");
        let from_request = PlaintextApiKey::from_request(&format!("  {}\n", key.expose()));

        assert_eq!(from_request.hash(&hash_key), key.hash(&hash_key));
    }

    #[test]
    fn different_hash_key_or_input_gives_different_hash() {
        let key_a = ApiKeyHashKey::from_hex(KEY_A).unwrap();
        let key_b = ApiKeyHashKey::from_hex(KEY_B).unwrap();
        let key = PlaintextApiKey::generate("dev");

        assert_ne!(key.hash(&key_a), key.hash(&key_b));
        assert_ne!(
            key.hash(&key_a),
            PlaintextApiKey::generate("dev").hash(&key_a)
        );
    }

    #[test]
    fn hash_key_parsing_rejects_bad_input() {
        let context = |input: &str| {
            ApiKeyHashKey::from_hex(input)
                .unwrap_err()
                .current_context()
                .to_string()
        };

        assert_eq!(context("abcd"), CryptoError::InvalidKeyLength.to_string());
        assert_eq!(context(""), CryptoError::InvalidKeyLength.to_string());
        assert_eq!(
            context(&"zz".repeat(32)),
            CryptoError::DecodingFailed.to_string()
        );
        assert!(ApiKeyHashKey::from_hex(&format!(" {KEY_A}\n")).is_ok());
    }

    #[test]
    fn debug_never_reveals_secrets() {
        let key = PlaintextApiKey::generate("dev");
        let hash_key = ApiKeyHashKey::from_hex(KEY_A).unwrap();
        let hashed = key.hash(&hash_key);

        assert!(!format!("{key:?}").contains(key.expose()));
        assert!(!format!("{hashed:?}").contains(hashed.peek()));
        assert!(!format!("{hash_key:?}").contains("0001"));
        let error = ApiKeyHashKey::from_hex(&"zq".repeat(32)).unwrap_err();
        assert!(!format!("{error} {error:?}").contains("zq"));
        assert!(!format!("{error:?}").contains("position"));
    }
}
