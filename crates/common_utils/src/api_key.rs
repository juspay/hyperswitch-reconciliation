//! Generation and hashing of merchant API keys.
//!
//! A key looks like `{env}_{64 base62 chars}`, where `env` is configured per deployment
//! (`dev`, `sbx`, `prd`). Only a keyed BLAKE3 hash of it is ever stored. The plaintext is shown
//! once, at creation.

use masking::{PeekInterface, Secret};
use rand::{distributions::Alphanumeric, Rng};

/// Random characters after `{env}_`. 64 base62 characters is about 380 bits of entropy.
pub const API_KEY_RANDOM_LENGTH: usize = 64;
/// Random characters kept for display, after `{env}_` (`prd_` + 6).
pub const API_KEY_DISPLAY_RANDOM_LENGTH: usize = 6;

/// Length of the hash key in bytes. It is configured as 64 hex characters.
pub const API_KEY_HASH_KEY_LENGTH: usize = 32;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ApiKeyHashKeyError {
    #[error("API key hash key must be {} hex characters", API_KEY_HASH_KEY_LENGTH * 2)]
    InvalidLength,
    #[error("API key hash key must be hex encoded")]
    InvalidHex,
}

/// Server-wide key for the keyed hash. There is no rotation: changing it invalidates every key.
#[derive(Clone)]
pub struct ApiKeyHashKey([u8; API_KEY_HASH_KEY_LENGTH]);

impl ApiKeyHashKey {
    /// Parses the configured value (64 hex characters, surrounding whitespace ignored).
    ///
    /// The error never contains the input.
    pub fn from_hex(hex_key: &str) -> Result<Self, ApiKeyHashKeyError> {
        let bytes = hex::decode(hex_key.trim()).map_err(|_| ApiKeyHashKeyError::InvalidHex)?;
        let key = <[u8; API_KEY_HASH_KEY_LENGTH]>::try_from(bytes)
            .map_err(|_| ApiKeyHashKeyError::InvalidLength)?;

        Ok(Self(key))
    }
}

impl std::fmt::Debug for ApiKeyHashKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiKeyHashKey(*** masked ***)")
    }
}

/// The plaintext key. Deliberately has no `Display`, and `Debug` is masked by `Secret`.
#[derive(Debug, Clone)]
pub struct PlaintextApiKey(Secret<String>);

impl PlaintextApiKey {
    /// Generates a key for the given environment prefix (`dev`, `sbx` or `prd`, see
    /// `recon::env::prefix_for_env`). It must not contain `_`, which ends it.
    ///
    /// Uses `thread_rng`, a ChaCha-based CSPRNG seeded from the OS.
    pub fn generate(env_prefix: &str) -> Self {
        let random: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(API_KEY_RANDOM_LENGTH)
            .map(char::from)
            .collect();

        Self(Secret::new(format!("{env_prefix}_{random}")))
    }

    /// Wraps a key received in a request. The value is trimmed.
    pub fn from_request(value: &str) -> Self {
        Self(Secret::new(value.trim().to_string()))
    }

    /// Environment prefix plus the first few random characters, safe to store and show.
    pub fn display_prefix(&self) -> String {
        let key = self.0.peek();
        // The env prefix has no `_`, so the first one ends it. A request key without one is
        // never generated here, but must not panic: show nothing of it.
        key.split_once('_')
            .map(|(env, random)| {
                let shown: String = random.chars().take(API_KEY_DISPLAY_RANDOM_LENGTH).collect();
                format!("{env}_{shown}")
            })
            .unwrap_or_default()
    }

    /// The only hashing entry point, so creation and authentication can't disagree.
    pub fn hash(&self, hash_key: &ApiKeyHashKey) -> HashedApiKey {
        HashedApiKey(
            blake3::keyed_hash(&hash_key.0, self.0.peek().as_bytes())
                .to_hex()
                .to_string(),
        )
    }

    /// For the one-time creation response only.
    pub fn expose(&self) -> &str {
        self.0.peek()
    }
}

/// Hex encoded keyed BLAKE3 hash of a key (64 characters). This is what the database stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashedApiKey(String);

impl HashedApiKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<HashedApiKey> for String {
    fn from(hashed: HashedApiKey) -> Self {
        hashed.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
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
        let keys: std::collections::HashSet<_> = (0..1000)
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
        assert_eq!(first.as_str().len(), 64);
        assert!(first.as_str().chars().all(|c| c.is_ascii_hexdigit()));
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
        assert_eq!(
            ApiKeyHashKey::from_hex("abcd").unwrap_err(),
            ApiKeyHashKeyError::InvalidLength
        );
        assert_eq!(
            ApiKeyHashKey::from_hex(&"zz".repeat(32)).unwrap_err(),
            ApiKeyHashKeyError::InvalidHex
        );
        assert_eq!(
            ApiKeyHashKey::from_hex("").unwrap_err(),
            ApiKeyHashKeyError::InvalidLength
        );
        assert!(ApiKeyHashKey::from_hex(&format!(" {KEY_A}\n")).is_ok());
    }

    #[test]
    fn debug_never_reveals_secrets() {
        let key = PlaintextApiKey::generate("dev");
        let hash_key = ApiKeyHashKey::from_hex(KEY_A).unwrap();

        assert!(!format!("{key:?}").contains(key.expose()));
        assert!(!format!("{hash_key:?}").contains("0001"));
        // The error carries no input either.
        let error = ApiKeyHashKey::from_hex("not hex at all").unwrap_err();
        assert!(!format!("{error} {error:?}").contains("not hex"));
    }
}
