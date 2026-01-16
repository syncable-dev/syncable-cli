//! Error types for the Platform API client
//!
//! Provides structured error types for all API operations.

use thiserror::Error;

/// Errors that can occur when interacting with the Syncable Platform API
#[derive(Debug, Error)]
pub enum PlatformApiError {
    /// HTTP request failed (network error, timeout, etc.)
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error ({status}): {message}")]
    ApiError {
        /// HTTP status code
        status: u16,
        /// Error message from the API
        message: String,
    },

    /// Failed to parse the API response
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// User is not authenticated - needs to run `sync-ctl auth login`
    #[error("Not authenticated - run `sync-ctl auth login` first")]
    Unauthorized,

    /// Requested resource was not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// User does not have permission for the requested operation
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded - please try again later")]
    RateLimited,

    /// Server error
    #[error("Server error ({status}): {message}")]
    ServerError {
        /// HTTP status code (5xx)
        status: u16,
        /// Error message
        message: String,
    },
}

/// Result type alias for Platform API operations
pub type Result<T> = std::result::Result<T, PlatformApiError>;
