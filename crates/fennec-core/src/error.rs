use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FennecError>;

/// Error category classification for appropriate handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// User input or configuration errors
    User,
    /// System or infrastructure errors
    System,
    /// Network connectivity errors
    Network,
    /// Security policy violations
    Security,
    /// Internal application errors
    Internal,
}

/// Error severity levels for logging and UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational - operation succeeded with warnings
    Info,
    /// Warning - operation succeeded but with issues
    Warning,
    /// Error - operation failed but system can continue
    Error,
    /// Critical - operation failed and system stability affected
    Critical,
}

/// Recovery actions that can be suggested to users
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry the operation as-is
    Retry,
    /// Retry with different parameters
    RetryWithChanges(String),
    /// Check and fix configuration
    CheckConfiguration(String),
    /// Check file or directory permissions
    CheckPermissions(String),
    /// Contact support or check documentation
    ContactSupport(String),
    /// Manual intervention required
    ManualAction(String),
    /// No recovery possible
    None,
}

/// Trait for error types that provide user-friendly information
pub trait ErrorInfo {
    /// Get the error category for appropriate handling
    fn category(&self) -> ErrorCategory;

    /// Get the severity level
    fn severity(&self) -> ErrorSeverity;

    /// Get suggested recovery actions
    fn recovery_actions(&self) -> Vec<RecoveryAction>;

    /// Check if the operation can be retried
    fn is_retryable(&self) -> bool {
        self.recovery_actions().iter().any(|action| {
            matches!(
                action,
                RecoveryAction::Retry | RecoveryAction::RetryWithChanges(_)
            )
        })
    }

    /// Get user-friendly description without technical details
    fn user_message(&self) -> String;

    /// Get additional context for debugging
    fn debug_context(&self) -> Option<String> {
        None
    }
}

#[derive(Error, Debug)]
pub enum FennecError {
    // Configuration errors with specific guidance
    #[error("Configuration file not found at '{path}'.")]
    ConfigNotFound { path: String },

    #[error("Invalid configuration: {issue}.")]
    ConfigInvalid { issue: String, suggestion: String },

    #[error("Failed to load configuration from '{path}': {source}")]
    ConfigLoadFailed {
        path: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    // File system errors with context
    #[error("Failed to read file '{path}': {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write file '{path}': {source}")]
    FileWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("File not found: '{path}'.")]
    FileNotFound { path: String },

    #[error("Permission denied accessing '{path}'.")]
    PermissionDenied { path: String },

    // Session management errors
    #[error("Session '{session_id}' not found.")]
    SessionNotFound { session_id: String },

    #[error("Session limit exceeded: {current} active sessions (maximum: {max}).")]
    SessionLimitExceeded { current: usize, max: usize },

    #[error("Session '{session_id}' is already active.")]
    SessionAlreadyActive { session_id: String },

    // Workspace errors
    #[error("Workspace not found at '{path}'.")]
    WorkspaceNotFound { path: String },

    #[error("Invalid workspace: {reason}.")]
    InvalidWorkspace { reason: String },

    // Service integration errors
    #[error("Service '{service}' is not available: {reason}")]
    ServiceUnavailable { service: String, reason: String },

    #[error("Service '{service}' initialization failed: {reason}")]
    ServiceInitFailed { service: String, reason: String },

    // Generic IO and serialization (preserved for compatibility)
    #[error("IO operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML parsing failed: {0}")]
    TomlParsing(#[from] toml::de::Error),

    // Domain-specific error delegation (for backward compatibility)
    #[error("Provider error: {0}")]
    Provider(Box<dyn std::error::Error + Send + Sync>),

    #[error("Command error: {0}")]
    Command(Box<dyn std::error::Error + Send + Sync>),

    #[error("Security error: {0}")]
    Security(Box<dyn std::error::Error + Send + Sync>),

    #[error("Memory error: {0}")]
    Memory(Box<dyn std::error::Error + Send + Sync>),

    #[error("TUI error: {0}")]
    Tui(Box<dyn std::error::Error + Send + Sync>),

    #[error("Orchestration error: {0}")]
    Orchestration(Box<dyn std::error::Error + Send + Sync>),

    // Catch-all for unexpected errors
    #[error("Unexpected error: {message}")]
    Unknown {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl ErrorInfo for FennecError {
    fn category(&self) -> ErrorCategory {
        match self {
            // Configuration errors are typically user errors
            FennecError::ConfigNotFound { .. }
            | FennecError::ConfigInvalid { .. }
            | FennecError::ConfigLoadFailed { .. } => ErrorCategory::User,

            // File system errors can be user or system
            FennecError::FileNotFound { .. } => ErrorCategory::User,
            FennecError::PermissionDenied { .. } => ErrorCategory::Security,
            FennecError::FileRead { .. } | FennecError::FileWrite { .. } => ErrorCategory::System,

            // Session and workspace errors are typically user
            FennecError::SessionNotFound { .. }
            | FennecError::SessionAlreadyActive { .. }
            | FennecError::WorkspaceNotFound { .. }
            | FennecError::InvalidWorkspace { .. } => ErrorCategory::User,

            FennecError::SessionLimitExceeded { .. } => ErrorCategory::System,

            // Service errors are system
            FennecError::ServiceUnavailable { .. } | FennecError::ServiceInitFailed { .. } => {
                ErrorCategory::System
            }

            // IO and serialization are system
            FennecError::Io(_) | FennecError::Serialization(_) | FennecError::TomlParsing(_) => {
                ErrorCategory::System
            }

            // Domain-specific errors delegate to their category
            FennecError::Security(_) => ErrorCategory::Security,
            FennecError::Provider(_) => ErrorCategory::Network,
            FennecError::Command(_)
            | FennecError::Memory(_)
            | FennecError::Tui(_)
            | FennecError::Orchestration(_) => ErrorCategory::Internal,

            FennecError::Unknown { .. } => ErrorCategory::Internal,
        }
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            // Configuration issues are errors but not critical
            FennecError::ConfigNotFound { .. }
            | FennecError::ConfigInvalid { .. }
            | FennecError::ConfigLoadFailed { .. } => ErrorSeverity::Error,

            // File system issues range from error to critical
            FennecError::FileNotFound { .. } => ErrorSeverity::Error,
            FennecError::PermissionDenied { .. } => ErrorSeverity::Error,
            FennecError::FileRead { .. } | FennecError::FileWrite { .. } => ErrorSeverity::Error,

            // Session management issues
            FennecError::SessionNotFound { .. } | FennecError::SessionAlreadyActive { .. } => {
                ErrorSeverity::Error
            }
            FennecError::SessionLimitExceeded { .. } => ErrorSeverity::Critical,

            // Workspace issues
            FennecError::WorkspaceNotFound { .. } | FennecError::InvalidWorkspace { .. } => {
                ErrorSeverity::Error
            }

            // Service issues are critical
            FennecError::ServiceUnavailable { .. } | FennecError::ServiceInitFailed { .. } => {
                ErrorSeverity::Critical
            }

            // System errors
            FennecError::Io(_) | FennecError::Serialization(_) | FennecError::TomlParsing(_) => {
                ErrorSeverity::Error
            }

            // Domain-specific errors default to error
            FennecError::Provider(_)
            | FennecError::Command(_)
            | FennecError::Security(_)
            | FennecError::Memory(_)
            | FennecError::Tui(_)
            | FennecError::Orchestration(_) => ErrorSeverity::Error,

            FennecError::Unknown { .. } => ErrorSeverity::Critical,
        }
    }

    fn recovery_actions(&self) -> Vec<RecoveryAction> {
        match self {
            FennecError::ConfigNotFound { path } => {
                vec![
                    RecoveryAction::CheckConfiguration(format!(
                        "Create configuration file at '{}'.",
                        path
                    )),
                    RecoveryAction::CheckConfiguration(
                        "Run 'fennec init' to create default configuration.".to_string(),
                    ),
                ]
            }

            FennecError::ConfigInvalid { suggestion, .. } => {
                vec![
                    RecoveryAction::CheckConfiguration(suggestion.clone()),
                    RecoveryAction::CheckConfiguration(
                        "Validate configuration syntax.".to_string(),
                    ),
                ]
            }

            FennecError::FileNotFound { path } => {
                vec![
                    RecoveryAction::ManualAction(format!("Verify that file '{}' exists.", path)),
                    RecoveryAction::CheckPermissions(
                        "Check read permissions for the file and parent directories.".to_string(),
                    ),
                ]
            }

            FennecError::PermissionDenied { path } => {
                vec![
                    RecoveryAction::CheckPermissions(format!(
                        "Grant appropriate permissions for '{}'.",
                        path
                    )),
                    RecoveryAction::ManualAction(
                        "Run with elevated privileges if necessary.".to_string(),
                    ),
                ]
            }

            FennecError::SessionLimitExceeded { max, .. } => {
                vec![
                    RecoveryAction::ManualAction(
                        "Close unused sessions before starting a new one.".to_string(),
                    ),
                    RecoveryAction::CheckConfiguration(format!(
                        "Increase session limit (current: {}).",
                        max
                    )),
                ]
            }

            FennecError::WorkspaceNotFound { path } => {
                vec![
                    RecoveryAction::ManualAction(format!(
                        "Create workspace directory at '{}'.",
                        path
                    )),
                    RecoveryAction::CheckConfiguration(
                        "Set a valid workspace path in configuration.".to_string(),
                    ),
                ]
            }

            FennecError::ServiceUnavailable { service, .. } => {
                vec![
                    RecoveryAction::Retry,
                    RecoveryAction::CheckConfiguration(format!(
                        "Verify {} service configuration.",
                        service
                    )),
                    RecoveryAction::ContactSupport(format!("Check {} service status.", service)),
                ]
            }

            // File I/O errors might be retryable
            FennecError::FileRead { .. } | FennecError::FileWrite { .. } => {
                vec![
                    RecoveryAction::Retry,
                    RecoveryAction::CheckPermissions("Verify file permissions.".to_string()),
                ]
            }

            // Most other errors suggest basic recovery
            _ => vec![
                RecoveryAction::Retry,
                RecoveryAction::ContactSupport(
                    "If the problem persists, check the documentation or contact support."
                        .to_string(),
                ),
            ],
        }
    }

    fn user_message(&self) -> String {
        match self {
            FennecError::ConfigNotFound { .. } => "Configuration file not found. Please create a configuration file or run 'fennec init'.".to_string(),
            FennecError::ConfigInvalid { issue, .. } => format!("Configuration is invalid: {}. Please check your settings.", issue),
            FennecError::FileNotFound { .. } => "The requested file could not be found. Please check the file path.".to_string(),
            FennecError::PermissionDenied { .. } => "Permission denied. Please check file permissions or run with appropriate privileges.".to_string(),
            FennecError::SessionLimitExceeded { .. } => "Too many active sessions. Please close some sessions before creating new ones.".to_string(),
            FennecError::WorkspaceNotFound { .. } => "Workspace directory not found. Please create the workspace or update your configuration.".to_string(),
            FennecError::ServiceUnavailable { service, .. } => format!("{} service is currently unavailable. Please try again later.", service),
            _ => "An error occurred while processing your request.".to_string(),
        }
    }

    fn debug_context(&self) -> Option<String> {
        match self {
            FennecError::ConfigLoadFailed { path, .. } => Some(format!("Config path: {}", path)),
            FennecError::ServiceInitFailed { service, reason } => {
                Some(format!("Service: {}, Reason: {}", service, reason))
            }
            FennecError::Unknown {
                source: Some(source),
                ..
            } => Some(format!("Source: {}", source)),
            _ => None,
        }
    }
}

/// Helper function to create user-friendly error from any error
pub fn user_friendly_error(error: &(dyn std::error::Error + 'static)) -> String {
    if let Some(fennec_error) = error.downcast_ref::<FennecError>() {
        fennec_error.user_message()
    } else {
        "An unexpected error occurred. Please try again.".to_string()
    }
}

/// Helper function to get recovery actions from any error
pub fn get_recovery_actions(error: &(dyn std::error::Error + 'static)) -> Vec<RecoveryAction> {
    if let Some(fennec_error) = error.downcast_ref::<FennecError>() {
        fennec_error.recovery_actions()
    } else {
        vec![RecoveryAction::Retry]
    }
}

/// Helper function to check if an error is retryable
pub fn is_retryable_error(error: &(dyn std::error::Error + 'static)) -> bool {
    if let Some(fennec_error) = error.downcast_ref::<FennecError>() {
        fennec_error.is_retryable()
    } else {
        false
    }
}

/// Display implementation for ErrorCategory
impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::User => write!(f, "User Error"),
            ErrorCategory::System => write!(f, "System Error"),
            ErrorCategory::Network => write!(f, "Network Error"),
            ErrorCategory::Security => write!(f, "Security Error"),
            ErrorCategory::Internal => write!(f, "Internal Error"),
        }
    }
}

/// Display implementation for ErrorSeverity
impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "Info"),
            ErrorSeverity::Warning => write!(f, "Warning"),
            ErrorSeverity::Error => write!(f, "Error"),
            ErrorSeverity::Critical => write!(f, "Critical"),
        }
    }
}

/// Display implementation for RecoveryAction
impl fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryAction::Retry => write!(f, "Try again"),
            RecoveryAction::RetryWithChanges(msg) => write!(f, "Try again: {}", msg),
            RecoveryAction::CheckConfiguration(msg) => write!(f, "Check configuration: {}", msg),
            RecoveryAction::CheckPermissions(msg) => write!(f, "Check permissions: {}", msg),
            RecoveryAction::ContactSupport(msg) => write!(f, "Contact support: {}", msg),
            RecoveryAction::ManualAction(msg) => write!(f, "Manual action required: {}", msg),
            RecoveryAction::None => write!(f, "No recovery available"),
        }
    }
}
