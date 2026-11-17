use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum HealthCheckError {
    #[error("Unable to reach the database")]
    DatabaseUnreachable,

    #[error("Unable to reach the encryption service")]
    EncryptionUnreachable,

    #[error("Unable to reach the user service")]
    UserClientUnreachable,

    #[error("Processor task is not alive")]
    ProcessorNotAlive,

    #[error("Health check timed out")]
    TimedOut,
}
