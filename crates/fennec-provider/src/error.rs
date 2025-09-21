use thiserror::Error;

pub type Result<T> = std::result::Result<T, ProviderError>;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    #[error("Rate limit exceeded: {message}, retry after: {retry_after:?}s")]
    RateLimit {
        message: String,
        retry_after: Option<u64>,
    },

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Token limit exceeded: {message}")]
    TokenLimit { message: String },

    #[error("Server error: {status_code}, message: {message}")]
    ServerError { status_code: u16, message: String },

    #[error("Network timeout: {message}")]
    Timeout { message: String },

    #[error("Stream error: {message}")]
    Stream { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Unknown provider error: {message}")]
    Unknown { message: String },
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool {
        match self {
            ProviderError::RateLimit { .. } => true,
            ProviderError::ServerError { status_code, .. } => *status_code >= 500,
            ProviderError::Timeout { .. } => true,
            ProviderError::Http(_) => true,
            _ => false,
        }
    }

    pub fn retry_after(&self) -> Option<u64> {
        match self {
            ProviderError::RateLimit { retry_after, .. } => *retry_after,
            ProviderError::ServerError { status_code, .. } if *status_code >= 500 => Some(1),
            _ => None,
        }
    }
}

impl From<ProviderError> for fennec_core::FennecError {
    fn from(err: ProviderError) -> Self {
        fennec_core::FennecError::Provider {
            message: err.to_string(),
        }
    }
}
