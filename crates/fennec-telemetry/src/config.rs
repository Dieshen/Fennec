//! Telemetry configuration and management

use crate::{Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::Level;

/// Main telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Global enable/disable toggle
    pub enabled: bool,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// File rotation and retention settings
    pub retention: RetentionConfig,

    /// Privacy and security settings
    pub privacy: PrivacyConfig,
}

/// Logging-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (TRACE, DEBUG, INFO, WARN, ERROR)
    pub level: LogLevel,

    /// Output format (JSON, Pretty, Compact)
    pub format: LogFormat,

    /// Enable console logging
    pub console_enabled: bool,

    /// Enable file logging
    pub file_enabled: bool,

    /// Directory for log files
    pub log_dir: PathBuf,

    /// Base filename for logs
    pub log_file_name: String,

    /// Maximum log file size before rotation (in MB)
    pub max_file_size_mb: u64,

    /// Include timestamps in logs
    pub include_timestamps: bool,

    /// Include source location in logs
    pub include_location: bool,

    /// Include thread names/IDs
    pub include_thread_info: bool,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Enable performance timing
    pub performance_timing: bool,

    /// Enable request correlation tracking
    pub correlation_tracking: bool,

    /// Enable Prometheus metrics export
    pub prometheus_enabled: bool,

    /// Prometheus metrics endpoint port
    pub prometheus_port: u16,

    /// Metrics collection interval (in seconds)
    pub collection_interval_seconds: u64,
}

/// Log retention and rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Maximum number of log files to keep
    pub max_files: u32,

    /// Maximum age of log files (in days)
    pub max_age_days: u32,

    /// Enable log compression
    pub compress_old_files: bool,

    /// Cleanup check interval (in hours)
    pub cleanup_interval_hours: u32,

    /// Maximum total disk usage for logs (in MB)
    pub max_total_size_mb: u64,
}

/// Privacy and security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Enable automatic sanitization of sensitive data
    pub sanitize_enabled: bool,

    /// Patterns to redact from logs (regex patterns)
    pub redaction_patterns: Vec<String>,

    /// Fields to always redact
    pub redacted_fields: Vec<String>,

    /// Enable audit trail logging
    pub audit_trail: bool,

    /// Audit log file path
    pub audit_log_path: Option<PathBuf>,
}

/// Log level configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::TRACE => LogLevel::Trace,
            Level::DEBUG => LogLevel::Debug,
            Level::INFO => LogLevel::Info,
            Level::WARN => LogLevel::Warn,
            Level::ERROR => LogLevel::Error,
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogFormat {
    /// Structured JSON format
    Json,
    /// Human-readable pretty format
    Pretty,
    /// Compact single-line format
    Compact,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        let default_log_dir = Self::default_log_dir().unwrap_or_else(|_| PathBuf::from("./logs"));

        Self {
            enabled: true,
            logging: LoggingConfig {
                level: LogLevel::Info,
                format: LogFormat::Pretty,
                console_enabled: true,
                file_enabled: true,
                log_dir: default_log_dir,
                log_file_name: "fennec".to_string(),
                max_file_size_mb: 100,
                include_timestamps: true,
                include_location: false,
                include_thread_info: false,
            },
            metrics: MetricsConfig {
                enabled: true,
                performance_timing: true,
                correlation_tracking: true,
                prometheus_enabled: false,
                prometheus_port: 9090,
                collection_interval_seconds: 60,
            },
            retention: RetentionConfig {
                max_files: 10,
                max_age_days: 30,
                compress_old_files: true,
                cleanup_interval_hours: 24,
                max_total_size_mb: 1024, // 1GB
            },
            privacy: PrivacyConfig {
                sanitize_enabled: true,
                redaction_patterns: vec![
                    // API keys and tokens
                    r"(?i)(api_?key|token|secret|password)\s*[:=]\s*['\x22]?([a-zA-Z0-9_\-\.]+)['\x22]?".to_string(),
                    // Credit card numbers
                    r"\b\d{4}[\s\-]?\d{4}[\s\-]?\d{4}[\s\-]?\d{4}\b".to_string(),
                    // Social security numbers
                    r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
                    // Email addresses (partial redaction)
                    r"\b([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})\b".to_string(),
                ],
                redacted_fields: vec![
                    "password".to_string(),
                    "api_key".to_string(),
                    "secret".to_string(),
                    "token".to_string(),
                    "authorization".to_string(),
                ],
                audit_trail: true,
                audit_log_path: None,
            },
        }
    }
}

impl TelemetryConfig {
    /// Load configuration from file or create default
    pub async fn load(config_path: Option<&Path>) -> Result<Self> {
        let config_file = match config_path {
            Some(path) => path.to_path_buf(),
            None => Self::default_config_path()?,
        };

        if config_file.exists() {
            let content = tokio::fs::read_to_string(&config_file).await?;
            let config: TelemetryConfig = toml::from_str(&content).map_err(|e| Error::Config {
                message: format!("Failed to parse telemetry config: {}", e),
            })?;
            Ok(config)
        } else {
            let mut config = Self::default();
            config.load_env_overrides();
            Ok(config)
        }
    }

    /// Save configuration to file
    pub async fn save(&self, config_path: Option<&Path>) -> Result<()> {
        let config_file = match config_path {
            Some(path) => path.to_path_buf(),
            None => Self::default_config_path()?,
        };

        // Ensure parent directory exists
        if let Some(parent) = config_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| Error::Config {
            message: format!("Failed to serialize telemetry config: {}", e),
        })?;

        tokio::fs::write(&config_file, content).await?;
        Ok(())
    }

    /// Load environment variable overrides
    pub fn load_env_overrides(&mut self) {
        // Global toggles
        if let Ok(enabled) = std::env::var("FENNEC_TELEMETRY_ENABLED") {
            self.enabled = enabled.parse().unwrap_or(self.enabled);
        }

        // Log level
        if let Ok(level) = std::env::var("FENNEC_LOG_LEVEL") {
            self.logging.level = match level.to_uppercase().as_str() {
                "TRACE" => LogLevel::Trace,
                "DEBUG" => LogLevel::Debug,
                "INFO" => LogLevel::Info,
                "WARN" => LogLevel::Warn,
                "ERROR" => LogLevel::Error,
                _ => self.logging.level,
            };
        }

        // Log format
        if let Ok(format) = std::env::var("FENNEC_LOG_FORMAT") {
            self.logging.format = match format.to_lowercase().as_str() {
                "json" => LogFormat::Json,
                "pretty" => LogFormat::Pretty,
                "compact" => LogFormat::Compact,
                _ => self.logging.format,
            };
        }

        // File logging
        if let Ok(enabled) = std::env::var("FENNEC_FILE_LOGGING") {
            self.logging.file_enabled = enabled.parse().unwrap_or(self.logging.file_enabled);
        }

        // Log directory
        if let Ok(dir) = std::env::var("FENNEC_LOG_DIR") {
            self.logging.log_dir = PathBuf::from(dir);
        }

        // Metrics
        if let Ok(enabled) = std::env::var("FENNEC_METRICS_ENABLED") {
            self.metrics.enabled = enabled.parse().unwrap_or(self.metrics.enabled);
        }

        // Privacy settings
        if let Ok(enabled) = std::env::var("FENNEC_SANITIZE_LOGS") {
            self.privacy.sanitize_enabled =
                enabled.parse().unwrap_or(self.privacy.sanitize_enabled);
        }
    }

    /// Get default configuration file path
    fn default_config_path() -> Result<PathBuf> {
        let project_dirs =
            ProjectDirs::from("com", "fennec", "fennec").ok_or_else(|| Error::Config {
                message: "Could not determine config directory".to_string(),
            })?;

        Ok(project_dirs.config_dir().join("telemetry.toml"))
    }

    /// Get default log directory
    fn default_log_dir() -> Result<PathBuf> {
        let project_dirs =
            ProjectDirs::from("com", "fennec", "fennec").ok_or_else(|| Error::Config {
                message: "Could not determine log directory".to_string(),
            })?;

        Ok(project_dirs.data_dir().join("logs"))
    }

    /// Get the log file path for a given log level
    pub fn log_file_path(&self, level: Option<LogLevel>) -> PathBuf {
        let suffix = match level {
            Some(LogLevel::Error) => "_error",
            Some(LogLevel::Warn) => "_warn",
            Some(LogLevel::Debug) => "_debug",
            Some(LogLevel::Trace) => "_trace",
            _ => "",
        };

        self.logging
            .log_dir
            .join(format!("{}{}.log", self.logging.log_file_name, suffix))
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate log directory is writable
        if self.logging.file_enabled {
            if !self.logging.log_dir.exists() {
                std::fs::create_dir_all(&self.logging.log_dir)?;
            }

            // Test write permissions
            let test_file = self.logging.log_dir.join(".fennec_test");
            std::fs::write(&test_file, "test")?;
            std::fs::remove_file(&test_file)?;
        }

        // Validate retention settings
        if self.retention.max_files == 0 {
            return Err(Error::Config {
                message: "max_files must be greater than 0".to_string(),
            });
        }

        if self.retention.max_age_days == 0 {
            return Err(Error::Config {
                message: "max_age_days must be greater than 0".to_string(),
            });
        }

        // Validate regex patterns
        for pattern in &self.privacy.redaction_patterns {
            regex::Regex::new(pattern).map_err(|e| Error::Config {
                message: format!("Invalid redaction pattern '{}': {}", pattern, e),
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_serialization() {
        let config = TelemetryConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: TelemetryConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.logging.level as u8, deserialized.logging.level as u8);
    }

    #[tokio::test]
    async fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("telemetry.toml");

        let original_config = TelemetryConfig::default();
        original_config.save(Some(&config_path)).await.unwrap();

        let loaded_config = TelemetryConfig::load(Some(&config_path)).await.unwrap();
        assert_eq!(original_config.enabled, loaded_config.enabled);
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("FENNEC_LOG_LEVEL", "DEBUG");
        std::env::set_var("FENNEC_LOG_FORMAT", "json");

        let mut config = TelemetryConfig::default();
        config.load_env_overrides();

        assert!(matches!(config.logging.level, LogLevel::Debug));
        assert!(matches!(config.logging.format, LogFormat::Json));

        std::env::remove_var("FENNEC_LOG_LEVEL");
        std::env::remove_var("FENNEC_LOG_FORMAT");
    }

    #[test]
    fn test_config_validation() {
        let mut config = TelemetryConfig::default();
        config.retention.max_files = 0;

        assert!(config.validate().is_err());

        config.retention.max_files = 5;
        config.privacy.redaction_patterns = vec!["[invalid".to_string()];

        assert!(config.validate().is_err());
    }
}
