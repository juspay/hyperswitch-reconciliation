use serde::Deserialize;
pub use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

/// Log format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON formatted logs
    Json,
    /// Plain text formatted logs
    Plain,
    /// Pretty formatted logs with colors and formatting
    #[default]
    Pretty,
}

/// Logger configuration options
#[derive(Debug, Clone, Deserialize)]
pub struct LoggerConfig {
    /// The minimum log level to capture
    pub log_level: Level,
    /// Whether to output logs to the console
    pub console_output: bool,
    /// The format of the logs for console output
    pub format: LogFormat,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            log_level: Level::debug(),
            console_output: true,
            format: LogFormat::Pretty,
        }
    }
}

/// Describes the level of verbosity of a span or event.
#[derive(Debug, Clone, Copy)]
pub struct Level(pub(super) tracing::Level);

impl Level {
    /// Returns the most verbose [`tracing::Level`]
    pub fn into_level(self) -> tracing::Level {
        self.0
    }

    /// Create a DEBUG level
    pub fn debug() -> Self {
        Self(tracing::Level::DEBUG)
    }

    /// Create an INFO level
    pub fn info() -> Self {
        Self(tracing::Level::INFO)
    }

    /// Create a WARN level
    pub fn warn() -> Self {
        Self(tracing::Level::WARN)
    }

    /// Create an ERROR level
    pub fn error() -> Self {
        Self(tracing::Level::ERROR)
    }

    /// Create a TRACE level
    pub fn trace() -> Self {
        Self(tracing::Level::TRACE)
    }
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::str::FromStr as _;

        let s = String::deserialize(deserializer)?;
        tracing::Level::from_str(&s)
            .map(Level)
            .map_err(serde::de::Error::custom)
    }
}

/// Initialize the logging system
pub fn init(config: LoggerConfig) {
    let env_filter = EnvFilter::new(config.log_level.into_level().as_str());

    let format = config.format;
    let registry = tracing_subscriber::registry().with(env_filter);

    match format {
        LogFormat::Json => {
            error_stack::Report::set_color_mode(error_stack::fmt::ColorMode::None);
            registry
                .with(fmt::layer().json().with_writer(std::io::stdout))
                .init();
        }
        LogFormat::Plain => {
            registry
                .with(fmt::layer().with_writer(std::io::stdout))
                .init();
        }
        LogFormat::Pretty => {
            registry
                .with(
                    fmt::layer()
                        .with_writer(std::io::stdout)
                        .with_ansi(true)
                        .pretty()
                        .with_line_number(true)
                        .with_target(true),
                )
                .init();
        }
    }

    // Redirect standard log crate to tracing
    let _ = tracing_log::LogTracer::init();
}
