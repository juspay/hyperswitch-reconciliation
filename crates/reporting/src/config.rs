use std::path::PathBuf;

use config::{Environment, File};
use error_stack::{Report, ResultExt};
use external_services::secrets::SecretsManagementConfig;
use masking::{PeekInterface, Secret};
use recon_env::{env, logging::LoggerConfig};
use recon_interfaces::secrets_interface::secret_state::{
    SecretState, SecretStateContainer, SecuredSecret,
};
use serde::Deserialize;

use crate::errors::ReportingAppError;

pub type ApplicationResult<T> = Result<T, Report<ReportingAppError>>;

/// Config directory, relative to the workspace root, when `CONFIG_DIR` is not set.
pub const DEFAULT_CONFIG_DIR: &str = "config/reporting";

/// Prefix for environment variable overrides, e.g. `REPORTING__SERVER__PORT`.
pub const ENV_PREFIX: &str = "REPORTING";

#[derive(clap::Parser, Default)]
pub struct CmdLineConf {
    /// Config file.
    /// The service looks for "config/reporting/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

/// Connection settings for the recon Postgres database.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
}

impl DatabaseConfig {
    /// `section` names the config section in error messages, e.g. `database`.
    pub fn validate(&self, section: &str) -> ApplicationResult<()> {
        let empty = |field: &str| {
            Err(Report::from(ReportingAppError::ValidationError(format!(
                "{section}.{field} must not be empty"
            ))))
        };
        if self.username.trim().is_empty() {
            return empty("username");
        }
        if self.password.peek().is_empty() {
            return empty("password");
        }
        if self.host.trim().is_empty() {
            return empty("host");
        }
        if self.dbname.trim().is_empty() {
            return empty("dbname");
        }
        Ok(())
    }
}

impl From<DatabaseConfig> for storage_impl::config::Database {
    fn from(database: DatabaseConfig) -> Self {
        Self {
            username: database.username,
            password: database.password,
            host: database.host,
            port: database.port,
            dbname: database.dbname,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub shutdown_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8081,
            workers: 4,
            shutdown_timeout: 5,
        }
    }
}

impl ServerConfig {
    pub fn validate(&self) -> ApplicationResult<()> {
        if self.host.trim().is_empty() {
            return Err(Report::from(ReportingAppError::ValidationError(
                "server.host must not be empty".into(),
            )));
        }
        if self.workers == 0 {
            return Err(Report::from(ReportingAppError::ValidationError(
                "server.workers must be greater than 0".into(),
            )));
        }
        Ok(())
    }
}

/// `database` has no default: a missing section fails at startup instead of sending queries
/// somewhere unintended.
#[derive(Debug, Clone, Deserialize)]
pub struct Config<S: SecretState> {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub logging: LoggerConfig,
    #[serde(default)]
    pub secrets_management: SecretsManagementConfig,
    /// Master database, shared with recon.
    pub database: SecretStateContainer<DatabaseConfig, S>,
}

impl Config<SecuredSecret> {
    pub fn new() -> ApplicationResult<Self> {
        Self::with_config_path(None)
    }

    /// Loads config in this order, later sources overriding earlier ones:
    /// 1. The config file. Without an explicit path it is `config.toml`, `sandbox.toml` or
    ///    `prod.toml` in `CONFIG_DIR` (default `config/reporting`), picked by `RUN_ENV`.
    /// 2. Environment variables prefixed with `REPORTING`, levels separated by `__`.
    pub fn with_config_path(config_path: Option<PathBuf>) -> ApplicationResult<Self> {
        let environment = env::which().to_string();
        let config_path =
            recon_env::config::config_path_in(&environment, config_path, DEFAULT_CONFIG_DIR);

        let config = recon_env::config::builder(&environment)
            .change_context(ReportingAppError::ValidationError(
                "Configuration error".into(),
            ))?
            .add_source(File::from(config_path).required(false))
            .add_source(
                Environment::with_prefix(ENV_PREFIX)
                    .try_parsing(true)
                    .separator("__"),
            )
            .build()
            .change_context(ReportingAppError::ValidationError(
                "Configuration error".into(),
            ))?;

        serde_path_to_error::deserialize(config)
            .attach("Unable to deserialize reporting service configuration")
            .change_context(ReportingAppError::ValidationError(
                "Configuration error".into(),
            ))
    }

    pub fn validate(&self) -> ApplicationResult<()> {
        self.server.validate()?;
        self.database.get_inner().validate("database")?;
        self.secrets_management
            .validate()
            .change_context(ReportingAppError::ValidationError(
                "Invalid secrets_management configuration".into(),
            ))?;
        Ok(())
    }
}
