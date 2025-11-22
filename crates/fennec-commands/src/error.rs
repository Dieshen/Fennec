use fennec_core::error::{ErrorCategory, ErrorInfo, ErrorSeverity, RecoveryAction};
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
    MemoryService(Box<dyn std::error::Error + Send + Sync>),

    #[error("Provider service error: {0}")]
    ProviderService(Box<dyn std::error::Error + Send + Sync>),

    #[error("Security service error: {0}")]
    SecurityService(Box<dyn std::error::Error + Send + Sync>),

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

            CommandError::ResourceLimitExceeded {
                resource,
                current,
                maximum,
            } => {
                vec![
                    RecoveryAction::ManualAction(format!(
                        "Reduce {} usage (current: {}, limit: {})",
                        resource, current, maximum
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_argument_error() {
        let err = CommandError::InvalidArgument {
            arg: "depth".to_string(),
            reason: "must be a number".to_string(),
            expected: "1-10".to_string(),
        };
        assert!(err.to_string().contains("Invalid argument"));
        assert!(err.to_string().contains("depth"));
        assert_eq!(err.category(), ErrorCategory::User);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_missing_argument_error() {
        let err = CommandError::MissingArgument {
            arg: "file".to_string(),
            description: "path to file".to_string(),
        };
        assert!(err.to_string().contains("Missing required argument"));
        assert!(err.user_message().contains("Missing required argument"));
    }

    #[test]
    fn test_invalid_argument_combination_error() {
        let err = CommandError::InvalidArgumentCombination {
            args: "--all and --file".to_string(),
            suggestion: "use only one".to_string(),
        };
        assert!(err.to_string().contains("Invalid argument combination"));
    }

    #[test]
    fn test_argument_out_of_range_error() {
        let err = CommandError::ArgumentOutOfRange {
            arg: "depth".to_string(),
            value: "100".to_string(),
            min: "1".to_string(),
            max: "10".to_string(),
        };
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn test_execution_failed_error() {
        let err = CommandError::ExecutionFailed {
            reason: "command not found".to_string(),
            command: "test".to_string(),
            exit_code: Some(127),
        };
        assert!(err.to_string().contains("execution failed"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert!(err.debug_context().unwrap().contains("Exit code"));
    }

    #[test]
    fn test_timeout_error() {
        let err = CommandError::Timeout {
            timeout_ms: 5000,
            command: "long-running".to_string(),
        };
        assert!(err.to_string().contains("timed out"));
        assert!(err.to_string().contains("5000"));
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_cancelled_error() {
        let err = CommandError::Cancelled {
            command: "edit".to_string(),
            reason: "user aborted".to_string(),
        };
        assert!(err.to_string().contains("cancelled"));
        assert_eq!(err.severity(), ErrorSeverity::Info);
    }

    #[test]
    fn test_preview_failed_error() {
        let err = CommandError::PreviewFailed {
            reason: "file too large".to_string(),
            command: "diff".to_string(),
        };
        assert!(err.to_string().contains("Preview generation failed"));
    }

    #[test]
    fn test_file_not_found_error() {
        let err = CommandError::FileNotFound {
            path: "/path/to/file.rs".to_string(),
            operation: "read".to_string(),
        };
        assert!(err.to_string().contains("File not found"));
        assert_eq!(err.category(), ErrorCategory::User);
    }

    #[test]
    fn test_permission_denied_error() {
        let err = CommandError::PermissionDenied {
            path: "/root/secret".to_string(),
            operation: "write".to_string(),
            required_permission: "write".to_string(),
        };
        assert!(err.to_string().contains("Permission denied"));
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_directory_not_found_error() {
        let err = CommandError::DirectoryNotFound {
            path: "/missing/dir".to_string(),
            operation: "list".to_string(),
        };
        assert!(err.to_string().contains("Directory not found"));
    }

    #[test]
    fn test_file_too_large_error() {
        let err = CommandError::FileTooLarge {
            path: "huge.bin".to_string(),
            size_mb: 500,
            max_size_mb: 100,
        };
        assert!(err.to_string().contains("too large"));
        assert!(err.to_string().contains("500"));
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_unsupported_file_type_error() {
        let err = CommandError::UnsupportedFileType {
            path: "file.xyz".to_string(),
            extension: "xyz".to_string(),
            supported: "rs, toml, md".to_string(),
        };
        assert!(err.to_string().contains("Unsupported file type"));
    }

    #[test]
    fn test_sandbox_violation_error() {
        let err = CommandError::SandboxViolation {
            action: "write to /etc".to_string(),
            level: "read-only".to_string(),
            required_level: "workspace-write".to_string(),
        };
        assert!(err.to_string().contains("Sandbox violation"));
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_approval_required_error() {
        let err = CommandError::ApprovalRequired {
            operation: "delete all".to_string(),
            risk_level: "high".to_string(),
            details: "destructive".to_string(),
        };
        assert!(err.to_string().contains("requires approval"));
    }

    #[test]
    fn test_security_denied_error() {
        let err = CommandError::SecurityDenied {
            operation: "rm -rf /".to_string(),
            reason: "dangerous".to_string(),
        };
        assert!(err.to_string().contains("denied by security policy"));
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_content_parsing_failed_error() {
        let err = CommandError::ContentParsingFailed {
            reason: "invalid JSON".to_string(),
            file_path: Some("data.json".to_string()),
            expected_format: "JSON".to_string(),
        };
        assert!(err.to_string().contains("parsing failed"));
    }

    #[test]
    fn test_content_generation_failed_error() {
        let err = CommandError::ContentGenerationFailed {
            reason: "template error".to_string(),
            operation: "generate".to_string(),
        };
        assert!(err.to_string().contains("Content generation failed"));
    }

    #[test]
    fn test_encoding_error() {
        let err = CommandError::EncodingError {
            reason: "invalid UTF-8".to_string(),
            file_path: "file.txt".to_string(),
            expected_encoding: "UTF-8".to_string(),
        };
        assert!(err.to_string().contains("encoding error"));
    }

    #[test]
    fn test_resource_limit_exceeded_error() {
        let err = CommandError::ResourceLimitExceeded {
            resource: "memory".to_string(),
            current: "500MB".to_string(),
            maximum: "256MB".to_string(),
        };
        assert!(err.to_string().contains("Resource limit exceeded"));
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_dependency_failed_error() {
        let err = CommandError::DependencyFailed {
            dependency: "git".to_string(),
            reason: "not found".to_string(),
            suggestion: "install git".to_string(),
        };
        assert!(err.to_string().contains("External dependency failed"));
        assert_eq!(err.category(), ErrorCategory::Network);
    }

    #[test]
    fn test_service_unavailable_error() {
        let err = CommandError::ServiceUnavailable {
            service: "API".to_string(),
            reason: "timeout".to_string(),
        };
        assert!(err.to_string().contains("Service unavailable"));
        assert_eq!(err.category(), ErrorCategory::Network);
    }

    #[test]
    fn test_memory_service_error() {
        let err = CommandError::MemoryService(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test error",
        )));
        assert!(err.to_string().contains("Memory service error"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_provider_service_error() {
        let err = CommandError::ProviderService(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test error",
        )));
        assert!(err.to_string().contains("Provider service error"));
        assert_eq!(err.category(), ErrorCategory::Network);
    }

    #[test]
    fn test_security_service_error() {
        let err = CommandError::SecurityService(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test error",
        )));
        assert!(err.to_string().contains("Security service error"));
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_io_error() {
        let err = CommandError::Io {
            operation: "read".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        };
        assert!(err.to_string().contains("IO operation failed"));
        assert!(err.debug_context().unwrap().contains("read"));
    }

    #[test]
    fn test_generic_error() {
        let err = CommandError::Generic {
            message: "something went wrong".to_string(),
            context: Some("during operation".to_string()),
        };
        assert!(err.to_string().contains("Command failed"));
        assert_eq!(err.debug_context().unwrap(), "during operation");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let cmd_err: CommandError = io_err.into();
        assert!(matches!(cmd_err, CommandError::Io { .. }));
    }

    #[test]
    fn test_fennec_error_conversion() {
        let cmd_err = CommandError::InvalidArgument {
            arg: "test".to_string(),
            reason: "invalid".to_string(),
            expected: "valid".to_string(),
        };
        let fennec_err: fennec_core::FennecError = cmd_err.into();
        assert!(matches!(fennec_err, fennec_core::FennecError::Command(_)));
    }

    #[test]
    fn test_recovery_actions_for_file_not_found() {
        let err = CommandError::FileNotFound {
            path: "test.rs".to_string(),
            operation: "read".to_string(),
        };
        let actions = err.recovery_actions();
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_recovery_actions_for_permission_denied() {
        let err = CommandError::PermissionDenied {
            path: "test.rs".to_string(),
            operation: "write".to_string(),
            required_permission: "write".to_string(),
        };
        let actions = err.recovery_actions();
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_helper_function_missing_argument() {
        let err = missing_argument("file", "path to file");
        assert!(matches!(err, CommandError::MissingArgument { .. }));
    }

    #[test]
    fn test_helper_function_invalid_argument() {
        let err = invalid_argument("depth", "not a number", "1-10");
        assert!(matches!(err, CommandError::InvalidArgument { .. }));
    }

    #[test]
    fn test_helper_function_file_not_found() {
        let err = file_not_found("test.rs", "read");
        assert!(matches!(err, CommandError::FileNotFound { .. }));
    }

    #[test]
    fn test_helper_function_permission_denied() {
        let err = permission_denied("test.rs", "write", "write");
        assert!(matches!(err, CommandError::PermissionDenied { .. }));
    }

    #[test]
    fn test_helper_function_sandbox_violation() {
        let err = sandbox_violation("write", "read-only", "workspace-write");
        assert!(matches!(err, CommandError::SandboxViolation { .. }));
    }

    #[test]
    fn test_helper_function_execution_failed() {
        let err = execution_failed("failed", "test", Some(1));
        assert!(matches!(err, CommandError::ExecutionFailed { .. }));
    }

    #[test]
    fn test_user_messages() {
        let errors = vec![
            CommandError::InvalidArgument {
                arg: "test".to_string(),
                reason: "invalid".to_string(),
                expected: "valid".to_string(),
            },
            CommandError::MissingArgument {
                arg: "file".to_string(),
                description: "path".to_string(),
            },
            CommandError::FileNotFound {
                path: "test.rs".to_string(),
                operation: "read".to_string(),
            },
            CommandError::PermissionDenied {
                path: "test.rs".to_string(),
                operation: "write".to_string(),
                required_permission: "write".to_string(),
            },
            CommandError::SandboxViolation {
                action: "write".to_string(),
                level: "read-only".to_string(),
                required_level: "workspace-write".to_string(),
            },
        ];

        for err in errors {
            let msg = err.user_message();
            assert!(!msg.is_empty());
        }
    }
}
