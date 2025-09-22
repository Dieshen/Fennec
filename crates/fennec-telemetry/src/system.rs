//! Main telemetry system implementation

use crate::{
    config::{LogFormat, TelemetryConfig},
    correlation::CorrelationLayer,
    filters::SanitizationLayer,
    metrics::MetricsLayer,
    retention::RetentionManager,
    rotation::RotatingFileWriter,
    Error, Result,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer, Registry,
};

/// Main telemetry system that coordinates all telemetry components
pub struct TelemetrySystem {
    config: Arc<RwLock<TelemetryConfig>>,
    retention_manager: Option<RetentionManager>,
    file_writer: Option<RotatingFileWriter>,
    _guard: Option<TelemetryGuard>,
}

/// Guard that ensures proper cleanup of telemetry resources
pub struct TelemetryGuard {
    _inner: Box<dyn Send + Sync>,
}

impl TelemetrySystem {
    /// Initialize the telemetry system with the given configuration
    pub async fn init(config: TelemetryConfig) -> Result<TelemetryGuard> {
        // Validate configuration
        config.validate()?;

        let config = Arc::new(RwLock::new(config));
        let config_read = config.read().await;

        // Create registry for all layers
        let registry = Registry::default();

        // Build the subscriber with layers
        let subscriber = registry
            .with(Self::build_env_filter(&config_read)?)
            .with(Self::build_console_layer(&config_read)?)
            .with(Self::build_file_layer(&config_read).await?)
            .with(Self::build_sanitization_layer(&config_read)?)
            .with(Self::build_correlation_layer(&config_read)?)
            .with(Self::build_metrics_layer(&config_read)?);

        // Initialize the global subscriber
        subscriber.try_init().map_err(|e| Error::System {
            message: format!("Failed to initialize tracing subscriber: {}", e),
        })?;

        // Start retention manager if file logging is enabled
        let retention_manager = if config_read.logging.file_enabled {
            Some(RetentionManager::new(config_read.retention.clone()).await?)
        } else {
            None
        };

        drop(config_read);

        let guard = TelemetryGuard {
            _inner: Box::new(()),
        };

        tracing::info!(
            telemetry.event = "system_initialized",
            telemetry.version = env!("CARGO_PKG_VERSION"),
            "Telemetry system initialized successfully"
        );

        Ok(guard)
    }

    /// Update the telemetry configuration at runtime
    pub async fn update_config(config: TelemetryConfig) -> Result<()> {
        config.validate()?;

        // For now, we log the configuration change
        // In a full implementation, we would rebuild the subscriber
        tracing::info!(
            telemetry.event = "config_updated",
            config.logging.level = ?config.logging.level,
            config.logging.format = ?config.logging.format,
            "Telemetry configuration updated"
        );

        Ok(())
    }

    /// Build environment filter for log level filtering
    fn build_env_filter(config: &TelemetryConfig) -> Result<EnvFilter> {
        let level: Level = config.logging.level.into();

        // Start with the configured level
        let mut filter = EnvFilter::new(level.to_string());

        // Add specific module filters if needed
        filter = filter.add_directive("hyper=warn".parse().unwrap());
        filter = filter.add_directive("reqwest=warn".parse().unwrap());
        filter = filter.add_directive("h2=warn".parse().unwrap());

        // Allow environment override
        if let Ok(env_filter) = std::env::var("RUST_LOG") {
            filter = EnvFilter::new(env_filter);
        }

        Ok(filter)
    }

    /// Build console logging layer
    fn build_console_layer(
        config: &TelemetryConfig,
    ) -> Result<Option<Box<dyn Layer<Registry> + Send + Sync>>> {
        if !config.enabled || !config.logging.console_enabled {
            return Ok(None);
        }

        let layer = match config.logging.format {
            LogFormat::Json => fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_target(true)
                .with_file(config.logging.include_location)
                .with_line_number(config.logging.include_location)
                .with_thread_ids(config.logging.include_thread_info)
                .with_thread_names(config.logging.include_thread_info)
                .boxed(),
            LogFormat::Pretty => fmt::layer()
                .pretty()
                .with_target(true)
                .with_file(config.logging.include_location)
                .with_line_number(config.logging.include_location)
                .with_thread_ids(config.logging.include_thread_info)
                .with_thread_names(config.logging.include_thread_info)
                .boxed(),
            LogFormat::Compact => fmt::layer()
                .compact()
                .with_target(false)
                .with_file(config.logging.include_location)
                .with_line_number(config.logging.include_location)
                .with_thread_ids(config.logging.include_thread_info)
                .with_thread_names(config.logging.include_thread_info)
                .boxed(),
        };

        Ok(Some(layer))
    }

    /// Build file logging layer
    async fn build_file_layer(
        config: &TelemetryConfig,
    ) -> Result<Option<Box<dyn Layer<Registry> + Send + Sync>>> {
        if !config.enabled || !config.logging.file_enabled {
            return Ok(None);
        }

        // Ensure log directory exists
        tokio::fs::create_dir_all(&config.logging.log_dir).await?;

        // Create rotating file writer
        let file_writer = RotatingFileWriter::new(
            config.logging.log_dir.clone(),
            config.logging.log_file_name.clone(),
            config.logging.max_file_size_mb,
        )?;

        let layer = match config.logging.format {
            LogFormat::Json => fmt::layer()
                .json()
                .with_writer(file_writer)
                .with_ansi(false)
                .with_current_span(true)
                .with_span_list(false)
                .with_target(true)
                .with_file(config.logging.include_location)
                .with_line_number(config.logging.include_location)
                .with_thread_ids(config.logging.include_thread_info)
                .with_thread_names(config.logging.include_thread_info)
                .boxed(),
            LogFormat::Pretty => fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false)
                .with_target(true)
                .with_file(config.logging.include_location)
                .with_line_number(config.logging.include_location)
                .with_thread_ids(config.logging.include_thread_info)
                .with_thread_names(config.logging.include_thread_info)
                .boxed(),
            LogFormat::Compact => fmt::layer()
                .compact()
                .with_writer(file_writer)
                .with_ansi(false)
                .with_target(false)
                .with_file(config.logging.include_location)
                .with_line_number(config.logging.include_location)
                .with_thread_ids(config.logging.include_thread_info)
                .with_thread_names(config.logging.include_thread_info)
                .boxed(),
        };

        Ok(Some(layer))
    }

    /// Build sanitization layer for privacy protection
    fn build_sanitization_layer(config: &TelemetryConfig) -> Result<Option<SanitizationLayer>> {
        if !config.enabled || !config.privacy.sanitize_enabled {
            return Ok(None);
        }

        Ok(Some(SanitizationLayer::new(config.privacy.clone())?))
    }

    /// Build correlation layer for request tracing
    fn build_correlation_layer(config: &TelemetryConfig) -> Result<Option<CorrelationLayer>> {
        if !config.enabled || !config.metrics.correlation_tracking {
            return Ok(None);
        }

        Ok(Some(CorrelationLayer::new()))
    }

    /// Build metrics layer for performance monitoring
    fn build_metrics_layer(config: &TelemetryConfig) -> Result<Option<MetricsLayer>> {
        if !config.enabled || !config.metrics.enabled {
            return Ok(None);
        }

        Ok(Some(MetricsLayer::new(config.metrics.clone())?))
    }

    /// Get current telemetry statistics
    pub async fn get_stats() -> TelemetryStats {
        TelemetryStats {
            logs_written: 0, // TODO: Implement actual counters
            files_rotated: 0,
            files_cleaned: 0,
            total_disk_usage_mb: 0,
            uptime_seconds: 0,
        }
    }

    /// Flush all pending telemetry data
    pub async fn flush() -> Result<()> {
        // Force flush all appenders and writers
        // This is automatically handled by tracing-subscriber, but we provide
        // an explicit flush method for graceful shutdown
        tracing::info!(telemetry.event = "manual_flush", "Flushing telemetry data");
        Ok(())
    }

    /// Perform emergency shutdown of telemetry system
    pub async fn emergency_shutdown() -> Result<()> {
        tracing::error!(
            telemetry.event = "emergency_shutdown",
            "Emergency telemetry shutdown initiated"
        );

        // Flush all data
        Self::flush().await?;

        Ok(())
    }
}

/// Telemetry system statistics
#[derive(Debug, Clone)]
pub struct TelemetryStats {
    pub logs_written: u64,
    pub files_rotated: u32,
    pub files_cleaned: u32,
    pub total_disk_usage_mb: u64,
    pub uptime_seconds: u64,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        // Log the telemetry system shutdown
        // Note: This happens during program shutdown, so logging may not work
        tracing::info!(
            telemetry.event = "system_shutdown",
            "Telemetry system shutting down"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_telemetry_init() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Test that logging works
        tracing::info!(test = "value", "Test log message");
        tracing::warn!("Test warning");
        tracing::error!("Test error");
    }

    #[tokio::test]
    async fn test_console_only_logging() {
        let mut config = TelemetryConfig::default();
        config.logging.file_enabled = false;
        config.logging.console_enabled = true;

        let _guard = TelemetrySystem::init(config).await.unwrap();

        tracing::info!("Console-only test message");
    }

    #[tokio::test]
    async fn test_disabled_telemetry() {
        let mut config = TelemetryConfig::default();
        config.enabled = false;

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Even when disabled, the system should initialize successfully
        tracing::info!("This message should be filtered out");
    }
}
