//! Clean ID generation using nanoid with base62 alphabet
//!
//! Format: Prefix + 20 alphanumeric characters (no special chars)
//! Example: txn_a3D9k2MP8xQ4r7V9w2z5

pub mod consts {
    /// Base62 alphabet (A-Z, a-z, 0-9) - no special characters
    pub const ALPHABETS: [char; 62] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
        'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1',
        '2', '3', '4', '5', '6', '7', '8', '9',
    ];

    /// Default ID length (20 characters)
    pub const ID_LENGTH: usize = 20;
}

/// Generates an ID with the default length (20 characters) and given prefix
///
/// The ID uses base62 encoding which includes uppercase letters, lowercase letters,
/// and digits (A-Z, a-z, 0-9), providing clean IDs without special characters.
///
/// # Arguments
///
/// * `prefix` - The prefix to use for the ID (e.g., "txn", "account", "entry")
///
/// # Examples
///
/// ```
/// use common_utils::id_generation::generate_id_with_default_len;
///
/// let txn_id = generate_id_with_default_len("txn");
/// // Example output: "txn_a3D9k2MP8xQ4r7V9w2z5"
/// ```
pub fn generate_id_with_default_len(prefix: &str) -> String {
    let len: usize = consts::ID_LENGTH;
    format!("{}_{}", prefix, nanoid::nanoid!(len, &consts::ALPHABETS))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_with_prefix() {
        let id = generate_id_with_default_len("txn");
        assert!(id.starts_with("txn_"), "ID should start with prefix");
        // prefix (3) + underscore (1) + ID length (20) = 24
        assert_eq!(id.len(), 24, "ID should be 24 characters total");
    }

    #[test]
    fn test_generate_id_format() {
        let id = generate_id_with_default_len("test");
        let id_part = id.split('_').nth(1).unwrap();

        // Should only contain alphanumeric characters from base62 alphabet
        assert!(
            id_part.chars().all(|c| c.is_ascii_alphanumeric()),
            "ID should only contain base62 alphanumeric characters"
        );

        // Should not contain special characters
        assert!(!id_part.contains('_'), "ID should not contain underscore");
        assert!(!id_part.contains('-'), "ID should not contain hyphen");
    }

    #[test]
    fn test_generate_id_uniqueness() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let id = generate_id_with_default_len("test");
            ids.insert(id);
        }
        assert_eq!(ids.len(), 1000, "All generated IDs should be unique");
    }

    #[test]
    fn test_different_prefixes() {
        let txn_id = generate_id_with_default_len("txn");
        let account_id = generate_id_with_default_len("account");
        let entry_id = generate_id_with_default_len("entry");

        assert!(txn_id.starts_with("txn_"));
        assert!(account_id.starts_with("account_"));
        assert!(entry_id.starts_with("entry_"));
    }
}
