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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Gcp,
    Aws,
    Azure,
    Hetzner,
    Scaleway,
    Cyso,
}

impl CloudProvider {
    /// Returns the lowercase string identifier for this provider
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudProvider::Gcp => "gcp",
            CloudProvider::Aws => "aws",
            CloudProvider::Azure => "azure",
            CloudProvider::Hetzner => "hetzner",
            CloudProvider::Scaleway => "scaleway",
            CloudProvider::Cyso => "cyso",
        }
    }

    /// Returns the human-readable display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            CloudProvider::Gcp => "Google Cloud Platform",
            CloudProvider::Aws => "Amazon Web Services",
            CloudProvider::Azure => "Microsoft Azure",
            CloudProvider::Hetzner => "Hetzner Cloud",
            CloudProvider::Scaleway => "Scaleway",
            CloudProvider::Cyso => "Cyso Cloud",
        }
    }

    /// Returns all supported cloud providers
    pub fn all() -> &'static [CloudProvider] {
        &[
            CloudProvider::Gcp,
            CloudProvider::Hetzner,
            CloudProvider::Aws,
            CloudProvider::Azure,
            CloudProvider::Scaleway,
            CloudProvider::Cyso,
        ]
    }

    /// Returns whether this provider is currently available for deployment
    ///
    /// Returns `true` for GCP and Hetzner (currently supported).
    /// Returns `false` for AWS, Azure, Scaleway, Cyso (coming soon).
    pub fn is_available(&self) -> bool {
        matches!(self, CloudProvider::Gcp | CloudProvider::Hetzner)
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
            "scaleway" => Ok(CloudProvider::Scaleway),
            "cyso" | "cyso-cloud" => Ok(CloudProvider::Cyso),
            _ => Err(format!(
                "Unknown cloud provider: '{}'. Valid options: gcp, aws, azure, hetzner, scaleway, cyso",
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
// Environment Types
// =============================================================================

/// Environment entity for a project
///
/// Environments define deployment targets within a project.
/// Each deployment configuration is associated with an environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    /// Unique environment identifier (UUID)
    pub id: String,
    /// Environment display name (e.g., "production", "staging", "development")
    pub name: String,
    /// Parent project ID
    pub project_id: String,
    /// Environment type: "cluster" (K8s) or "cloud" (Cloud Runner)
    pub environment_type: String,
    /// Cluster ID (only for cluster type)
    #[serde(default)]
    pub cluster_id: Option<String>,
    /// Kubernetes namespace (only for cluster type)
    #[serde(default)]
    pub namespace: Option<String>,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether the environment is active
    #[serde(default = "default_true")]
    pub is_active: bool,
    /// When the environment was created
    #[serde(default)]
    pub created_at: Option<String>,
    /// When the environment was last updated
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_true() -> bool {
    true
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

/// Response from creating a deployment config
///
/// The API returns the config wrapped with a wasUpdated flag indicating
/// whether an existing config was updated or a new one was created.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentConfigResponse {
    /// The created or updated deployment config
    pub config: DeploymentConfig,
    /// Whether this was an update to an existing config (vs new creation)
    pub was_updated: bool,
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

// =============================================================================
// Cluster Types
// =============================================================================

/// K8s cluster entity from platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterEntity {
    /// Unique cluster identifier
    pub id: String,
    /// Cluster display name
    pub name: String,
    /// Cloud provider hosting the cluster
    pub provider: CloudProvider,
    /// Region where cluster is deployed
    pub region: String,
    /// Current cluster status
    pub status: ClusterStatus,
    /// Kubernetes version (if available)
    pub kubernetes_version: Option<String>,
    /// Number of nodes in the cluster (if available)
    pub node_count: Option<i32>,
    /// When the cluster was created
    pub created_at: String,
}

/// Status of a K8s cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterStatus {
    Provisioning,
    Running,
    Updating,
    Deleting,
    Error,
    #[serde(other)]
    Unknown,
}

impl ClusterStatus {
    /// Returns a human-readable display string for the status
    pub fn display(&self) -> &'static str {
        match self {
            ClusterStatus::Provisioning => "Provisioning",
            ClusterStatus::Running => "Running",
            ClusterStatus::Updating => "Updating",
            ClusterStatus::Deleting => "Deleting",
            ClusterStatus::Error => "Error",
            ClusterStatus::Unknown => "Unknown",
        }
    }
}

// =============================================================================
// Artifact Registry Types
// =============================================================================

/// Artifact registry for container images
///
/// This maps to the backend's ProvisionedArtifactRegistryDto
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRegistry {
    /// Unique registry identifier
    pub id: String,
    /// Registry display name
    pub name: String,
    /// Cloud provider hosting the registry
    #[serde(alias = "provider")]
    pub cloud_provider: CloudProvider,
    /// Region where registry is located
    pub region: String,
    /// URL to push/pull images
    pub registry_url: String,
    /// Current registry status
    pub status: RegistryStatus,
    /// When the registry was created (ISO 8601 format)
    #[serde(default)]
    pub created_at: Option<String>,
    /// When the registry was last updated
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl ArtifactRegistry {
    /// Get the cloud provider (for backwards compatibility)
    pub fn provider(&self) -> &CloudProvider {
        &self.cloud_provider
    }
}

/// Status of an artifact registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryStatus {
    Provisioning,
    Ready,
    Error,
    #[serde(other)]
    Unknown,
}

impl RegistryStatus {
    /// Returns a human-readable display string for the status
    pub fn display(&self) -> &'static str {
        match self {
            RegistryStatus::Provisioning => "Provisioning",
            RegistryStatus::Ready => "Ready",
            RegistryStatus::Error => "Error",
            RegistryStatus::Unknown => "Unknown",
        }
    }
}

/// Request to provision a new artifact registry
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRegistryRequest {
    /// Project ID for the registry
    pub project_id: String,
    /// Cluster ID to associate registry with
    pub cluster_id: String,
    /// Cluster name for display
    pub cluster_name: String,
    /// Name for the new registry
    pub registry_name: String,
    /// Cloud provider hosting the registry
    pub cloud_provider: String,
    /// Region for the registry
    pub region: String,
    /// GCP project ID (required for GCP provider)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_project_id: Option<String>,
}

/// Response from registry provisioning
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRegistryResponse {
    /// Task ID for tracking provisioning progress
    pub task_id: String,
    /// Initial status
    pub status: String,
    /// Human-readable message
    pub message: String,
    /// Registry name (if immediately available)
    pub registry_name: Option<String>,
    /// Registry URL (if immediately available)
    pub registry_url: Option<String>,
    /// Cloud provider
    pub cloud_provider: String,
    /// When the task was created
    pub created_at: String,
}

/// Task status when polling registry provisioning
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryTaskStatus {
    /// Current task state
    pub status: RegistryTaskState,
    /// Current step description
    pub current_step: Option<String>,
    /// Progress percentage (0-100)
    pub progress: Option<u8>,
    /// Overall status message
    pub overall_status: Option<String>,
    /// Overall human-readable message
    pub overall_message: Option<String>,
    /// Output data when completed
    #[serde(default)]
    pub output: RegistryTaskOutput,
    /// Error info if failed
    pub error: Option<RegistryTaskError>,
}

/// State of a registry provisioning task
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RegistryTaskState {
    Processing,
    Completed,
    Failed,
    Cancelled,
    #[serde(other)]
    Unknown,
}

/// Output data from a completed registry provisioning task
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryTaskOutput {
    /// Name of the provisioned registry
    pub registry_name: Option<String>,
    /// URL to push/pull images
    pub registry_url: Option<String>,
    /// Cloud provider that hosts the registry
    pub cloud_provider: Option<String>,
    /// URL to the commit that created the registry
    pub commit_url: Option<String>,
}

/// Error details from a failed registry provisioning task
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryTaskError {
    /// Error name/type
    pub name: String,
    /// Error message
    pub message: String,
}

// =============================================================================
// CLI Wizard Types
// =============================================================================

/// Deployment target type for the CLI wizard
///
/// Determines whether the service deploys to a managed Cloud Runner
/// (GCP Cloud Run, Hetzner container) or to a Kubernetes cluster.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentTarget {
    /// Deploy to Cloud Runner (GCP Cloud Run or Hetzner container)
    /// No cluster required - fully managed by cloud provider
    CloudRunner,
    /// Deploy to a Kubernetes cluster
    /// Requires cluster selection
    Kubernetes,
}

impl DeploymentTarget {
    /// Returns the API string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            DeploymentTarget::CloudRunner => "cloud_runner",
            DeploymentTarget::Kubernetes => "kubernetes",
        }
    }

    /// Returns a human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            DeploymentTarget::CloudRunner => "Cloud Runner",
            DeploymentTarget::Kubernetes => "Kubernetes",
        }
    }
}

impl fmt::Display for DeploymentTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Deployment configuration being built by the CLI wizard
///
/// This type accumulates selections made during the wizard flow
/// before being converted to a CreateDeploymentConfigRequest.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WizardDeploymentConfig {
    /// Service name (from Dockerfile discovery or user input)
    pub service_name: Option<String>,
    /// Path to the Dockerfile relative to repo root
    pub dockerfile_path: Option<String>,
    /// Build context path relative to repo root
    pub build_context: Option<String>,
    /// Port the service listens on
    pub port: Option<u16>,
    /// Git branch to deploy from
    pub branch: Option<String>,
    /// Deployment target type
    pub target: Option<DeploymentTarget>,
    /// Selected cloud provider
    pub provider: Option<CloudProvider>,
    /// Selected cluster ID (required for Kubernetes target)
    pub cluster_id: Option<String>,
    /// Selected registry ID (or None to provision new)
    pub registry_id: Option<String>,
    /// Environment ID for deployment
    pub environment_id: Option<String>,
    /// Enable auto-deploy on push
    pub auto_deploy: bool,
    /// Region/Location for Cloud Runner deployment (e.g., "nbg1" for Hetzner, "us-central1" for GCP)
    pub region: Option<String>,
    /// Machine/Instance type for Cloud Runner (e.g., "cx22" for Hetzner, "e2-small" for GCP)
    pub machine_type: Option<String>,
    /// Whether the service should be publicly accessible
    pub is_public: bool,
    /// Health check endpoint path (optional, e.g., "/health" or "/healthz")
    pub health_check_path: Option<String>,
}

impl WizardDeploymentConfig {
    /// Create a new empty wizard config
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all required fields are set for the selected target
    pub fn is_complete(&self) -> bool {
        let base_complete = self.service_name.is_some()
            && self.port.is_some()
            && self.branch.is_some()
            && self.target.is_some()
            && self.provider.is_some()
            && self.environment_id.is_some();

        if !base_complete {
            return false;
        }

        // K8s requires cluster selection
        if self.target == Some(DeploymentTarget::Kubernetes) {
            return self.cluster_id.is_some();
        }

        // Cloud Runner requires region and machine type
        if self.target == Some(DeploymentTarget::CloudRunner) {
            return self.region.is_some() && self.machine_type.is_some();
        }

        true
    }

    /// Get a list of missing required fields
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.service_name.is_none() {
            missing.push("service_name");
        }
        if self.port.is_none() {
            missing.push("port");
        }
        if self.branch.is_none() {
            missing.push("branch");
        }
        if self.target.is_none() {
            missing.push("target");
        }
        if self.provider.is_none() {
            missing.push("provider");
        }
        if self.environment_id.is_none() {
            missing.push("environment_id");
        }
        if self.target == Some(DeploymentTarget::Kubernetes) && self.cluster_id.is_none() {
            missing.push("cluster_id");
        }
        if self.target == Some(DeploymentTarget::CloudRunner) {
            if self.region.is_none() {
                missing.push("region");
            }
            if self.machine_type.is_none() {
                missing.push("machine_type");
            }
        }
        missing
    }
}

/// Repository connected to a project
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRepository {
    /// Connection ID
    pub id: String,
    /// Project ID
    pub project_id: String,
    /// GitHub repository ID
    pub repository_id: i64,
    /// Repository name (e.g., "my-repo")
    pub repository_name: String,
    /// Full repository name (e.g., "owner/my-repo")
    pub repository_full_name: String,
    /// Repository owner
    pub repository_owner: String,
    /// Whether the repository is private
    pub repository_private: bool,
    /// Default branch name
    #[serde(default)]
    pub default_branch: Option<String>,
    /// Whether the connection is active
    #[serde(default = "default_true")]
    pub is_active: bool,
    /// Connection type (e.g., "app")
    #[serde(default)]
    pub connection_type: Option<String>,
    /// Repository type (e.g., "application", "gitops")
    #[serde(default)]
    pub repository_type: Option<String>,
    /// Whether this is the primary GitOps repository
    #[serde(default)]
    pub is_primary_git_ops: Option<bool>,
    /// GitHub installation ID
    #[serde(default)]
    pub github_installation_id: Option<i64>,
    /// User ID who connected the repository
    #[serde(default)]
    pub user_id: Option<String>,
    /// When the repository was connected
    #[serde(default)]
    pub created_at: Option<String>,
    /// When the repository was last updated
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Response for listing project repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRepositoriesResponse {
    /// Connected repositories
    pub repositories: Vec<ProjectRepository>,
    /// Total count
    pub total_count: i32,
}

/// Cloud Runner configuration for internal wizard use
///
/// Note: This is used internally by the wizard to collect configuration.
/// When sending to the API, use `build_cloud_runner_config()` to create
/// the provider-nested structure the backend expects.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CloudRunnerConfig {
    /// Region/location (e.g., "nbg1", "us-central1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Machine/instance type (e.g., "cx22", "e2-small")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_type: Option<String>,
    /// Whether service should be publicly accessible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    /// Health check endpoint path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_path: Option<String>,
}

/// Build the cloud runner config in the provider-nested structure expected by backend.
///
/// The backend expects:
/// - For GCP: `{ "gcp": { "region": "...", "allowUnauthenticated": true } }`
/// - For Hetzner: `{ "hetzner": { "location": "...", "serverType": "..." } }`
///
/// # Arguments
/// * `provider` - The cloud provider (GCP, Hetzner, etc.)
/// * `region` - Region/location for deployment
/// * `machine_type` - Machine/server type
/// * `is_public` - Whether the service should be publicly accessible
/// * `health_check_path` - Optional health check endpoint path
pub fn build_cloud_runner_config(
    provider: &CloudProvider,
    region: &str,
    machine_type: &str,
    is_public: bool,
    health_check_path: Option<&str>,
) -> serde_json::Value {
    match provider {
        CloudProvider::Gcp => {
            let mut gcp_config = serde_json::json!({
                "region": region,
                "allowUnauthenticated": is_public,
            });
            if let Some(path) = health_check_path {
                gcp_config["healthCheckPath"] = serde_json::json!(path);
            }
            serde_json::json!({
                "gcp": gcp_config
            })
        }
        CloudProvider::Hetzner => {
            serde_json::json!({
                "hetzner": {
                    "location": region,
                    "serverType": machine_type
                }
            })
        }
        // For other providers, use a generic structure
        _ => {
            serde_json::json!({
                provider.as_str(): {
                    "region": region,
                    "machineType": machine_type,
                    "isPublic": is_public
                }
            })
        }
    }
}

/// Request body for creating a new deployment configuration
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentConfigRequest {
    /// Project ID
    pub project_id: String,
    /// Service name for the deployment
    pub service_name: String,
    /// Repository ID (from GitHub/GitLab integration)
    pub repository_id: i64,
    /// Full repository name (e.g., "owner/repo")
    pub repository_full_name: String,
    /// Path to Dockerfile relative to repo root
    /// Note: Backend may use "dockerfile" or "dockerfilePath" - sending both for compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile_path: Option<String>,
    /// Alias for dockerfile_path (some backend endpoints expect this name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<String>,
    /// Build context path relative to repo root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_context: Option<String>,
    /// Alias for build_context (some backend endpoints expect this name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Port the service listens on
    pub port: i32,
    /// Git branch to deploy from
    pub branch: String,
    /// Target type: "kubernetes" or "cloud_runner"
    pub target_type: String,
    /// Cloud provider (gcp, hetzner)
    pub cloud_provider: String,
    /// Environment ID for deployment
    pub environment_id: String,
    /// Cluster ID (required for kubernetes target)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<String>,
    /// Registry ID (optional - will provision if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    /// Enable auto-deploy on push
    pub auto_deploy_enabled: bool,
    /// Public access for the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    /// Cloud Runner specific configuration (provider-nested structure)
    ///
    /// Use `build_cloud_runner_config()` to create this value.
    /// Backend expects: `{ "gcp": {...} }` or `{ "hetzner": {...} }`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_runner_config: Option<serde_json::Value>,
}

/// Provider deployment availability status for the wizard
///
/// Combines provider connection status with available resources
/// to help users select where to deploy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDeploymentStatus {
    /// The cloud provider
    pub provider: CloudProvider,
    /// Whether the provider is connected (has credentials)
    pub is_connected: bool,
    /// Available Kubernetes clusters (empty if no clusters or not connected)
    pub clusters: Vec<ClusterSummary>,
    /// Available artifact registries (empty if none or not connected)
    pub registries: Vec<RegistrySummary>,
    /// Whether Cloud Runner is available for this provider
    pub cloud_runner_available: bool,
    /// Display message for the wizard (e.g., "2 clusters, 1 registry")
    pub summary: String,
}

/// Summary of a K8s cluster for wizard display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterSummary {
    /// Cluster ID
    pub id: String,
    /// Cluster display name
    pub name: String,
    /// Region
    pub region: String,
    /// Is cluster running/healthy
    pub is_healthy: bool,
}

/// Summary of an artifact registry for wizard display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySummary {
    /// Registry ID
    pub id: String,
    /// Registry display name
    pub name: String,
    /// Region
    pub region: String,
    /// Is registry ready
    pub is_ready: bool,
}

impl ProviderDeploymentStatus {
    /// Check if this provider can be used for deployment
    pub fn can_deploy(&self) -> bool {
        self.is_connected && (self.cloud_runner_available || !self.clusters.is_empty())
    }

    /// Get available deployment targets for this provider
    pub fn available_targets(&self) -> Vec<DeploymentTarget> {
        let mut targets = Vec::new();
        if self.cloud_runner_available {
            targets.push(DeploymentTarget::CloudRunner);
        }
        if !self.clusters.is_empty() {
            targets.push(DeploymentTarget::Kubernetes);
        }
        targets
    }
}

// =========================================================================
// GitHub Integration Types
// =========================================================================

/// GitHub App installation connected to the organization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubInstallation {
    /// GitHub App installation ID
    pub installation_id: i64,
    /// GitHub account ID
    pub account_id: i64,
    /// GitHub account login/username
    pub account_login: String,
    /// Account type: "User" or "Organization"
    pub account_type: String,
    /// Target type: "User" or "Organization"
    #[serde(default)]
    pub target_type: Option<String>,
    /// Permissions granted to the app
    #[serde(default)]
    pub permissions: Option<serde_json::Value>,
    /// Events the app is subscribed to
    #[serde(default)]
    pub events: Option<Vec<String>>,
    /// Repository selection: "all" or "selected"
    #[serde(default)]
    pub repository_selection: Option<String>,
    /// GitHub App ID
    #[serde(default)]
    pub app_id: Option<i64>,
    /// GitHub App slug
    #[serde(default)]
    pub app_slug: Option<String>,
    /// When the installation was suspended
    #[serde(default)]
    pub suspended_at: Option<String>,
    /// Who suspended the installation
    #[serde(default)]
    pub suspended_by: Option<String>,
    /// When the installation was created
    #[serde(default)]
    pub created_at: Option<String>,
    /// When the installation was last updated
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Response for listing GitHub installations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubInstallationsResponse {
    /// List of GitHub App installations
    pub installations: Vec<GitHubInstallation>,
}

/// Response for getting GitHub App installation URL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubInstallationUrlResponse {
    /// URL to install the GitHub App
    pub installation_url: String,
}

/// Repository available for connection (from GitHub)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableRepository {
    /// GitHub repository ID
    pub id: i64,
    /// Repository name (e.g., "my-repo")
    pub name: String,
    /// Full repository name (e.g., "owner/my-repo")
    pub full_name: String,
    /// Repository owner
    #[serde(default)]
    pub owner: Option<String>,
    /// Whether the repository is private
    #[serde(default)]
    pub private: bool,
    /// Default branch name
    #[serde(default)]
    pub default_branch: Option<String>,
    /// Repository description
    #[serde(default)]
    pub description: Option<String>,
    /// Repository HTML URL
    #[serde(default)]
    pub html_url: Option<String>,
    /// GitHub installation ID this repo is accessible through
    #[serde(default)]
    pub installation_id: Option<i64>,
}

/// Response for listing available repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableRepositoriesResponse {
    /// List of available repositories
    pub repositories: Vec<AvailableRepository>,
    /// IDs of repositories already connected to the project
    #[serde(default)]
    pub connected_repositories: Vec<i64>,
    /// Total count of available repositories
    #[serde(default)]
    pub total_count: i32,
    /// Current page number
    #[serde(default)]
    pub page: i32,
    /// Items per page
    #[serde(default)]
    pub per_page: i32,
    /// Whether there are more pages
    #[serde(default)]
    pub has_more: bool,
}

/// Request to connect a repository to a project
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRepositoryRequest {
    /// Project ID to connect the repository to
    pub project_id: String,
    /// GitHub repository ID
    pub repository_id: i64,
    /// Repository name
    pub repository_name: String,
    /// Full repository name (owner/repo)
    pub repository_full_name: String,
    /// Repository owner
    pub repository_owner: String,
    /// Whether the repository is private
    pub repository_private: bool,
    /// Default branch name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    /// Connection type (e.g., "app")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<String>,
    /// GitHub installation ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_installation_id: Option<i64>,
    /// Repository type: "application" or "gitops"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_type: Option<String>,
}

/// Response after connecting a repository to a project
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRepositoryResponse {
    /// Connection ID
    pub id: String,
    /// Project ID
    pub project_id: String,
    /// GitHub repository ID
    pub repository_id: i64,
    /// Full repository name
    pub repository_full_name: String,
    /// Whether the connection is active
    #[serde(default = "default_true")]
    pub is_active: bool,
}

/// Request to initialize GitOps repository for a project
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeGitOpsRequest {
    /// GitHub installation ID to use for creating the repo
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installation_id: Option<i64>,
}

/// Response after initializing GitOps repository
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeGitOpsResponse {
    /// Full name of the created/existing GitOps repository
    pub repo_full_name: String,
    /// GitHub installation ID used
    pub installation_id: i64,
}

// =============================================================================
// Hetzner Availability Types (Dynamic Resource Fetching)
// =============================================================================

/// Hetzner location with geographic metadata (from Hetzner API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HetznerLocation {
    /// Location ID (e.g., 1)
    pub id: i64,
    /// Location code (e.g., "fsn1", "nbg1")
    pub name: String,
    /// Location description
    pub description: String,
    /// Country code
    pub country: String,
    /// City name
    pub city: String,
    /// Geographic latitude
    pub latitude: f64,
    /// Geographic longitude
    pub longitude: f64,
    /// Network zone (e.g., "eu-central")
    pub network_zone: String,
}

/// Location with available server types (from availability API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationWithAvailability {
    /// Location details
    pub location: HetznerLocation,
    /// Server type names available at this location
    pub available_server_types: Vec<String>,
}

/// Server type summary with availability and pricing (from availability API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerTypeSummary {
    /// Server type ID
    pub id: i64,
    /// Server type name (e.g., "cx22", "cx32")
    pub name: String,
    /// Number of vCPUs
    pub cores: i32,
    /// Memory in GB
    pub memory_gb: f64,
    /// Disk size in GB
    pub disk_gb: i64,
    /// Hourly price in EUR (gross)
    pub price_hourly: f64,
    /// Monthly price in EUR (gross)
    pub price_monthly: f64,
    /// Locations where this server type is currently available
    pub available_in: Vec<String>,
}

/// Availability check result for a specific server type at a location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityCheckResult {
    /// Whether the server type is available
    pub available: bool,
    /// The location that was checked
    pub location: String,
    /// The server type that was checked
    pub server_type: String,
    /// Reason if unavailable: "capacity" or "unsupported"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Alternative locations where this server type IS available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative_locations: Option<Vec<String>>,
}

/// Response wrapper for locations with availability
#[derive(Debug, Clone, Deserialize)]
pub struct LocationsAvailabilityResponse {
    pub data: Vec<LocationWithAvailability>,
}

/// Response wrapper for server types
#[derive(Debug, Clone, Deserialize)]
pub struct ServerTypesResponse {
    pub data: Vec<ServerTypeSummary>,
}

// =============================================================================
// Hetzner Options Types (from /api/v1/cloud-runner/hetzner/options)
// =============================================================================

/// Simple Hetzner location (from getHetznerOptions endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HetznerSimpleLocation {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub city: String,
    pub country: String,
    pub network_zone: String,
}

/// Hetzner server type with pricing (from getHetznerOptions endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HetznerSimpleServerType {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub cores: i32,
    pub memory: f64,
    pub disk: i64,
    pub cpu_type: String,
    pub architecture: String,
    pub deprecated: bool,
    #[serde(default)]
    pub available_locations: Vec<String>,
    #[serde(default)]
    pub price_monthly: f64,
}

/// Combined Hetzner options response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HetznerOptionsData {
    pub locations: Vec<HetznerSimpleLocation>,
    pub server_types: Vec<HetznerSimpleServerType>,
}

/// Wrapped response for getHetznerOptions
#[derive(Debug, Clone, Deserialize)]
pub struct HetznerOptionsResponse {
    pub data: HetznerOptionsData,
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
        assert_eq!(CloudProvider::Scaleway.as_str(), "scaleway");
        assert_eq!(CloudProvider::Cyso.as_str(), "cyso");
    }

    #[test]
    fn test_cloud_provider_display_name() {
        assert_eq!(CloudProvider::Gcp.display_name(), "Google Cloud Platform");
        assert_eq!(CloudProvider::Aws.display_name(), "Amazon Web Services");
        assert_eq!(CloudProvider::Azure.display_name(), "Microsoft Azure");
        assert_eq!(CloudProvider::Hetzner.display_name(), "Hetzner Cloud");
        assert_eq!(CloudProvider::Scaleway.display_name(), "Scaleway");
        assert_eq!(CloudProvider::Cyso.display_name(), "Cyso Cloud");
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
        assert_eq!(
            CloudProvider::from_str("scaleway").unwrap(),
            CloudProvider::Scaleway
        );
        assert_eq!(
            CloudProvider::from_str("cyso").unwrap(),
            CloudProvider::Cyso
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
        assert_eq!(all.len(), 6);
        assert!(all.contains(&CloudProvider::Gcp));
        assert!(all.contains(&CloudProvider::Aws));
        assert!(all.contains(&CloudProvider::Azure));
        assert!(all.contains(&CloudProvider::Hetzner));
        assert!(all.contains(&CloudProvider::Scaleway));
        assert!(all.contains(&CloudProvider::Cyso));
    }

    #[test]
    fn test_cloud_provider_is_available() {
        // Available providers
        assert!(CloudProvider::Gcp.is_available());
        assert!(CloudProvider::Hetzner.is_available());

        // Coming soon providers
        assert!(!CloudProvider::Aws.is_available());
        assert!(!CloudProvider::Azure.is_available());
        assert!(!CloudProvider::Scaleway.is_available());
        assert!(!CloudProvider::Cyso.is_available());
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

    // =========================================================================
    // CLI Wizard Types Tests
    // =========================================================================

    #[test]
    fn test_deployment_target_as_str() {
        assert_eq!(DeploymentTarget::CloudRunner.as_str(), "cloud_runner");
        assert_eq!(DeploymentTarget::Kubernetes.as_str(), "kubernetes");
    }

    #[test]
    fn test_deployment_target_display_name() {
        assert_eq!(DeploymentTarget::CloudRunner.display_name(), "Cloud Runner");
        assert_eq!(DeploymentTarget::Kubernetes.display_name(), "Kubernetes");
    }

    #[test]
    fn test_wizard_config_is_complete_cloud_runner() {
        let mut config = WizardDeploymentConfig::new();
        assert!(!config.is_complete());

        config.service_name = Some("api".to_string());
        config.port = Some(8080);
        config.branch = Some("main".to_string());
        config.target = Some(DeploymentTarget::CloudRunner);
        config.provider = Some(CloudProvider::Gcp);
        config.environment_id = Some("env-123".to_string());

        // Cloud Runner requires region and machine type
        assert!(!config.is_complete());

        config.region = Some("us-central1".to_string());
        config.machine_type = Some("e2-small".to_string());

        assert!(config.is_complete());
    }

    #[test]
    fn test_wizard_config_is_complete_kubernetes() {
        let mut config = WizardDeploymentConfig::new();
        config.service_name = Some("api".to_string());
        config.port = Some(8080);
        config.branch = Some("main".to_string());
        config.target = Some(DeploymentTarget::Kubernetes);
        config.provider = Some(CloudProvider::Gcp);
        config.environment_id = Some("env-123".to_string());

        // K8s requires cluster_id
        assert!(!config.is_complete());

        config.cluster_id = Some("cluster-123".to_string());
        assert!(config.is_complete());
    }

    #[test]
    fn test_wizard_config_missing_fields() {
        let config = WizardDeploymentConfig::new();
        let missing = config.missing_fields();
        assert!(missing.contains(&"service_name"));
        assert!(missing.contains(&"port"));
        assert!(missing.contains(&"branch"));
    }

    #[test]
    fn test_provider_deployment_status_can_deploy() {
        let status = ProviderDeploymentStatus {
            provider: CloudProvider::Gcp,
            is_connected: true,
            clusters: vec![],
            registries: vec![],
            cloud_runner_available: true,
            summary: "Cloud Run available".to_string(),
        };
        assert!(status.can_deploy());

        let disconnected = ProviderDeploymentStatus {
            provider: CloudProvider::Aws,
            is_connected: false,
            clusters: vec![],
            registries: vec![],
            cloud_runner_available: false,
            summary: "Not connected".to_string(),
        };
        assert!(!disconnected.can_deploy());
    }

    #[test]
    fn test_provider_deployment_status_available_targets() {
        let status = ProviderDeploymentStatus {
            provider: CloudProvider::Gcp,
            is_connected: true,
            clusters: vec![ClusterSummary {
                id: "c1".to_string(),
                name: "prod-cluster".to_string(),
                region: "us-central1".to_string(),
                is_healthy: true,
            }],
            registries: vec![],
            cloud_runner_available: true,
            summary: "1 cluster, Cloud Run".to_string(),
        };

        let targets = status.available_targets();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&DeploymentTarget::CloudRunner));
        assert!(targets.contains(&DeploymentTarget::Kubernetes));
    }

    // =========================================================================
    // Environment Tests
    // =========================================================================

    #[test]
    fn test_environment_serialization() {
        let env = Environment {
            id: "env-123".to_string(),
            name: "production".to_string(),
            project_id: "proj-456".to_string(),
            environment_type: "cluster".to_string(),
            cluster_id: Some("cluster-789".to_string()),
            namespace: Some("prod-ns".to_string()),
            description: Some("Production environment".to_string()),
            is_active: true,
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            updated_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("\"id\":\"env-123\""));
        assert!(json.contains("\"name\":\"production\""));
        assert!(json.contains("\"projectId\":\"proj-456\""));
        assert!(json.contains("\"environmentType\":\"cluster\""));
        assert!(json.contains("\"clusterId\":\"cluster-789\""));
    }

    #[test]
    fn test_environment_deserialization() {
        let json = r#"{
            "id": "env-abc",
            "name": "staging",
            "projectId": "proj-def",
            "environmentType": "cloud",
            "isActive": true,
            "createdAt": "2024-01-15T12:00:00Z",
            "updatedAt": "2024-01-15T12:00:00Z"
        }"#;

        let env: Environment = serde_json::from_str(json).unwrap();
        assert_eq!(env.id, "env-abc");
        assert_eq!(env.name, "staging");
        assert_eq!(env.project_id, "proj-def");
        assert_eq!(env.environment_type, "cloud");
        assert!(env.cluster_id.is_none());
        assert_eq!(env.created_at, Some("2024-01-15T12:00:00Z".to_string()));
    }

    #[test]
    fn test_environment_optional_fields_default() {
        let json = r#"{
            "id": "env-min",
            "name": "minimal",
            "projectId": "proj-min",
            "environmentType": "cloud"
        }"#;

        let env: Environment = serde_json::from_str(json).unwrap();
        assert!(env.cluster_id.is_none());
        assert!(env.created_at.is_none());
        assert!(env.is_active); // default_true
    }

    #[test]
    fn test_create_deployment_config_request_serialization() {
        let request = CreateDeploymentConfigRequest {
            project_id: "proj-123".to_string(),
            service_name: "api".to_string(),
            repository_id: 12345,
            repository_full_name: "org/repo".to_string(),
            dockerfile_path: Some("Dockerfile".to_string()),
            dockerfile: Some("Dockerfile".to_string()),
            build_context: Some(".".to_string()),
            context: Some(".".to_string()),
            port: 8080,
            branch: "main".to_string(),
            target_type: "cloud_runner".to_string(),
            cloud_provider: "gcp".to_string(),
            environment_id: "env-123".to_string(),
            cluster_id: None,
            registry_id: Some("reg-456".to_string()),
            auto_deploy_enabled: true,
            is_public: None,
            cloud_runner_config: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"serviceName\":\"api\""));
        assert!(json.contains("\"port\":8080"));
        // Optional None fields should be skipped
        assert!(!json.contains("clusterId"));
        assert!(!json.contains("isPublic"));
    }

    // =========================================================================
    // Cloud Runner Config Builder Tests
    // =========================================================================

    #[test]
    fn test_build_cloud_runner_config_gcp() {
        let config = build_cloud_runner_config(
            &CloudProvider::Gcp,
            "us-central1",
            "e2-small",
            true,
            Some("/health"),
        );
        let gcp = config.get("gcp").expect("should have gcp key");
        assert_eq!(gcp.get("region").and_then(|v| v.as_str()), Some("us-central1"));
        assert_eq!(gcp.get("allowUnauthenticated").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(gcp.get("healthCheckPath").and_then(|v| v.as_str()), Some("/health"));
    }

    #[test]
    fn test_build_cloud_runner_config_gcp_private() {
        let config = build_cloud_runner_config(
            &CloudProvider::Gcp,
            "europe-west1",
            "e2-medium",
            false,
            None,
        );
        let gcp = config.get("gcp").expect("should have gcp key");
        assert_eq!(gcp.get("region").and_then(|v| v.as_str()), Some("europe-west1"));
        assert_eq!(gcp.get("allowUnauthenticated").and_then(|v| v.as_bool()), Some(false));
        // No health check path when not provided
        assert!(gcp.get("healthCheckPath").is_none());
    }

    #[test]
    fn test_build_cloud_runner_config_hetzner() {
        let config = build_cloud_runner_config(
            &CloudProvider::Hetzner,
            "nbg1",
            "cx22",
            true,
            None,
        );
        let hetzner = config.get("hetzner").expect("should have hetzner key");
        assert_eq!(hetzner.get("location").and_then(|v| v.as_str()), Some("nbg1"));
        assert_eq!(hetzner.get("serverType").and_then(|v| v.as_str()), Some("cx22"));
    }

    #[test]
    fn test_build_cloud_runner_config_hetzner_different_location() {
        let config = build_cloud_runner_config(
            &CloudProvider::Hetzner,
            "fsn1",
            "cx32",
            false,
            Some("/healthz"),
        );
        let hetzner = config.get("hetzner").expect("should have hetzner key");
        assert_eq!(hetzner.get("location").and_then(|v| v.as_str()), Some("fsn1"));
        assert_eq!(hetzner.get("serverType").and_then(|v| v.as_str()), Some("cx32"));
        // Hetzner config doesn't include health check path in current implementation
    }
}
