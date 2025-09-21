use thiserror::Error;

pub type Result<T> = std::result::Result<T, FennecError>;

#[derive(Error, Debug)]
pub enum FennecError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Provider error: {message}")]
    Provider { message: String },

    #[error("Command error: {message}")]
    Command { message: String },

    #[error("Security error: {message}")]
    Security { message: String },

    #[error("Memory error: {message}")]
    Memory { message: String },

    #[error("TUI error: {message}")]
    Tui { message: String },

    #[error("Session error: {message}")]
    Session { message: String },

    #[error("Unknown error: {message}")]
    Unknown { message: String },
}