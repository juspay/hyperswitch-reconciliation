//! Decrypts the secrets in the loaded config.
use async_trait::async_trait;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use recon_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::config::{Config, DatabaseConfig};

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

#[async_trait]
impl SecretsHandler for DatabaseConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
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

/// Decrypts every secret in `conf`.
pub async fn fetch_raw_secrets(
    conf: Config<SecuredSecret>,
    secret_management_client: &dyn SecretManagementInterface,
) -> Result<Config<RawSecret>, SecretsManagementError> {
    let database = DatabaseConfig::convert_to_raw_secret(conf.database, secret_management_client)
        .await
        .change_context(SecretsManagementError::FetchSecretFailed)
        .attach("Failed to decrypt database configuration")?;

    Ok(Config {
        server: conf.server,
        logging: conf.logging,
        secrets_management: conf.secrets_management,
        database,
    })
}
