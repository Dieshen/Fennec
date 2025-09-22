//! # Fennec Telemetry
//!
//! Comprehensive telemetry, logging, and observability system for Fennec.
//!
//! ## Features
//!
//! - **Structured Logging**: JSON and human-readable formats with `tracing`
//! - **File Rotation**: Size and time-based log rotation with compression
//! - **Privacy & Security**: Automatic sanitization of sensitive data
//! - **Performance Metrics**: Request tracing, timing, and correlation IDs
//! - **Configurable**: Runtime log level adjustment and environment-based config
//! - **Retention Policies**: Automatic cleanup and archival of old logs
//!
//! ## Quick Start
//!
//! ```rust
//! use fennec_telemetry::{TelemetrySystem, TelemetryConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = TelemetryConfig::default();
//!     let _guard = TelemetrySystem::init(config).await?;
//!     
//!     tracing::info!("Application started");
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod correlation;
pub mod filters;
pub mod formatters;
pub mod metrics;
pub mod retention;
pub mod rotation;
pub mod sanitization;
pub mod system;

#[cfg(test)]
mod tests;

pub use config::{LogFormat, LogLevel, TelemetryConfig};
pub use correlation::{CorrelationId, RequestContext};
pub use system::{TelemetryGuard, TelemetrySystem};

// Re-export commonly used tracing macros and types
pub use tracing::{debug, error, info, trace, warn, Instrument, Span};

/// Result type for telemetry operations
pub type Result<T> = std::result::Result<T, Error>;

/// Telemetry-specific errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Log rotation error: {message}")]
    Rotation { message: String },

    #[error("Retention policy error: {message}")]
    Retention { message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Telemetry system error: {message}")]
    System { message: String },
}
