//! Helpers for loading config files. Each binary keeps its own config struct and calls these.

use std::path::PathBuf;

use crate::env;

/// Construct config builder extending it by fall-back defaults and setting config file to load.
pub fn builder(
    environment: &str,
) -> Result<config::ConfigBuilder<config::builder::DefaultState>, config::ConfigError> {
    config::Config::builder()
        // Here, it should be `set_override()` not `set_default()`.
        // "env" can't be altered by config field.
        // Should be single source of truth.
        .set_override("env", environment)
}

/// Directory of config files when `CONFIG_DIR` is not set.
pub const DEFAULT_CONFIG_DIR: &str = "config";

/// Config path, with `config/` as the default config directory.
pub fn config_path(environment: &str, explicit_config_path: Option<PathBuf>) -> PathBuf {
    config_path_in(environment, explicit_config_path, DEFAULT_CONFIG_DIR)
}

/// Config path, with `default_config_dir` used when `CONFIG_DIR` is not set.
/// A binary whose config files live outside `config/` passes its own directory here.
pub fn config_path_in(
    environment: &str,
    explicit_config_path: Option<PathBuf>,
    default_config_dir: &str,
) -> PathBuf {
    let mut config_path = PathBuf::new();
    if let Some(explicit_config_path_val) = explicit_config_path {
        config_path.push(explicit_config_path_val);
    } else {
        let config_file_name = match environment {
            "production" => "prod.toml",
            "sandbox" => "sandbox.toml",
            _ => "config.toml",
        };

        let config_directory = get_config_directory_or(default_config_dir);
        config_path.push(config_directory);
        config_path.push(config_file_name);
    }
    config_path
}

/// Get the Directory for the config file
/// Read the env variable `CONFIG_DIR` or fallback to `config`
pub fn get_config_directory() -> PathBuf {
    get_config_directory_or(DEFAULT_CONFIG_DIR)
}

/// Read the env variable `CONFIG_DIR` or fall back to `default_config_dir`, relative to the
/// workspace root.
pub fn get_config_directory_or(default_config_dir: &str) -> PathBuf {
    let mut config_path = PathBuf::new();

    let config_directory =
        std::env::var(env::vars::CONFIG_DIR).unwrap_or_else(|_| default_config_dir.into());

    config_path.push(env::workspace_path());
    config_path.push(config_directory);
    config_path
}
