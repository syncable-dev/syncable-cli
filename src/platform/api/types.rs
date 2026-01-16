//! API response types for the Syncable Platform API
//!
//! These types mirror the backend DTOs for organizations, projects, and related entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Generic API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResponse<T> {
    /// The response data
    pub data: T,
}

/// Organization information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Unique organization identifier (UUID)
    pub id: String,
    /// Organization display name
    pub name: String,
    /// URL-friendly slug
    pub slug: String,
    /// Optional logo URL
    pub logo: Option<String>,
    /// When the organization was created
    pub created_at: DateTime<Utc>,
}

/// Project information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Unique project identifier (UUID)
    pub id: String,
    /// Project display name
    pub name: String,
    /// Project description
    pub description: String,
    /// Parent organization ID
    pub organization_id: String,
    /// When the project was created
    pub created_at: DateTime<Utc>,
    /// Project context/notes (optional)
    #[serde(default)]
    pub context: Option<String>,
}

/// Project member information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMember {
    /// User ID of the member
    pub user_id: String,
    /// Member's role in the project
    pub role: String,
}

/// Request body for creating a new project
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    /// ID of the user creating the project
    pub creator_id: String,
    /// Project name
    pub name: String,
    /// Project description
    pub description: String,
    /// Project context/notes
    #[serde(default)]
    pub context: String,
}

/// User profile information (from /api/users/me)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    /// User ID (UUID)
    pub id: String,
    /// User's email address
    pub email: String,
    /// User's display name
    pub name: Option<String>,
    /// Profile image URL
    pub image: Option<String>,
}

/// API error response format
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorResponse {
    /// Error message
    pub error: Option<String>,
    /// Detailed error message
    pub message: Option<String>,
}

impl ApiErrorResponse {
    /// Get the error message, preferring `message` over `error`
    pub fn get_message(&self) -> String {
        self.message
            .clone()
            .or_else(|| self.error.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}
