use fennec_core::error::{ErrorCategory, ErrorInfo, ErrorSeverity, RecoveryAction};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ProviderError>;

#[derive(Error, Debug)]
pub enum ProviderError {
    // Network and HTTP errors
    #[error("HTTP request failed: {operation} - {source}")]
    Http {
        operation: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Network connection failed: {endpoint} - {reason}")]
    ConnectionFailed { endpoint: String, reason: String },

    #[error("Network timeout: {operation} exceeded {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },

    #[error("SSL/TLS error: {details}")]
    TlsError { details: String },

    // Authentication and authorization errors
    #[error("Authentication failed: {provider} - {reason}")]
    AuthenticationFailed { provider: String, reason: String },

    #[error("API key invalid or missing for {provider}")]
    ApiKeyInvalid { provider: String },

    #[error("Authorization denied: {operation} requires {required_permission}")]
    AuthorizationDenied {
        operation: String,
        required_permission: String,
    },

    #[error("API key expired for {provider}. Please renew your subscription")]
    ApiKeyExpired { provider: String },

    // Rate limiting and quotas
    #[error("Rate limit exceeded for {provider}: {message}. Retry after {retry_after}s")]
    RateLimit {
        provider: String,
        message: String,
        retry_after: u64,
        daily_limit: Option<u64>,
        current_usage: Option<u64>,
    },

    #[error("Monthly quota exceeded for {provider}: {used}/{limit} requests")]
    QuotaExceeded {
        provider: String,
        used: u64,
        limit: u64,
        reset_date: String,
    },

    #[error("Token limit exceeded: {used}/{limit} tokens. {suggestion}")]
    TokenLimit {
        used: u64,
        limit: u64,
        suggestion: String,
    },

    // Request validation errors
    #[error("Invalid request: {field} - {issue}")]
    InvalidRequest { field: String, issue: String },

    #[error("Request too large: {size} bytes exceeds limit of {limit} bytes")]
    RequestTooLarge { size: usize, limit: usize },

    #[error("Unsupported content type: {content_type}. Supported: {supported}")]
    UnsupportedContentType {
        content_type: String,
        supported: String,
    },

    #[error("Missing required parameter: {parameter}")]
    MissingParameter { parameter: String },

    // Model and capability errors
    #[error("Model '{model}' not found or not available")]
    ModelNotFound { model: String },

    #[error("Model '{model}' does not support capability: {capability}")]
    ModelCapabilityUnsupported { model: String, capability: String },

    #[error("Model '{model}' is currently unavailable: {reason}")]
    ModelUnavailable { model: String, reason: String },

    #[error("Model configuration invalid: {setting} = {value}")]
    ModelConfigInvalid { setting: String, value: String },

    // Server and service errors
    #[error("Provider server error: {provider} returned {status_code} - {message}")]
    ServerError {
        provider: String,
        status_code: u16,
        message: String,
        is_temporary: bool,
    },

    #[error("Service unavailable: {provider} is experiencing issues")]
    ServiceUnavailable { provider: String, reason: String },

    #[error("Service maintenance: {provider} is under maintenance until {until}")]
    ServiceMaintenance { provider: String, until: String },

    // Streaming and response errors
    #[error("Stream error: {operation} - {reason}")]
    StreamError { operation: String, reason: String },

    #[error("Response parsing failed: expected {expected}, got {actual}")]
    ResponseParsingFailed { expected: String, actual: String },

    #[error("Incomplete response: {received}/{expected} bytes")]
    IncompleteResponse { received: usize, expected: usize },

    #[error("Response format invalid: {details}")]
    InvalidResponseFormat { details: String },

    // Configuration errors
    #[error("Provider configuration missing: {provider} not configured")]
    ConfigurationMissing { provider: String },

    #[error("Provider configuration invalid: {provider} - {setting}: {issue}")]
    ConfigurationInvalid {
        provider: String,
        setting: String,
        issue: String,
    },

    #[error("Provider '{provider}' not supported. Available: {available}")]
    ProviderNotSupported { provider: String, available: String },

    // Content and safety errors
    #[error("Content filtered: {reason}")]
    ContentFiltered { reason: String },

    #[error("Content too large: {size} tokens exceeds {limit} token limit")]
    ContentTooLarge { size: usize, limit: usize },

    #[error("Content encoding error: {encoding} - {details}")]
    ContentEncodingError { encoding: String, details: String },

    // Serialization errors (wrapped for better context)
    #[error("JSON serialization failed: {operation} - {source}")]
    Json {
        operation: String,
        #[source]
        source: serde_json::Error,
    },

    // Generic and fallback errors
    #[error("Provider error: {message}")]
    Generic {
        message: String,
        provider: String,
        context: Option<String>,
    },
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool {
        match self {
            ProviderError::RateLimit { .. } => true,
            ProviderError::ServerError {
                is_temporary: true, ..
            } => true,
            ProviderError::Timeout { .. } => true,
            ProviderError::Http { .. } => true,
            ProviderError::ConnectionFailed { .. } => true,
            ProviderError::ServiceUnavailable { .. } => true,
            ProviderError::StreamError { .. } => true,
            ProviderError::IncompleteResponse { .. } => true,
            _ => false,
        }
    }

    pub fn retry_after(&self) -> Option<u64> {
        match self {
            ProviderError::RateLimit { retry_after, .. } => Some(*retry_after),
            ProviderError::ServerError {
                is_temporary: true, ..
            } => Some(1),
            ProviderError::ServiceUnavailable { .. } => Some(30),
            ProviderError::Timeout { .. } => Some(5),
            _ => None,
        }
    }

    pub fn should_exponential_backoff(&self) -> bool {
        match self {
            ProviderError::RateLimit { .. }
            | ProviderError::ServerError { .. }
            | ProviderError::ServiceUnavailable { .. } => true,
            _ => false,
        }
    }
}

impl ErrorInfo for ProviderError {
    fn category(&self) -> ErrorCategory {
        match self {
            // User errors - configuration and request issues
            ProviderError::InvalidRequest { .. }
            | ProviderError::MissingParameter { .. }
            | ProviderError::ConfigurationMissing { .. }
            | ProviderError::ConfigurationInvalid { .. }
            | ProviderError::ProviderNotSupported { .. }
            | ProviderError::ModelNotFound { .. }
            | ProviderError::ModelCapabilityUnsupported { .. }
            | ProviderError::ModelConfigInvalid { .. }
            | ProviderError::UnsupportedContentType { .. }
            | ProviderError::RequestTooLarge { .. }
            | ProviderError::ContentTooLarge { .. }
            | ProviderError::TokenLimit { .. } => ErrorCategory::User,

            // Security errors
            ProviderError::AuthenticationFailed { .. }
            | ProviderError::ApiKeyInvalid { .. }
            | ProviderError::AuthorizationDenied { .. }
            | ProviderError::ApiKeyExpired { .. }
            | ProviderError::ContentFiltered { .. } => ErrorCategory::Security,

            // Network errors
            ProviderError::Http { .. }
            | ProviderError::ConnectionFailed { .. }
            | ProviderError::Timeout { .. }
            | ProviderError::TlsError { .. }
            | ProviderError::RateLimit { .. }
            | ProviderError::QuotaExceeded { .. }
            | ProviderError::ServerError { .. }
            | ProviderError::ServiceUnavailable { .. }
            | ProviderError::ServiceMaintenance { .. } => ErrorCategory::Network,

            // Internal errors
            ProviderError::StreamError { .. }
            | ProviderError::ResponseParsingFailed { .. }
            | ProviderError::IncompleteResponse { .. }
            | ProviderError::InvalidResponseFormat { .. }
            | ProviderError::ModelUnavailable { .. }
            | ProviderError::ContentEncodingError { .. }
            | ProviderError::Json { .. }
            | ProviderError::Generic { .. } => ErrorCategory::Internal,
        }
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            // Critical errors that prevent service operation
            ProviderError::AuthenticationFailed { .. }
            | ProviderError::ApiKeyInvalid { .. }
            | ProviderError::ApiKeyExpired { .. }
            | ProviderError::ConfigurationMissing { .. } => ErrorSeverity::Critical,

            // Errors that prevent current operation
            ProviderError::QuotaExceeded { .. }
            | ProviderError::ServiceMaintenance { .. }
            | ProviderError::ModelNotFound { .. }
            | ProviderError::AuthorizationDenied { .. } => ErrorSeverity::Error,

            // Warnings for temporary or recoverable issues
            ProviderError::RateLimit { .. }
            | ProviderError::Timeout { .. }
            | ProviderError::ServiceUnavailable { .. }
            | ProviderError::ModelUnavailable { .. }
            | ProviderError::IncompleteResponse { .. } => ErrorSeverity::Warning,

            // Standard errors
            _ => ErrorSeverity::Error,
        }
    }

    fn recovery_actions(&self) -> Vec<RecoveryAction> {
        match self {
            ProviderError::ApiKeyInvalid { provider } => {
                vec![
                    RecoveryAction::CheckConfiguration(format!(
                        "Set valid API key for {}",
                        provider
                    )),
                    RecoveryAction::ContactSupport(format!("Verify {} account status", provider)),
                ]
            }

            ProviderError::ApiKeyExpired { provider } => {
                vec![
                    RecoveryAction::ManualAction(format!("Renew {} subscription", provider)),
                    RecoveryAction::CheckConfiguration(
                        "Update API key with renewed subscription".to_string(),
                    ),
                ]
            }

            ProviderError::RateLimit { retry_after, .. } => {
                vec![
                    RecoveryAction::RetryWithChanges(format!(
                        "Wait {} seconds before retrying",
                        retry_after
                    )),
                    RecoveryAction::CheckConfiguration(
                        "Consider upgrading to higher rate limits".to_string(),
                    ),
                ]
            }

            ProviderError::QuotaExceeded { reset_date, .. } => {
                vec![
                    RecoveryAction::ManualAction(format!(
                        "Wait until quota resets on {}",
                        reset_date
                    )),
                    RecoveryAction::CheckConfiguration(
                        "Upgrade plan for higher quotas".to_string(),
                    ),
                ]
            }

            ProviderError::TokenLimit { suggestion, .. } => {
                vec![
                    RecoveryAction::RetryWithChanges(suggestion.clone()),
                    RecoveryAction::ManualAction(
                        "Reduce input size or split into smaller requests".to_string(),
                    ),
                ]
            }

            ProviderError::ModelNotFound { .. } => {
                vec![
                    RecoveryAction::RetryWithChanges("Use a supported model".to_string()),
                    RecoveryAction::CheckConfiguration(
                        "Check available models for your provider".to_string(),
                    ),
                ]
            }

            ProviderError::ConfigurationMissing { provider } => {
                vec![
                    RecoveryAction::CheckConfiguration(format!(
                        "Configure {} provider settings",
                        provider
                    )),
                    RecoveryAction::ManualAction(
                        "Run setup wizard or check documentation".to_string(),
                    ),
                ]
            }

            ProviderError::ServiceUnavailable { .. } => {
                vec![
                    RecoveryAction::Retry,
                    RecoveryAction::RetryWithChanges("Try a different provider".to_string()),
                    RecoveryAction::ContactSupport("Check provider status page".to_string()),
                ]
            }

            ProviderError::ServiceMaintenance { until, .. } => {
                vec![
                    RecoveryAction::ManualAction(format!(
                        "Wait until maintenance completes ({})",
                        until
                    )),
                    RecoveryAction::RetryWithChanges(
                        "Use alternative provider temporarily".to_string(),
                    ),
                ]
            }

            ProviderError::InvalidRequest { field, issue } => {
                vec![RecoveryAction::RetryWithChanges(format!(
                    "Fix {}: {}",
                    field, issue
                ))]
            }

            ProviderError::Timeout { .. } => {
                vec![
                    RecoveryAction::Retry,
                    RecoveryAction::RetryWithChanges("Increase timeout limit".to_string()),
                    RecoveryAction::CheckConfiguration("Check network connection".to_string()),
                ]
            }

            // Most network errors can be retried
            ProviderError::Http { .. }
            | ProviderError::ConnectionFailed { .. }
            | ProviderError::StreamError { .. } => {
                vec![
                    RecoveryAction::Retry,
                    RecoveryAction::CheckConfiguration("Check network connectivity".to_string()),
                ]
            }

            // Default recovery actions
            _ => vec![
                RecoveryAction::Retry,
                RecoveryAction::ContactSupport(
                    "Check provider documentation if the problem persists".to_string(),
                ),
            ],
        }
    }

    fn user_message(&self) -> String {
        match self {
            ProviderError::ApiKeyInvalid { provider } => format!(
                "Invalid API key for {}. Please check your configuration.",
                provider
            ),
            ProviderError::ApiKeyExpired { provider } => format!(
                "{} subscription expired. Please renew your subscription.",
                provider
            ),
            ProviderError::RateLimit { provider, .. } => format!(
                "{} rate limit exceeded. Please wait before trying again.",
                provider
            ),
            ProviderError::QuotaExceeded { provider, .. } => format!(
                "{} monthly quota exceeded. Please upgrade your plan or wait for reset.",
                provider
            ),
            ProviderError::TokenLimit { .. } => {
                "Request too long. Please reduce the size of your input.".to_string()
            }
            ProviderError::ModelNotFound { .. } => {
                "AI model not available. Please select a different model.".to_string()
            }
            ProviderError::ServiceUnavailable { provider, .. } => format!(
                "{} service is temporarily unavailable. Please try again later.",
                provider
            ),
            ProviderError::ServiceMaintenance { provider, .. } => {
                format!("{} is under maintenance. Please try again later.", provider)
            }
            ProviderError::Timeout { .. } => {
                "Request timed out. Please check your connection and try again.".to_string()
            }
            ProviderError::ConfigurationMissing { provider } => format!(
                "{} is not configured. Please set up your provider configuration.",
                provider
            ),
            ProviderError::InvalidRequest { .. } => {
                "Invalid request. Please check your input and try again.".to_string()
            }
            ProviderError::ContentFiltered { reason } => {
                format!("Content blocked: {}. Please modify your request.", reason)
            }
            _ => "AI service error. Please try again or contact support.".to_string(),
        }
    }

    fn debug_context(&self) -> Option<String> {
        match self {
            ProviderError::ServerError {
                provider,
                status_code,
                message,
                ..
            } => Some(format!(
                "Provider: {}, Status: {}, Message: {}",
                provider, status_code, message
            )),
            ProviderError::RateLimit {
                current_usage: Some(usage),
                daily_limit: Some(limit),
                ..
            } => Some(format!("Usage: {}/{}", usage, limit)),
            ProviderError::TokenLimit { used, limit, .. } => {
                Some(format!("Tokens: {}/{}", used, limit))
            }
            ProviderError::Generic {
                context: Some(context),
                ..
            } => Some(context.clone()),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ProviderError {
    fn from(err: reqwest::Error) -> Self {
        ProviderError::Http {
            operation: "HTTP request".to_string(),
            source: err,
        }
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(err: serde_json::Error) -> Self {
        ProviderError::Json {
            operation: "JSON processing".to_string(),
            source: err,
        }
    }
}

impl From<ProviderError> for fennec_core::FennecError {
    fn from(err: ProviderError) -> Self {
        fennec_core::FennecError::Provider(Box::new(err))
    }
}

/// Helper functions for creating common provider errors
pub fn api_key_invalid(provider: &str) -> ProviderError {
    ProviderError::ApiKeyInvalid {
        provider: provider.to_string(),
    }
}

pub fn rate_limit_exceeded(provider: &str, message: &str, retry_after: u64) -> ProviderError {
    ProviderError::RateLimit {
        provider: provider.to_string(),
        message: message.to_string(),
        retry_after,
        daily_limit: None,
        current_usage: None,
    }
}

pub fn model_not_found(model: &str) -> ProviderError {
    ProviderError::ModelNotFound {
        model: model.to_string(),
    }
}

pub fn token_limit_exceeded(used: u64, limit: u64, suggestion: &str) -> ProviderError {
    ProviderError::TokenLimit {
        used,
        limit,
        suggestion: suggestion.to_string(),
    }
}

pub fn service_unavailable(provider: &str, reason: &str) -> ProviderError {
    ProviderError::ServiceUnavailable {
        provider: provider.to_string(),
        reason: reason.to_string(),
    }
}
