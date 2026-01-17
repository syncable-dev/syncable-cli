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

    /// Could not connect to the Syncable API
    #[error("Could not connect to Syncable API - check your internet connection")]
    ConnectionFailed,
}

impl PlatformApiError {
    /// Get a user-friendly suggestion for resolving this error
    ///
    /// Returns actionable advice that helps users fix the issue.
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            Self::Unauthorized => Some("Run `sync-ctl auth login` to authenticate"),
            Self::RateLimited => Some("Wait a moment and try again"),
            Self::HttpError(_) => Some("Check your internet connection"),
            Self::ServerError { .. } => {
                Some("The server is experiencing issues. Try again later")
            }
            Self::PermissionDenied(_) => {
                Some("Check your project permissions in the Syncable dashboard")
            }
            Self::NotFound(_) => Some("Verify the resource ID is correct"),
            Self::ParseError(_) => Some("This may be a bug - please report it"),
            Self::ApiError { status, .. } if *status >= 400 && *status < 500 => {
                Some("Check the request parameters")
            }
            Self::ConnectionFailed => {
                Some("Check your internet connection and try again")
            }
            _ => None,
        }
    }

    /// Format the error with suggestion if available
    ///
    /// Returns the error message followed by a suggestion on how to resolve it.
    pub fn with_suggestion(&self) -> String {
        match self.suggestion() {
            Some(suggestion) => format!("{}\n  → {}", self, suggestion),
            None => self.to_string(),
        }
    }
}

/// Result type alias for Platform API operations
pub type Result<T> = std::result::Result<T, PlatformApiError>;
