//! Error types for AG-UI core operations.

use thiserror::Error;

/// Errors that can occur in AG-UI core operations.
#[derive(Debug, Error)]
pub enum AgUiError {
    /// Error during JSON serialization/deserialization
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Validation error for event or message data
    #[error("Validation error: {0}")]
    Validation(String),

    /// Invalid event format or structure
    #[error("Invalid event: {0}")]
    InvalidEvent(String),

    /// Invalid message format or content
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// State operation error
    #[error("State error: {0}")]
    State(String),
}

/// Result type alias using AgUiError
pub type Result<T> = std::result::Result<T, AgUiError>;
