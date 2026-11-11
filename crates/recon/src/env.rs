//! Information about the current environment.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Environment variables accessed by the application.
pub mod vars {
    /// Parent directory where `Cargo.toml` is stored.
    pub const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";

    /// Environment variable that sets development/production environment.
    pub const RUN_ENV: &str = "RUN_ENV";

    /// Directory of config TOML files. Default is `config`.
    pub const CONFIG_DIR: &str = "CONFIG_DIR";
}

/// Current environment.
#[derive(
    Debug,
    Default,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Env {
    /// Development environment.
    #[default]
    Development,
    /// Production environment.
    Production,
    Sandbox,
}

impl Env {
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Development => "dev",
            Self::Sandbox => "sbx",
            Self::Production => "prd",
        }
    }
}

/// Name of current environment. One of "development", "sandbox" or "production".
pub fn which() -> Env {
    let default_env = Env::Development;
    std::env::var(vars::RUN_ENV).map_or_else(|_| default_env, |v| v.parse().unwrap_or(default_env))
}

/// Path to the root directory of the cargo workspace.
/// It is recommended that this be used by the application as the base path to build other paths
/// such as configuration and logs directories.
pub fn workspace_path() -> PathBuf {
    if let Ok(manifest_dir) = std::env::var(vars::CARGO_MANIFEST_DIR) {
        let mut path = PathBuf::from(manifest_dir);
        path.pop();
        path.pop();
        path
    } else {
        PathBuf::from(".")
    }
}

/// Service name deduced from name of the binary.
/// This macro must be called within binaries only.
///
/// Example: `recon`.
#[macro_export]
macro_rules! service_name {
    () => {
        env!("CARGO_BIN_NAME")
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_env_prefix_is_a_valid_key_prefix() {
        use strum::IntoEnumIterator;

        for env in Env::iter() {
            let prefix = env.prefix();
            assert!(!prefix.is_empty() && !prefix.contains('_'), "{env}");
        }
    }

    #[test]
    fn each_env_has_its_own_prefix() {
        assert_eq!(Env::Development.prefix(), "dev");
        assert_eq!(Env::Sandbox.prefix(), "sbx");
        assert_eq!(Env::Production.prefix(), "prd");
    }
}
