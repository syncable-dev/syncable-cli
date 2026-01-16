//! API response types for the Syncable Platform API
//!
//! These types mirror the backend DTOs for organizations, projects, and related entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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

/// Cloud provider types supported by the platform
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Gcp,
    Aws,
    Azure,
    Hetzner,
}

impl CloudProvider {
    /// Returns the lowercase string identifier for this provider
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudProvider::Gcp => "gcp",
            CloudProvider::Aws => "aws",
            CloudProvider::Azure => "azure",
            CloudProvider::Hetzner => "hetzner",
        }
    }

    /// Returns the human-readable display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            CloudProvider::Gcp => "Google Cloud Platform",
            CloudProvider::Aws => "Amazon Web Services",
            CloudProvider::Azure => "Microsoft Azure",
            CloudProvider::Hetzner => "Hetzner Cloud",
        }
    }

    /// Returns all supported cloud providers
    pub fn all() -> &'static [CloudProvider] {
        &[
            CloudProvider::Gcp,
            CloudProvider::Aws,
            CloudProvider::Azure,
            CloudProvider::Hetzner,
        ]
    }
}

impl fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for CloudProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gcp" | "google" | "google-cloud" => Ok(CloudProvider::Gcp),
            "aws" | "amazon" => Ok(CloudProvider::Aws),
            "azure" | "microsoft" => Ok(CloudProvider::Azure),
            "hetzner" => Ok(CloudProvider::Hetzner),
            _ => Err(format!(
                "Unknown cloud provider: '{}'. Valid options: gcp, aws, azure, hetzner",
                s
            )),
        }
    }
}

/// Minimal credential info (no secrets - just connection status)
///
/// SECURITY NOTE: This type intentionally contains only non-sensitive metadata.
/// Actual credentials (OAuth tokens, API keys, etc.) are NEVER exposed through
/// this API. The agent only needs to know IF a provider is connected, not the
/// actual credential values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudCredentialStatus {
    /// Unique identifier for this credential record
    pub id: String,
    /// The cloud provider this credential is for (lowercase: gcp, aws, azure, hetzner)
    pub provider: String,
    // NOTE: Never include tokens/secrets here - this is intentionally minimal
}

// =============================================================================
// Deployment Types
// =============================================================================

/// Deployment configuration for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentConfig {
    /// Unique identifier for this deployment config
    pub id: String,
    /// The project this config belongs to
    pub project_id: String,
    /// Repository ID (from GitHub/GitLab integration)
    pub repository_id: i64,
    /// Full repository name (e.g., "owner/repo")
    pub repository_full_name: String,
    /// Name of the service being deployed
    pub service_name: String,
    /// Environment ID for deployment
    pub environment_id: String,
    /// Target type: "kubernetes" or "cloud_runner"
    pub target_type: Option<String>,
    /// Branch to deploy from
    pub branch: String,
    /// Port the service listens on
    pub port: i32,
    /// Whether auto-deploy on push is enabled
    pub auto_deploy_enabled: bool,
    /// Deployment strategy (e.g., "rolling", "blue_green")
    pub deployment_strategy: Option<String>,
    /// When this config was created
    pub created_at: DateTime<Utc>,
}

/// Request to trigger deployment
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerDeploymentRequest {
    /// Project ID for the deployment
    pub project_id: String,
    /// Deployment config ID to use
    pub config_id: String,
    /// Optional specific commit SHA to deploy (defaults to latest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}

/// Response from triggering a deployment
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerDeploymentResponse {
    /// The deployment config ID used
    pub config_id: String,
    /// Task ID to track deployment progress
    pub backstage_task_id: String,
    /// Initial status of the deployment
    pub status: String,
    /// Human-readable message about the deployment
    pub message: String,
}

/// Deployment task status
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentTaskStatus {
    /// Task status: "processing", "completed", "failed"
    pub status: String,
    /// Progress percentage (0-100)
    pub progress: i32,
    /// Current step description
    pub current_step: Option<String>,
    /// Overall deployment status: "generating", "building", "deploying", "healthy", "failed"
    pub overall_status: String,
    /// Human-readable overall message
    pub overall_message: String,
    /// Error message if deployment failed
    pub error: Option<String>,
}

/// Deployed service info
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployedService {
    /// Unique deployment ID
    pub id: String,
    /// Project this deployment belongs to
    pub project_id: String,
    /// Name of the deployed service
    pub service_name: String,
    /// Full repository name
    pub repository_full_name: String,
    /// Deployment status
    pub status: String,
    /// Task ID used for this deployment
    pub backstage_task_id: Option<String>,
    /// Commit SHA that was deployed
    pub commit_sha: Option<String>,
    /// Public URL of the deployed service
    pub public_url: Option<String>,
    /// When this deployment was created
    pub created_at: DateTime<Utc>,
}

/// Paginated list of deployments
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedDeployments {
    /// List of deployments
    pub data: Vec<DeployedService>,
    /// Pagination info
    pub pagination: PaginationInfo,
}

/// Pagination information for list responses
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfo {
    /// Cursor for next page (if any)
    pub next_cursor: Option<String>,
    /// Whether there are more results
    pub has_more: bool,
}

// =============================================================================
// Log Types
// =============================================================================

/// A single log entry from a container
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// ISO timestamp when log was generated
    pub timestamp: String,
    /// Log message content
    pub message: String,
    /// Container metadata labels
    pub labels: std::collections::HashMap<String, String>,
}

/// Statistics about the log query
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogQueryStats {
    /// Number of log entries returned
    pub entries_returned: i32,
    /// Time taken to execute query in milliseconds
    pub query_time_ms: i64,
}

/// Response from log query endpoint
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLogsResponse {
    /// Log entries
    pub data: Vec<LogEntry>,
    /// Query statistics
    pub stats: LogQueryStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_provider_as_str() {
        assert_eq!(CloudProvider::Gcp.as_str(), "gcp");
        assert_eq!(CloudProvider::Aws.as_str(), "aws");
        assert_eq!(CloudProvider::Azure.as_str(), "azure");
        assert_eq!(CloudProvider::Hetzner.as_str(), "hetzner");
    }

    #[test]
    fn test_cloud_provider_display_name() {
        assert_eq!(CloudProvider::Gcp.display_name(), "Google Cloud Platform");
        assert_eq!(CloudProvider::Aws.display_name(), "Amazon Web Services");
        assert_eq!(CloudProvider::Azure.display_name(), "Microsoft Azure");
        assert_eq!(CloudProvider::Hetzner.display_name(), "Hetzner Cloud");
    }

    #[test]
    fn test_cloud_provider_from_str() {
        assert_eq!(CloudProvider::from_str("gcp").unwrap(), CloudProvider::Gcp);
        assert_eq!(CloudProvider::from_str("GCP").unwrap(), CloudProvider::Gcp);
        assert_eq!(CloudProvider::from_str("aws").unwrap(), CloudProvider::Aws);
        assert_eq!(
            CloudProvider::from_str("azure").unwrap(),
            CloudProvider::Azure
        );
        assert_eq!(
            CloudProvider::from_str("hetzner").unwrap(),
            CloudProvider::Hetzner
        );
        assert!(CloudProvider::from_str("unknown").is_err());
    }

    #[test]
    fn test_cloud_provider_display() {
        assert_eq!(format!("{}", CloudProvider::Gcp), "gcp");
        assert_eq!(format!("{}", CloudProvider::Aws), "aws");
    }

    #[test]
    fn test_cloud_provider_all() {
        let all = CloudProvider::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&CloudProvider::Gcp));
        assert!(all.contains(&CloudProvider::Aws));
        assert!(all.contains(&CloudProvider::Azure));
        assert!(all.contains(&CloudProvider::Hetzner));
    }

    #[test]
    fn test_cloud_credential_status_serialization() {
        let status = CloudCredentialStatus {
            id: "cred-123".to_string(),
            provider: "gcp".to_string(),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"id\":\"cred-123\""));
        assert!(json.contains("\"provider\":\"gcp\""));
        // Verify no tokens/secrets in serialized output
        assert!(!json.contains("token"));
        assert!(!json.contains("secret"));
        assert!(!json.contains("key"));
    }
}
