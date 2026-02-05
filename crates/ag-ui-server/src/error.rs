//! Error types for AG-UI server operations.

use ag_ui_core::AgUiError;
use thiserror::Error;

/// Errors that can occur in AG-UI server operations.
#[derive(Debug, Error)]
pub enum ServerError {
    /// Core AG-UI error
    #[error("Core error: {0}")]
    Core(#[from] AgUiError),

    /// Transport layer error (SSE, WebSocket, etc.)
    #[error("Transport error: {0}")]
    Transport(String),

    /// Serialization error during event emission
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Channel or stream error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),
}

/// Result type alias using ServerError
pub type Result<T> = std::result::Result<T, ServerError>;
