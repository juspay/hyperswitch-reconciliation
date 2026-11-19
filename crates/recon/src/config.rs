use std::{fs, path::PathBuf};

use crate::errors::ReconAppError;
use common_utils::{
    api_key::ApiKeyHashKey,
    errors::{CryptoError, CustomResult},
};
use config::{Environment, File};
use error_stack::{Report, ResultExt};
use external_services::{
    faas::FaasConfig, file_storage::FileStorageConfig, secrets::SecretsManagementConfig,
    ApplicationEncryptionConfig, SchedulerConfig, UserConfig,
};
use masking::{PeekInterface, Secret};
use recon_env::{env, logging::LoggerConfig};
use recon_interfaces::{
    secrets_interface::secret_state::{SecretState, SecretStateContainer, SecuredSecret},
    types::Proxy,
};
use serde::Deserialize;

pub type ApplicationResult<T> = Result<T, Report<ReconAppError>>;

#[derive(clap::Parser, Default)]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            username: "db_user".to_string(),
            password: Secret::new("db_pass".to_string()),
            host: "localhost".to_string(),
            port: 5432,
            dbname: "recon".to_string(),
        }
    }
}

impl DatabaseConfig {
    pub fn validate(&self) -> ApplicationResult<()> {
        if self.username.is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "Database username must not be empty".into(),
            )));
        }
        if self.password.peek().is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "Database password must not be empty".into(),
            )));
        }
        if self.host.is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "Database host must not be empty".into(),
            )));
        }
        if self.dbname.is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "Database name must not be empty".into(),
            )));
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
            port: 8080,
            workers: 4,
            shutdown_timeout: 5,
        }
    }
}

impl ServerConfig {
    pub fn validate(&self) -> ApplicationResult<()> {
        if self.host.is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "Server host must not be empty".into(),
            )));
        }
        if self.workers == 0 {
            return Err(Report::from(ReconAppError::ValidationError(
                "Server workers must be greater than 0".into(),
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: Secret<String>,
    pub jwt_expiry_hours: u64,
    pub admin_api_key: Secret<String>,
    pub internal_api_key: Secret<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: Secret::new("jwt_secret".to_string()),
            jwt_expiry_hours: 24,
            admin_api_key: Secret::new("admin_api_key".to_string()),
            internal_api_key: Secret::new("internal_api_key".to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeysConfig {
    pub hash_key: Secret<String>,
}

/// Empty on purpose, so a missing `[api_keys]` section fails `validate`: a publicly known default
/// would defeat the keyed hash.
impl Default for ApiKeysConfig {
    fn default() -> Self {
        Self {
            hash_key: Secret::new(String::new()),
        }
    }
}

impl ApiKeysConfig {
    /// Emptiness only: this runs before decryption, when the value is ciphertext under KMS.
    pub fn validate(&self) -> ApplicationResult<()> {
        if self.hash_key.peek().trim().is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "api_keys.hash_key must not be empty".into(),
            )));
        }
        Ok(())
    }

    /// Call this on the decrypted config only.
    pub fn get_hash_key(&self) -> CustomResult<ApiKeyHashKey, CryptoError> {
        ApiKeyHashKey::from_hex(self.hash_key.peek())
    }
}

impl AuthConfig {
    pub fn validate(&self) -> ApplicationResult<()> {
        if self.internal_api_key.peek().trim().is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "auth.internal_api_key must not be empty".into(),
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestionServiceConfig {
    pub transformation_base_url: String,
    /// Dedicated file storage config for SFTP Internal ingestion
    pub sftp_file_storage: FileStorageConfig,
}

impl Default for IngestionServiceConfig {
    fn default() -> Self {
        Self {
            transformation_base_url: "http://localhost:8080".to_string(),
            sftp_file_storage: FileStorageConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReportType {
    Transaction,
    Exception,
    TransactionFlow,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReportConfig {
    pub transaction_report_function: String,
    pub exception_report_function: String,
    pub transaction_flow_report_function: String,
}

impl ReportConfig {
    pub fn function_name(&self, report_type: ReportType) -> &str {
        match report_type {
            ReportType::Transaction => &self.transaction_report_function,
            ReportType::Exception => &self.exception_report_function,
            ReportType::TransactionFlow => &self.transaction_flow_report_function,
        }
    }

    pub fn validate(&self) -> ApplicationResult<()> {
        if self.transaction_report_function.trim().is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "report.transaction_report_function must not be empty".into(),
            )));
        }
        if self.exception_report_function.trim().is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "report.exception_report_function must not be empty".into(),
            )));
        }
        if self.transaction_flow_report_function.trim().is_empty() {
            return Err(Report::from(ReconAppError::ValidationError(
                "report.transaction_flow_report_function must not be empty".into(),
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config<S: SecretState> {
    pub database: SecretStateContainer<DatabaseConfig, S>,
    pub server: ServerConfig,
    pub proxy: Proxy,
    pub logging: LoggerConfig,
    pub auth: SecretStateContainer<AuthConfig, S>,
    pub api_keys: SecretStateContainer<ApiKeysConfig, S>,
    pub secrets_management: SecretsManagementConfig,
    pub file_storage: FileStorageConfig,
    pub faas: FaasConfig,
    pub scheduler: SecretStateContainer<SchedulerConfig, S>,
    pub ingestion_service: IngestionServiceConfig,
    pub reporting_service: ReportConfig,
    pub application_encryption: SecretStateContainer<ApplicationEncryptionConfig, S>,
    pub user_client: SecretStateContainer<UserConfig, S>,
}

impl Config<SecuredSecret> {
    pub fn new() -> ApplicationResult<Self> {
        Self::with_config_path(None)
    }

    pub fn with_config_path(config_path: Option<PathBuf>) -> ApplicationResult<Self> {
        // Configuration values are picked up in the following priority order (1 being least
        // priority):
        // 1. Defaults from the implementation of the `Default` trait.
        // 2. Values from config file. The config file accessed depends on the environment
        //    specified by the `RUN_ENV` environment variable. `RUN_ENV` can be one of
        //    `development`, `sandbox` or `production`. If nothing is specified for `RUN_ENV`,
        //    `/config/config.toml` file is read.
        // 3. Environment variables prefixed with `RECON` and each level separated by double
        //    underscores.
        //
        // Values in config file override the defaults in `Default` trait, and the values set using
        // environment variables override both the defaults and the config file values.

        let environment = env::which();
        let config_path = recon_env::config::config_path(&environment.to_string(), config_path);

        let config = recon_env::config::builder(&environment.to_string())
            .change_context(ReconAppError::ValidationError("Configuration error".into()))?
            .add_source(File::from(config_path).required(false))
            .add_source(
                Environment::with_prefix("RECON")
                    .try_parsing(true)
                    .separator("__"),
            )
            .build()
            .change_context(ReconAppError::ValidationError("Configuration error".into()))?;

        serde_path_to_error::deserialize(config)
            .attach("Unable to deserialize application configuration")
            .change_context(ReconAppError::ValidationError("Configuration error".into()))
    }

    pub fn validate(&self) -> ApplicationResult<()> {
        self.server.validate()?;
        self.database.get_inner().validate()?;
        self.auth.get_inner().validate()?;
        self.api_keys.get_inner().validate()?;
        self.application_encryption
            .get_inner()
            .validate()
            .map_err(|message| Report::new(ReconAppError::ValidationError(message)))?;
        self.user_client
            .get_inner()
            .validate()
            .map_err(|message| Report::new(ReconAppError::ValidationError(message)))?;
        self.reporting_service.validate()?;
        Ok(())
    }
}

#[allow(clippy::panic, clippy::expect_used)]
/// Loads the configuration from a TOML file at the specified path.
/// # Panics
/// If the file cannot be read or parsed, it will panic with an error message.
pub fn load_config(path: &str) -> Config<SecuredSecret> {
    let config_content =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("Could not read config file: {path}"));
    toml::from_str(&config_content).expect("Could not parse config file")
}

pub fn load_config_from_env() -> Config<SecuredSecret> {
    let config_path =
        std::env::var("CONFIG_FILE").unwrap_or_else(|_| "config/config.toml".to_string());
    load_config(&config_path)
}
