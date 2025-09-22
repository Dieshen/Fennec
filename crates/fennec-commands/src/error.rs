use fennec_core::{ErrorCategory, ErrorInfo, ErrorSeverity, RecoveryAction};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CommandError>;

#[derive(Error, Debug)]
pub enum CommandError {
    // Validation errors with specific guidance
    #[error("Invalid argument '{arg}': {reason}. Expected: {expected}")]
    InvalidArgument {
        arg: String,
        reason: String,
        expected: String,
    },

    #[error("Missing required argument: {arg}")]
    MissingArgument { arg: String, description: String },

    #[error("Invalid argument combination: {args}. {suggestion}")]
    InvalidArgumentCombination { args: String, suggestion: String },

    #[error("Argument value out of range: {arg} = {value}. Valid range: {min}-{max}")]
    ArgumentOutOfRange {
        arg: String,
        value: String,
        min: String,
        max: String,
    },

    // Execution errors with context
    #[error("Command execution failed: {reason}")]
    ExecutionFailed {
        reason: String,
        command: String,
        exit_code: Option<i32>,
    },

    #[error("Command timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64, command: String },

    #[error("Command cancelled by user")]
    Cancelled { command: String, reason: String },

    #[error("Preview generation failed: {reason}")]
    PreviewFailed { reason: String, command: String },

    // File operation errors with actionable guidance
    #[error("File not found: '{path}'. Please check if the file exists")]
    FileNotFound { path: String, operation: String },

    #[error("Permission denied: '{path}'. Please check file permissions")]
    PermissionDenied {
        path: String,
        operation: String,
        required_permission: String,
    },

    #[error("Directory not found: '{path}'. Please create the directory first")]
    DirectoryNotFound { path: String, operation: String },

    #[error("File is too large: '{path}' ({size_mb}MB). Maximum allowed: {max_size_mb}MB")]
    FileTooLarge {
        path: String,
        size_mb: u64,
        max_size_mb: u64,
    },

    #[error("Unsupported file type: '{path}' ({extension}). Supported types: {supported}")]
    UnsupportedFileType {
        path: String,
        extension: String,
        supported: String,
    },

    // Sandbox and security errors
    #[error("Sandbox violation: {action} not allowed in {level} mode. Use --sandbox-level {required_level} or request approval")]
    SandboxViolation {
        action: String,
        level: String,
        required_level: String,
    },

    #[error("Operation requires approval: {operation}. Risk level: {risk_level}")]
    ApprovalRequired {
        operation: String,
        risk_level: String,
        details: String,
    },

    #[error("Operation denied by security policy: {operation}")]
    SecurityDenied { operation: String, reason: String },

    // Content and processing errors
    #[error("Content parsing failed: {reason}. Please check file format")]
    ContentParsingFailed {
        reason: String,
        file_path: Option<String>,
        expected_format: String,
    },

    #[error("Content generation failed: {reason}")]
    ContentGenerationFailed { reason: String, operation: String },

    #[error("Text encoding error: {reason}. Please check file encoding")]
    EncodingError {
        reason: String,
        file_path: String,
        expected_encoding: String,
    },

    // Resource and dependency errors
    #[error("Resource limit exceeded: {resource} - current: {current}, maximum: {maximum}")]
    ResourceLimitExceeded {
        resource: String,
        current: String,
        maximum: String,
    },

    #[error("External dependency failed: {dependency} - {reason}")]
    DependencyFailed {
        dependency: String,
        reason: String,
        suggestion: String,
    },

    #[error("Service unavailable: {service}. Please try again later")]
    ServiceUnavailable { service: String, reason: String },

    // Integration errors
    #[error("Memory service error: {0}")]
    MemoryService(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Provider service error: {0}")]
    ProviderService(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Security service error: {0}")]
    SecurityService(#[from] Box<dyn std::error::Error + Send + Sync>),

    // IO errors (wrapped for better context)
    #[error("IO operation failed: {operation} - {source}")]
    Io {
        operation: String,
        #[source]
        source: std::io::Error,
    },

    // Generic errors
    #[error("Command failed: {message}")]
    Generic {
        message: String,
        context: Option<String>,
    },
}

impl ErrorInfo for CommandError {
    fn category(&self) -> ErrorCategory {
        match self {
            // User input errors
            CommandError::InvalidArgument { .. }
            | CommandError::MissingArgument { .. }
            | CommandError::InvalidArgumentCombination { .. }
            | CommandError::ArgumentOutOfRange { .. }
            | CommandError::FileNotFound { .. }
            | CommandError::DirectoryNotFound { .. }
            | CommandError::UnsupportedFileType { .. }
            | CommandError::ContentParsingFailed { .. }
            | CommandError::EncodingError { .. } => ErrorCategory::User,

            // Security errors
            CommandError::SandboxViolation { .. }
            | CommandError::ApprovalRequired { .. }
            | CommandError::SecurityDenied { .. }
            | CommandError::PermissionDenied { .. }
            | CommandError::SecurityService(_) => ErrorCategory::Security,

            // System errors
            CommandError::ExecutionFailed { .. }
            | CommandError::Timeout { .. }
            | CommandError::FileTooLarge { .. }
            | CommandError::ResourceLimitExceeded { .. }
            | CommandError::Io { .. } => ErrorCategory::System,

            // Network/service errors
            CommandError::ServiceUnavailable { .. }
            | CommandError::DependencyFailed { .. }
            | CommandError::ProviderService(_) => ErrorCategory::Network,

            // Internal errors
            CommandError::Cancelled { .. }
            | CommandError::PreviewFailed { .. }
            | CommandError::ContentGenerationFailed { .. }
            | CommandError::MemoryService(_)
            | CommandError::Generic { .. } => ErrorCategory::Internal,
        }
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            // User errors are typically non-critical
            CommandError::InvalidArgument { .. }
            | CommandError::MissingArgument { .. }
            | CommandError::InvalidArgumentCombination { .. }
            | CommandError::ArgumentOutOfRange { .. }
            | CommandError::FileNotFound { .. }
            | CommandError::DirectoryNotFound { .. }
            | CommandError::UnsupportedFileType { .. }
            | CommandError::ContentParsingFailed { .. }
            | CommandError::EncodingError { .. } => ErrorSeverity::Error,

            // Security errors are important but not critical
            CommandError::SandboxViolation { .. }
            | CommandError::ApprovalRequired { .. }
            | CommandError::SecurityDenied { .. }
            | CommandError::PermissionDenied { .. } => ErrorSeverity::Error,

            // System resource issues can be critical
            CommandError::ResourceLimitExceeded { .. } => ErrorSeverity::Critical,
            CommandError::FileTooLarge { .. } => ErrorSeverity::Warning,

            // Execution issues range from error to critical
            CommandError::ExecutionFailed { .. } => ErrorSeverity::Error,
            CommandError::Timeout { .. } => ErrorSeverity::Warning,
            CommandError::Cancelled { .. } => ErrorSeverity::Info,

            // Service issues
            CommandError::ServiceUnavailable { .. } | CommandError::DependencyFailed { .. } => {
                ErrorSeverity::Error
            }

            // Internal and integration errors
            CommandError::PreviewFailed { .. } | CommandError::ContentGenerationFailed { .. } => {
                ErrorSeverity::Error
            }
            CommandError::MemoryService(_)
            | CommandError::ProviderService(_)
            | CommandError::SecurityService(_) => ErrorSeverity::Error,

            CommandError::Io { .. } => ErrorSeverity::Error,
            CommandError::Generic { .. } => ErrorSeverity::Error,
        }
    }

    fn recovery_actions(&self) -> Vec<RecoveryAction> {
        match self {
            CommandError::InvalidArgument { expected, .. } => {
                vec![
                    RecoveryAction::RetryWithChanges(format!("Use: {}", expected)),
                    RecoveryAction::ContactSupport(
                        "Check command documentation for valid arguments.".to_string(),
                    ),
                ]
            }

            CommandError::MissingArgument { arg, description } => {
                vec![RecoveryAction::RetryWithChanges(format!(
                    "Add required argument: {} ({})",
                    arg, description
                ))]
            }

            CommandError::InvalidArgumentCombination { suggestion, .. } => {
                vec![RecoveryAction::RetryWithChanges(suggestion.clone())]
            }

            CommandError::ArgumentOutOfRange { min, max, .. } => {
                vec![RecoveryAction::RetryWithChanges(format!(
                    "Use a value between {} and {}",
                    min, max
                ))]
            }

            CommandError::FileNotFound { path, .. } => {
                vec![
                    RecoveryAction::ManualAction(format!(
                        "Create file '{}' or check the path",
                        path
                    )),
                    RecoveryAction::CheckPermissions(
                        "Verify read permissions for the file path".to_string(),
                    ),
                ]
            }

            CommandError::PermissionDenied {
                required_permission,
                ..
            } => {
                vec![
                    RecoveryAction::CheckPermissions(format!(
                        "Grant {} permission",
                        required_permission
                    )),
                    RecoveryAction::ManualAction(
                        "Run with elevated privileges if necessary".to_string(),
                    ),
                ]
            }

            CommandError::DirectoryNotFound { path, .. } => {
                vec![RecoveryAction::ManualAction(format!(
                    "Create directory: mkdir -p '{}'",
                    path
                ))]
            }

            CommandError::FileTooLarge { max_size_mb, .. } => {
                vec![
                    RecoveryAction::RetryWithChanges(format!(
                        "Use a file smaller than {}MB",
                        max_size_mb
                    )),
                    RecoveryAction::CheckConfiguration(
                        "Increase file size limit in configuration".to_string(),
                    ),
                ]
            }

            CommandError::UnsupportedFileType { supported, .. } => {
                vec![RecoveryAction::RetryWithChanges(format!(
                    "Use one of these file types: {}",
                    supported
                ))]
            }

            CommandError::SandboxViolation { required_level, .. } => {
                vec![
                    RecoveryAction::RetryWithChanges(format!(
                        "Use --sandbox-level {}",
                        required_level
                    )),
                    RecoveryAction::ManualAction("Request approval for this operation".to_string()),
                ]
            }

            CommandError::ApprovalRequired { operation, .. } => {
                vec![
                    RecoveryAction::ManualAction(format!("Approve the {} operation", operation)),
                    RecoveryAction::RetryWithChanges("Use a lower-risk alternative".to_string()),
                ]
            }

            CommandError::ContentParsingFailed {
                expected_format, ..
            } => {
                vec![
                    RecoveryAction::CheckConfiguration(format!(
                        "Ensure file is in {} format",
                        expected_format
                    )),
                    RecoveryAction::ManualAction("Check file content and encoding".to_string()),
                ]
            }

            CommandError::EncodingError {
                expected_encoding, ..
            } => {
                vec![RecoveryAction::ManualAction(format!(
                    "Convert file to {} encoding",
                    expected_encoding
                ))]
            }

            CommandError::ResourceLimitExceeded { resource, maximum } => {
                vec![
                    RecoveryAction::ManualAction(format!(
                        "Reduce {} usage (current limit: {})",
                        resource, maximum
                    )),
                    RecoveryAction::CheckConfiguration("Increase resource limits".to_string()),
                ]
            }

            CommandError::ServiceUnavailable { service, .. } => {
                vec![
                    RecoveryAction::Retry,
                    RecoveryAction::CheckConfiguration(format!(
                        "Verify {} service configuration",
                        service
                    )),
                ]
            }

            CommandError::DependencyFailed { suggestion, .. } => {
                vec![
                    RecoveryAction::ManualAction(suggestion.clone()),
                    RecoveryAction::Retry,
                ]
            }

            CommandError::ExecutionFailed { .. } => {
                vec![
                    RecoveryAction::Retry,
                    RecoveryAction::CheckConfiguration("Verify command configuration".to_string()),
                ]
            }

            CommandError::Timeout { .. } => {
                vec![
                    RecoveryAction::RetryWithChanges("Increase timeout limit".to_string()),
                    RecoveryAction::Retry,
                ]
            }

            // Most other errors can be retried
            _ => vec![
                RecoveryAction::Retry,
                RecoveryAction::ContactSupport(
                    "If the problem persists, check documentation".to_string(),
                ),
            ],
        }
    }

    fn user_message(&self) -> String {
        match self {
            CommandError::InvalidArgument { reason, .. } => format!("Invalid argument: {}. Please check your input.", reason),
            CommandError::MissingArgument { description, .. } => format!("Missing required argument: {}.", description),
            CommandError::FileNotFound { .. } => "File not found. Please check the file path and try again.".to_string(),
            CommandError::PermissionDenied { .. } => "Permission denied. Please check file permissions or run with appropriate privileges.".to_string(),
            CommandError::SandboxViolation { action, .. } => format!("Action '{}' is not allowed in current sandbox mode. Please adjust sandbox settings or request approval.", action),
            CommandError::ApprovalRequired { operation, .. } => format!("Operation '{}' requires approval due to security policy.", operation),
            CommandError::FileTooLarge { .. } => "File is too large for processing. Please use a smaller file.".to_string(),
            CommandError::UnsupportedFileType { .. } => "Unsupported file type. Please use a supported file format.".to_string(),
            CommandError::ExecutionFailed { .. } => "Command execution failed. Please check your input and try again.".to_string(),
            CommandError::Timeout { .. } => "Operation timed out. Please try again or increase the timeout limit.".to_string(),
            CommandError::ServiceUnavailable { service, .. } => format!("{} service is currently unavailable. Please try again later.", service),
            _ => "An error occurred while executing the command. Please try again.".to_string(),
        }
    }

    fn debug_context(&self) -> Option<String> {
        match self {
            CommandError::ExecutionFailed {
                command, exit_code, ..
            } => Some(format!("Command: {}, Exit code: {:?}", command, exit_code)),
            CommandError::Timeout {
                command,
                timeout_ms,
            } => Some(format!("Command: {}, Timeout: {}ms", command, timeout_ms)),
            CommandError::Io { operation, .. } => Some(format!("IO operation: {}", operation)),
            CommandError::Generic {
                context: Some(context),
                ..
            } => Some(context.clone()),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError::Io {
            operation: "file operation".to_string(),
            source: err,
        }
    }
}

impl From<CommandError> for fennec_core::FennecError {
    fn from(err: CommandError) -> Self {
        fennec_core::FennecError::Command(Box::new(err))
    }
}

/// Helper function to create a validation error for missing arguments
pub fn missing_argument(arg: &str, description: &str) -> CommandError {
    CommandError::MissingArgument {
        arg: arg.to_string(),
        description: description.to_string(),
    }
}

/// Helper function to create a validation error for invalid arguments
pub fn invalid_argument(arg: &str, reason: &str, expected: &str) -> CommandError {
    CommandError::InvalidArgument {
        arg: arg.to_string(),
        reason: reason.to_string(),
        expected: expected.to_string(),
    }
}

/// Helper function to create a file not found error
pub fn file_not_found(path: &str, operation: &str) -> CommandError {
    CommandError::FileNotFound {
        path: path.to_string(),
        operation: operation.to_string(),
    }
}

/// Helper function to create a permission denied error
pub fn permission_denied(path: &str, operation: &str, required_permission: &str) -> CommandError {
    CommandError::PermissionDenied {
        path: path.to_string(),
        operation: operation.to_string(),
        required_permission: required_permission.to_string(),
    }
}

/// Helper function to create a sandbox violation error
pub fn sandbox_violation(action: &str, level: &str, required_level: &str) -> CommandError {
    CommandError::SandboxViolation {
        action: action.to_string(),
        level: level.to_string(),
        required_level: required_level.to_string(),
    }
}

/// Helper function to create an execution failed error
pub fn execution_failed(reason: &str, command: &str, exit_code: Option<i32>) -> CommandError {
    CommandError::ExecutionFailed {
        reason: reason.to_string(),
        command: command.to_string(),
        exit_code,
    }
}
