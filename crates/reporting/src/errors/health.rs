use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum HealthCheckError {
    #[error("Unable to reach the database")]
    DatabaseUnreachable,

    #[error("Health check timed out")]
    TimedOut,
}
