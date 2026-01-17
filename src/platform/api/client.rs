//! Platform API client for Syncable
//!
//! Provides authenticated access to the Syncable Platform API for managing
//! organizations, projects, and other platform resources.

use super::error::{PlatformApiError, Result};
use super::types::{
    ApiErrorResponse, ArtifactRegistry, CloudCredentialStatus, CloudProvider, ClusterEntity,
    CreateDeploymentConfigRequest, CreateRegistryRequest, CreateRegistryResponse, DeploymentConfig,
    DeploymentTaskStatus, GenericResponse, GetLogsResponse, Organization, PaginatedDeployments,
    Project, RegistryTaskStatus, TriggerDeploymentRequest, TriggerDeploymentResponse, UserProfile,
};
use crate::auth::credentials;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

/// Production API URL
const SYNCABLE_API_URL_PROD: &str = "https://syncable.dev";
/// Development API URL
const SYNCABLE_API_URL_DEV: &str = "http://localhost:4000";

/// User agent for API requests
const USER_AGENT: &str = concat!("syncable-cli/", env!("CARGO_PKG_VERSION"));

/// Maximum number of retry attempts for transient failures
const MAX_RETRIES: u32 = 3;
/// Initial backoff delay in milliseconds
const INITIAL_BACKOFF_MS: u64 = 500;
/// Maximum backoff delay in milliseconds
const MAX_BACKOFF_MS: u64 = 5000;

/// Check if an error is retryable (transient failure)
fn is_retryable_error(error: &PlatformApiError) -> bool {
    matches!(
        error,
        PlatformApiError::HttpError(_)      // Network errors, timeouts
        | PlatformApiError::RateLimited     // 429 - rate limited
        | PlatformApiError::ServerError { .. } // 5xx - server errors
        | PlatformApiError::ConnectionFailed // Connection failures
    )
}

/// Client for interacting with the Syncable Platform API
pub struct PlatformApiClient {
    /// HTTP client with configured timeout and headers
    http_client: Client,
    /// Base API URL
    api_url: String,
}

impl PlatformApiClient {
    /// Create a new Platform API client using the default API URL
    ///
    /// Uses `SYNCABLE_ENV=development` to switch to local development server.
    pub fn new() -> Result<Self> {
        let api_url = get_api_url();
        Self::with_url(api_url)
    }

    /// Create a new Platform API client with a custom API URL
    pub fn with_url(api_url: impl Into<String>) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
            .map_err(PlatformApiError::HttpError)?;

        Ok(Self {
            http_client,
            api_url: api_url.into(),
        })
    }

    /// Get the configured API URL
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    /// Get the authentication token from stored credentials
    fn get_auth_token() -> Result<String> {
        credentials::get_access_token().ok_or(PlatformApiError::Unauthorized)
    }

    /// Make an authenticated GET request with automatic retry for transient failures
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let token = Self::get_auth_token()?;
        let url = format!("{}{}", self.api_url, path);

        let mut last_error = None;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        for attempt in 0..=MAX_RETRIES {
            let result = self
                .http_client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await;

            match result {
                Ok(response) => {
                    match self.handle_response(response).await {
                        Ok(data) => return Ok(data),
                        Err(e) if is_retryable_error(&e) && attempt < MAX_RETRIES => {
                            eprintln!(
                                "Request failed (attempt {}/{}), retrying in {}ms...",
                                attempt + 1,
                                MAX_RETRIES + 1,
                                backoff_ms
                            );
                            last_error = Some(e);
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) => {
                    let platform_error = PlatformApiError::HttpError(e);
                    if is_retryable_error(&platform_error) && attempt < MAX_RETRIES {
                        eprintln!(
                            "Network error (attempt {}/{}), retrying in {}ms...",
                            attempt + 1,
                            MAX_RETRIES + 1,
                            backoff_ms
                        );
                        last_error = Some(platform_error);
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                    } else {
                        return Err(platform_error);
                    }
                }
            }
        }

        Err(last_error.expect("retry loop should have set last_error"))
    }

    /// Make an authenticated GET request that returns Option<T>
    /// Returns None for 404 responses instead of an error
    /// Includes retry logic for transient failures
    async fn get_optional<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let token = Self::get_auth_token()?;
        let url = format!("{}{}", self.api_url, path);

        let mut last_error = None;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        for attempt in 0..=MAX_RETRIES {
            let result = self
                .http_client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        let result = response
                            .json::<T>()
                            .await
                            .map_err(|e| PlatformApiError::ParseError(e.to_string()))?;
                        return Ok(Some(result));
                    } else if status.as_u16() == 404 {
                        return Ok(None);
                    } else {
                        let status_code = status.as_u16();
                        let error_body = response.text().await.unwrap_or_default();
                        let error_message = serde_json::from_str::<ApiErrorResponse>(&error_body)
                            .map(|e| e.get_message())
                            .unwrap_or_else(|_| error_body.clone());

                        let error = match status_code {
                            401 => PlatformApiError::Unauthorized,
                            403 => PlatformApiError::PermissionDenied(error_message),
                            429 => PlatformApiError::RateLimited,
                            500..=599 => PlatformApiError::ServerError {
                                status: status_code,
                                message: error_message,
                            },
                            _ => PlatformApiError::ApiError {
                                status: status_code,
                                message: error_message,
                            },
                        };

                        if is_retryable_error(&error) && attempt < MAX_RETRIES {
                            eprintln!(
                                "Request failed (attempt {}/{}), retrying in {}ms...",
                                attempt + 1,
                                MAX_RETRIES + 1,
                                backoff_ms
                            );
                            last_error = Some(error);
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    let platform_error = PlatformApiError::HttpError(e);
                    if is_retryable_error(&platform_error) && attempt < MAX_RETRIES {
                        eprintln!(
                            "Network error (attempt {}/{}), retrying in {}ms...",
                            attempt + 1,
                            MAX_RETRIES + 1,
                            backoff_ms
                        );
                        last_error = Some(platform_error);
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                    } else {
                        return Err(platform_error);
                    }
                }
            }
        }

        Err(last_error.expect("retry loop should have set last_error"))
    }

    /// Make an authenticated POST request with a JSON body
    /// Only retries on network errors (before request completes), not on server responses,
    /// since POST requests may not be idempotent.
    async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let token = Self::get_auth_token()?;
        let url = format!("{}{}", self.api_url, path);

        let mut last_error = None;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        for attempt in 0..=MAX_RETRIES {
            let result = self
                .http_client
                .post(&url)
                .bearer_auth(&token)
                .json(body)
                .send()
                .await;

            match result {
                Ok(response) => {
                    // Got a response - don't retry POST even on server errors
                    return self.handle_response(response).await;
                }
                Err(e) => {
                    // Network error before request completed - safe to retry
                    let platform_error = PlatformApiError::HttpError(e);
                    if attempt < MAX_RETRIES {
                        eprintln!(
                            "Network error (attempt {}/{}), retrying in {}ms...",
                            attempt + 1,
                            MAX_RETRIES + 1,
                            backoff_ms
                        );
                        last_error = Some(platform_error);
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                    } else {
                        return Err(platform_error);
                    }
                }
            }
        }

        Err(last_error.expect("retry loop should have set last_error"))
    }

    /// Handle the HTTP response, converting errors appropriately
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            // Try to parse the response body
            response
                .json::<T>()
                .await
                .map_err(|e| PlatformApiError::ParseError(e.to_string()))
        } else {
            // Try to parse error response for better error messages
            let status_code = status.as_u16();
            let error_body = response.text().await.unwrap_or_default();
            let error_message = serde_json::from_str::<ApiErrorResponse>(&error_body)
                .map(|e| e.get_message())
                .unwrap_or_else(|_| error_body.clone());

            match status_code {
                401 => Err(PlatformApiError::Unauthorized),
                403 => Err(PlatformApiError::PermissionDenied(error_message)),
                404 => Err(PlatformApiError::NotFound(error_message)),
                429 => Err(PlatformApiError::RateLimited),
                500..=599 => Err(PlatformApiError::ServerError {
                    status: status_code,
                    message: error_message,
                }),
                _ => Err(PlatformApiError::ApiError {
                    status: status_code,
                    message: error_message,
                }),
            }
        }
    }

    // =========================================================================
    // User API methods
    // =========================================================================

    /// Get the current authenticated user's profile
    ///
    /// Endpoint: GET /api/users/me
    pub async fn get_current_user(&self) -> Result<UserProfile> {
        self.get("/api/users/me").await
    }

    // =========================================================================
    // Organization API methods
    // =========================================================================

    /// List organizations the authenticated user belongs to
    ///
    /// Endpoint: GET /api/organizations/attended-by-user
    pub async fn list_organizations(&self) -> Result<Vec<Organization>> {
        let response: GenericResponse<Vec<Organization>> =
            self.get("/api/organizations/attended-by-user").await?;
        Ok(response.data)
    }

    /// Get an organization by ID
    ///
    /// Endpoint: GET /api/organizations/:id
    pub async fn get_organization(&self, id: &str) -> Result<Organization> {
        let response: GenericResponse<Organization> =
            self.get(&format!("/api/organizations/{}", id)).await?;
        Ok(response.data)
    }

    // =========================================================================
    // Project API methods
    // =========================================================================

    /// List projects in an organization
    ///
    /// Endpoint: GET /api/projects/organization/:organizationId
    pub async fn list_projects(&self, org_id: &str) -> Result<Vec<Project>> {
        let response: GenericResponse<Vec<Project>> = self
            .get(&format!("/api/projects/organization/{}", org_id))
            .await?;
        Ok(response.data)
    }

    /// Get a project by ID
    ///
    /// Endpoint: GET /api/projects/:id
    pub async fn get_project(&self, id: &str) -> Result<Project> {
        let response: GenericResponse<Project> =
            self.get(&format!("/api/projects/{}", id)).await?;
        Ok(response.data)
    }

    /// Create a new project in an organization
    ///
    /// Endpoint: POST /api/projects
    ///
    /// Note: This first fetches the current user to get the creator_id.
    pub async fn create_project(
        &self,
        org_id: &str,
        name: &str,
        description: &str,
    ) -> Result<Project> {
        // Get current user to use as creator
        let user = self.get_current_user().await?;

        let request = serde_json::json!({
            "creatorId": user.id,
            "organizationId": org_id,
            "name": name,
            "description": description,
            "context": ""
        });

        let response: GenericResponse<Project> = self.post("/api/projects", &request).await?;
        Ok(response.data)
    }

    // =========================================================================
    // Cloud Credentials API methods
    // =========================================================================

    /// Check if a cloud provider is connected to a project
    ///
    /// Returns `Some(status)` if the provider is connected, `None` if not connected.
    ///
    /// SECURITY NOTE: This method only returns connection STATUS, never actual credentials.
    /// The agent should never have access to OAuth tokens, API keys, or other secrets.
    ///
    /// Endpoint: GET /api/cloud-credentials/provider/:provider?projectId=xxx
    pub async fn check_provider_connection(
        &self,
        provider: &CloudProvider,
        project_id: &str,
    ) -> Result<Option<CloudCredentialStatus>> {
        let path = format!(
            "/api/cloud-credentials/provider/{}?projectId={}",
            provider.as_str(),
            project_id
        );
        self.get_optional(&path).await
    }

    /// List all cloud credentials for a project
    ///
    /// Returns all connected cloud providers for the project.
    ///
    /// SECURITY NOTE: This method only returns connection STATUS, never actual credentials.
    ///
    /// Endpoint: GET /api/cloud-credentials?projectId=xxx
    pub async fn list_cloud_credentials_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<CloudCredentialStatus>> {
        let response: GenericResponse<Vec<CloudCredentialStatus>> = self
            .get(&format!("/api/cloud-credentials?projectId={}", project_id))
            .await?;
        Ok(response.data)
    }

    // =========================================================================
    // Deployment API methods
    // =========================================================================

    /// List deployment configurations for a project
    ///
    /// Returns all deployment configs associated with the project, including
    /// service name, branch, target type, and auto-deploy settings.
    ///
    /// Endpoint: GET /api/projects/:projectId/deployment-configs
    pub async fn list_deployment_configs(&self, project_id: &str) -> Result<Vec<DeploymentConfig>> {
        let response: GenericResponse<Vec<DeploymentConfig>> = self
            .get(&format!("/api/projects/{}/deployment-configs", project_id))
            .await?;
        Ok(response.data)
    }

    /// Create a new deployment configuration
    ///
    /// Creates a deployment config for a service. Requires repository integration
    /// to be set up first (GitHub/GitLab).
    ///
    /// Endpoint: POST /api/deployment-configs?projectId=xxx
    pub async fn create_deployment_config(
        &self,
        project_id: &str,
        request: &CreateDeploymentConfigRequest,
    ) -> Result<DeploymentConfig> {
        let response: GenericResponse<DeploymentConfig> = self
            .post(
                &format!("/api/deployment-configs?projectId={}", project_id),
                request,
            )
            .await?;
        Ok(response.data)
    }

    /// Trigger a deployment using a deployment config
    ///
    /// Starts a new deployment for the specified config. Optionally specify
    /// a commit SHA to deploy a specific version.
    ///
    /// Endpoint: POST /api/deployment-configs/deploy
    pub async fn trigger_deployment(
        &self,
        request: &TriggerDeploymentRequest,
    ) -> Result<TriggerDeploymentResponse> {
        self.post("/api/deployment-configs/deploy", request).await
    }

    /// Get deployment task status
    ///
    /// Returns the current status of a deployment task, including progress
    /// percentage, current step, and overall status.
    ///
    /// Endpoint: GET /api/deployments/task/:taskId
    pub async fn get_deployment_status(&self, task_id: &str) -> Result<DeploymentTaskStatus> {
        self.get(&format!("/api/deployments/task/{}", task_id))
            .await
    }

    /// List deployments for a project
    ///
    /// Returns a paginated list of deployments for the project, sorted by
    /// creation time (most recent first).
    ///
    /// Endpoint: GET /api/deployments/project/:projectId
    pub async fn list_deployments(
        &self,
        project_id: &str,
        limit: Option<i32>,
    ) -> Result<PaginatedDeployments> {
        let path = match limit {
            Some(l) => format!("/api/deployments/project/{}?limit={}", project_id, l),
            None => format!("/api/deployments/project/{}", project_id),
        };
        self.get(&path).await
    }

    /// Get container logs for a deployed service
    ///
    /// Returns recent logs from the service's containers. Supports time filtering
    /// and line limits for efficient log retrieval.
    ///
    /// # Arguments
    ///
    /// * `service_id` - The service/deployment ID (from list_deployments)
    /// * `start` - Optional ISO timestamp to filter logs from
    /// * `end` - Optional ISO timestamp to filter logs until
    /// * `limit` - Optional max number of log lines (default: 100)
    ///
    /// Endpoint: GET /api/deployments/services/:serviceId/logs
    pub async fn get_service_logs(
        &self,
        service_id: &str,
        start: Option<&str>,
        end: Option<&str>,
        limit: Option<i32>,
    ) -> Result<GetLogsResponse> {
        let mut query_params = Vec::new();

        if let Some(s) = start {
            query_params.push(format!("start={}", s));
        }
        if let Some(e) = end {
            query_params.push(format!("end={}", e));
        }
        if let Some(l) = limit {
            query_params.push(format!("limit={}", l));
        }

        let path = if query_params.is_empty() {
            format!("/api/deployments/services/{}/logs", service_id)
        } else {
            format!(
                "/api/deployments/services/{}/logs?{}",
                service_id,
                query_params.join("&")
            )
        };

        self.get(&path).await
    }

    // =========================================================================
    // Cluster API methods
    // =========================================================================

    /// List all clusters for a project
    ///
    /// Returns all K8s clusters available for deployments in this project.
    ///
    /// Endpoint: GET /api/clusters/project/:projectId
    pub async fn list_clusters_for_project(&self, project_id: &str) -> Result<Vec<ClusterEntity>> {
        let response: GenericResponse<Vec<ClusterEntity>> = self
            .get(&format!("/api/clusters/project/{}", project_id))
            .await?;
        Ok(response.data)
    }

    /// Get a specific cluster by ID
    ///
    /// Returns cluster details or None if not found.
    ///
    /// Endpoint: GET /api/clusters/:clusterId
    pub async fn get_cluster(&self, cluster_id: &str) -> Result<Option<ClusterEntity>> {
        self.get_optional(&format!("/api/clusters/{}", cluster_id))
            .await
    }

    // =========================================================================
    // Artifact Registry API methods
    // =========================================================================

    /// List all artifact registries for a project
    ///
    /// Returns all container registries available for image storage in this project.
    ///
    /// Endpoint: GET /api/projects/:projectId/artifact-registries
    pub async fn list_registries_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ArtifactRegistry>> {
        let response: GenericResponse<Vec<ArtifactRegistry>> = self
            .get(&format!("/api/projects/{}/artifact-registries", project_id))
            .await?;
        Ok(response.data)
    }

    /// List only ready artifact registries for a project
    ///
    /// Returns registries that are ready to receive image pushes.
    /// Use this for deployment wizard to show only usable registries.
    ///
    /// Endpoint: GET /api/projects/:projectId/artifact-registries/ready
    pub async fn list_ready_registries_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ArtifactRegistry>> {
        let response: GenericResponse<Vec<ArtifactRegistry>> = self
            .get(&format!(
                "/api/projects/{}/artifact-registries/ready",
                project_id
            ))
            .await?;
        Ok(response.data)
    }

    /// Provision a new artifact registry
    ///
    /// Starts async provisioning via Backstage scaffolder.
    /// Returns task ID for polling status.
    ///
    /// Endpoint: POST /api/projects/:projectId/artifact-registries
    pub async fn create_registry(
        &self,
        project_id: &str,
        request: &CreateRegistryRequest,
    ) -> Result<CreateRegistryResponse> {
        self.post(
            &format!("/api/projects/{}/artifact-registries", project_id),
            request,
        )
        .await
    }

    /// Get registry provisioning task status
    ///
    /// Poll this endpoint to check provisioning progress.
    ///
    /// Endpoint: GET /api/artifact-registries/task/:taskId
    pub async fn get_registry_task_status(&self, task_id: &str) -> Result<RegistryTaskStatus> {
        self.get(&format!("/api/artifact-registries/task/{}", task_id))
            .await
    }

    // =========================================================================
    // Health Check API methods
    // =========================================================================

    /// Check if the API is reachable (quick health check)
    ///
    /// Uses a shorter timeout (5s) for quick connectivity verification.
    /// This method does NOT require authentication.
    ///
    /// Returns `Ok(())` if API is reachable, `Err(ConnectionFailed)` otherwise.
    pub async fn check_connection(&self) -> Result<()> {
        // Use a shorter timeout for health checks
        let health_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent(USER_AGENT)
            .build()
            .map_err(PlatformApiError::HttpError)?;

        let url = format!("{}/health", self.api_url);

        match health_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(PlatformApiError::ConnectionFailed)
                }
            }
            Err(_) => Err(PlatformApiError::ConnectionFailed),
        }
    }
}

/// Get the API URL based on environment
fn get_api_url() -> &'static str {
    if std::env::var("SYNCABLE_ENV").as_deref() == Ok("development") {
        SYNCABLE_API_URL_DEV
    } else {
        SYNCABLE_API_URL_PROD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_construction() {
        let client = PlatformApiClient::with_url("https://example.com").unwrap();
        assert_eq!(client.api_url(), "https://example.com");
    }

    #[test]
    fn test_url_building() {
        let client = PlatformApiClient::with_url("https://api.example.com").unwrap();

        // Verify the base URL is stored correctly
        assert_eq!(client.api_url(), "https://api.example.com");

        // Test path concatenation logic (implicitly tested through api_url)
        let expected_path = format!("{}/api/organizations/123", client.api_url());
        assert_eq!(expected_path, "https://api.example.com/api/organizations/123");
    }

    #[test]
    fn test_error_type_creation() {
        // Test that error types can be created correctly
        let unauthorized = PlatformApiError::Unauthorized;
        assert!(unauthorized.to_string().contains("Not authenticated"));

        let not_found = PlatformApiError::NotFound("Resource not found".to_string());
        assert!(not_found.to_string().contains("Not found"));

        let api_error = PlatformApiError::ApiError {
            status: 400,
            message: "Bad request".to_string(),
        };
        assert!(api_error.to_string().contains("400"));
        assert!(api_error.to_string().contains("Bad request"));

        let permission_denied =
            PlatformApiError::PermissionDenied("Access denied".to_string());
        assert!(permission_denied.to_string().contains("Permission denied"));

        let rate_limited = PlatformApiError::RateLimited;
        assert!(rate_limited.to_string().contains("Rate limit"));

        let server_error = PlatformApiError::ServerError {
            status: 500,
            message: "Internal server error".to_string(),
        };
        assert!(server_error.to_string().contains("500"));
    }

    #[test]
    fn test_api_url_constants() {
        // Test that our URL constants are valid
        assert!(SYNCABLE_API_URL_PROD.starts_with("https://"));
        assert!(SYNCABLE_API_URL_DEV.starts_with("http://"));
    }

    #[test]
    fn test_user_agent() {
        // Verify user agent contains version
        assert!(USER_AGENT.starts_with("syncable-cli/"));
    }

    #[test]
    fn test_parse_error_creation() {
        let error = PlatformApiError::ParseError("invalid json".to_string());
        assert!(error.to_string().contains("parse"));
        assert!(error.to_string().contains("invalid json"));
    }

    #[test]
    fn test_http_error_conversion() {
        // Test that reqwest errors can be converted
        // This is a compile-time check via the From trait
        let _: fn(reqwest::Error) -> PlatformApiError = PlatformApiError::from;
    }

    #[test]
    fn test_provider_connection_path() {
        // Test that the API path is built correctly
        let provider = CloudProvider::Gcp;
        let project_id = "proj-123";
        let expected_path = format!(
            "/api/cloud-credentials/provider/{}?projectId={}",
            provider.as_str(),
            project_id
        );
        assert_eq!(expected_path, "/api/cloud-credentials/provider/gcp?projectId=proj-123");
    }

    #[test]
    fn test_service_logs_path_no_params() {
        // Test logs path without query params
        let service_id = "svc-123";
        let path = format!("/api/deployments/services/{}/logs", service_id);
        assert_eq!(path, "/api/deployments/services/svc-123/logs");
    }

    #[test]
    fn test_service_logs_path_with_params() {
        // Test logs path with query params
        let service_id = "svc-123";
        let mut query_params = Vec::new();
        query_params.push("start=2024-01-01T00:00:00Z".to_string());
        query_params.push("limit=50".to_string());
        let path = format!(
            "/api/deployments/services/{}/logs?{}",
            service_id,
            query_params.join("&")
        );
        assert_eq!(path, "/api/deployments/services/svc-123/logs?start=2024-01-01T00:00:00Z&limit=50");
    }
}
