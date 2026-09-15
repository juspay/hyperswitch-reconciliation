//! Secret transformers for configuration structs
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use external_services::{ApplicationEncryptionConfig, UserConfig};
use masking::{PeekInterface, Secret};
use recon_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::config::{AuthConfig, Config, DatabaseConfig};

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

#[async_trait]
impl SecretsHandler for AuthConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let auth = value.get_inner();
        let jwt_secret = secret_management_client
            .get_secret(auth.jwt_secret.clone())
            .await?;
        let admin_api_key = secret_management_client
            .get_secret(auth.admin_api_key.clone())
            .await?;

        Ok(value.transition_state(|auth| Self {
            jwt_secret: Secret::new(jwt_secret.peek().clone()),
            admin_api_key: Secret::new(admin_api_key.peek().clone()),
            ..auth
        }))
    }
}

/// Fetch raw secrets from secured configuration
pub async fn fetch_raw_secrets(
    conf: Config<SecuredSecret>,
    secret_management_client: &dyn SecretManagementInterface,
) -> Result<Config<RawSecret>, SecretsManagementError> {
    let database = DatabaseConfig::convert_to_raw_secret(conf.database, secret_management_client)
        .await
        .change_context(SecretsManagementError::FetchSecretFailed)
        .attach_printable("Failed to decrypt database configuration")?;

    let auth = AuthConfig::convert_to_raw_secret(conf.auth, secret_management_client)
        .await
        .change_context(SecretsManagementError::FetchSecretFailed)
        .attach_printable("Failed to decrypt auth configuration")?;

    let application_encryption = ApplicationEncryptionConfig::convert_to_raw_secret(
        conf.application_encryption,
        secret_management_client,
    )
    .await
    .change_context(SecretsManagementError::FetchSecretFailed)
    .attach_printable("Failed to decrypt application encryption configuration")?;

    let user_client = UserConfig::convert_to_raw_secret(conf.user_client, secret_management_client)
        .await
        .change_context(SecretsManagementError::FetchSecretFailed)
        .attach_printable("Failed to decrypt HyperSwitch user configuration")?;

    Ok(Config {
        database,
        server: conf.server,
        proxy: conf.proxy,
        logging: conf.logging,
        auth,
        secrets_management: conf.secrets_management,
        file_storage: conf.file_storage,
        faas: conf.faas,
        ingestion_service: conf.ingestion_service,
        reporting_service: conf.reporting_service,
        application_encryption,
        user_client,
    })
}
