use std::fmt;

use async_trait::async_trait;
use error_stack::Report;
use masking::{PeekInterface, Secret};
use recon_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};
use serde::Deserialize;

/// Connection settings for a Postgres database. Each binary deserializes its own.
#[derive(Debug, Clone, Deserialize)]
pub struct Database {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
}

impl Default for Database {
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

/// Returned by [`Database::validate`] when a required field is empty.
#[derive(Debug)]
pub struct InvalidDatabaseConfig(&'static str);

impl fmt::Display for InvalidDatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for InvalidDatabaseConfig {}

impl Database {
    pub fn get_database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.peek(),
            self.host,
            self.port,
            self.dbname
        )
    }

    pub fn validate(&self) -> Result<(), Report<InvalidDatabaseConfig>> {
        if self.username.is_empty() {
            return Err(Report::new(InvalidDatabaseConfig(
                "Database username must not be empty",
            )));
        }
        if self.password.peek().is_empty() {
            return Err(Report::new(InvalidDatabaseConfig(
                "Database password must not be empty",
            )));
        }
        if self.host.is_empty() {
            return Err(Report::new(InvalidDatabaseConfig(
                "Database host must not be empty",
            )));
        }
        if self.dbname.is_empty() {
            return Err(Report::new(InvalidDatabaseConfig(
                "Database name must not be empty",
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl SecretsHandler for Database {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> error_stack::Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let db = value.get_inner();
        let db_password = secret_management_client
            .get_secret(db.password.clone())
            .await?;

        Ok(value.transition_state(|db| Self {
            password: Secret::new(db_password.peek().clone()),
            ..db
        }))
    }
}
