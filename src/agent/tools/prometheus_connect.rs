//! Prometheus Connect Tool
//!
//! Establishes a connection to Prometheus via port-forward (preferred) or direct URL.
//! Used after prometheus_discover to set up the connection for k8s_optimize.
//!
//! # Connection Methods
//!
//! 1. **Port-forward (preferred)** - No authentication needed
//!    - Connects directly to the pod, bypassing ingress/auth
//!    - Works with any in-cluster Prometheus
//!
//! 2. **Direct URL** - May require authentication
//!    - For externally exposed Prometheus
//!    - Supports Basic auth and Bearer token

use super::background::BackgroundProcessManager;
use super::error::{ErrorCategory, format_error_for_llm};
use crate::agent::ui::prometheus_display::{ConnectionMode, PrometheusConnectionDisplay};
use crate::analyzer::k8s_optimize::{PrometheusAuth, PrometheusClient};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

/// Arguments for the prometheus_connect tool
#[derive(Debug, Deserialize)]
pub struct PrometheusConnectArgs {
    /// Service name from discovery (for port-forward)
    #[serde(default)]
    pub service: Option<String>,

    /// Namespace (for port-forward)
    #[serde(default)]
    pub namespace: Option<String>,

    /// External URL (alternative to service discovery)
    #[serde(default)]
    pub url: Option<String>,

    /// Port (default: 9090)
    #[serde(default)]
    pub port: Option<u16>,

    /// Authentication type: "none", "basic", "bearer" (only for external URL)
    #[serde(default)]
    pub auth_type: Option<String>,

    /// Username for basic auth (only for external URL)
    #[serde(default)]
    pub username: Option<String>,

    /// Password for basic auth (only for external URL)
    #[serde(default)]
    pub password: Option<String>,

    /// Bearer token (only for external URL)
    #[serde(default)]
    pub token: Option<String>,
}

/// Error type for prometheus connection
#[derive(Debug, thiserror::Error)]
#[error("Prometheus connect error: {0}")]
pub struct PrometheusConnectError(String);

/// Tool for connecting to Prometheus
#[derive(Clone)]
pub struct PrometheusConnectTool {
    bg_manager: Arc<BackgroundProcessManager>,
}

impl PrometheusConnectTool {
    /// Create a new PrometheusConnectTool with shared background process manager
    pub fn new(bg_manager: Arc<BackgroundProcessManager>) -> Self {
        Self { bg_manager }
    }

    /// Validate port range (1-65535)
    fn validate_port(port: u16) -> Result<(), String> {
        if port == 0 {
            return Err("Port must be between 1 and 65535 (got 0)".to_string());
        }
        Ok(())
    }

    /// Validate URL format (must start with http:// or https://)
    fn validate_url(url: &str) -> Result<(), String> {
        let url_lower = url.to_lowercase();
        if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
            return Err(format!(
                "URL must start with http:// or https:// (got '{}')",
                url
            ));
        }
        Ok(())
    }

    /// Build auth from args
    fn build_auth(args: &PrometheusConnectArgs) -> PrometheusAuth {
        match args.auth_type.as_deref() {
            Some("basic") => {
                if let (Some(username), Some(password)) = (&args.username, &args.password) {
                    PrometheusAuth::Basic {
                        username: username.clone(),
                        password: password.clone(),
                    }
                } else {
                    PrometheusAuth::None
                }
            }
            Some("bearer") => {
                if let Some(token) = &args.token {
                    PrometheusAuth::Bearer(token.clone())
                } else {
                    PrometheusAuth::None
                }
            }
            _ => PrometheusAuth::None,
        }
    }

    /// Test if a Prometheus URL is reachable
    async fn test_connection(url: &str, auth: PrometheusAuth) -> bool {
        match PrometheusClient::with_auth(url, auth) {
            Ok(client) => client.is_available().await,
            Err(_) => false,
        }
    }
}

impl Tool for PrometheusConnectTool {
    const NAME: &'static str = "prometheus_connect";

    type Args = PrometheusConnectArgs;
    type Output = String;
    type Error = PrometheusConnectError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Connect to Prometheus for K8s optimization analysis.

**Use after prometheus_discover or when user provides a URL.**

**Connection Methods (in order of preference):**

1. **Port-forward** (recommended) - NO authentication needed
   - Provide: service, namespace, port
   - Starts kubectl port-forward in background
   - Direct pod connection bypasses auth

2. **External URL** - May require authentication
   - Provide: url
   - Optional: auth_type, username/password or token

**Examples:**

Port-forward (no auth):
```json
{"service": "prometheus-server", "namespace": "monitoring", "port": 9090}
```

External URL without auth:
```json
{"url": "http://prometheus.example.com"}
```

External URL with basic auth:
```json
{"url": "https://prometheus.example.com", "auth_type": "basic", "username": "admin", "password": "secret"}
```

**Returns:**
- Connection URL for use with k8s_optimize
- Connection mode (port-forward or direct)
- Local port (if port-forward)"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Kubernetes service name (for port-forward)"
                    },
                    "namespace": {
                        "type": "string",
                        "description": "Kubernetes namespace (for port-forward)"
                    },
                    "url": {
                        "type": "string",
                        "description": "External Prometheus URL (alternative to port-forward)"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Target port (default: 9090)"
                    },
                    "auth_type": {
                        "type": "string",
                        "description": "Authentication type for external URL: 'none', 'basic', 'bearer'",
                        "enum": ["none", "basic", "bearer"]
                    },
                    "username": {
                        "type": "string",
                        "description": "Username for basic auth (only for external URL)"
                    },
                    "password": {
                        "type": "string",
                        "description": "Password for basic auth (only for external URL)"
                    },
                    "token": {
                        "type": "string",
                        "description": "Bearer token (only for external URL)"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate port if provided
        if let Some(port) = args.port {
            if let Err(e) = Self::validate_port(port) {
                return Ok(format_error_for_llm(
                    "prometheus_connect",
                    ErrorCategory::ValidationFailed,
                    &e,
                    Some(vec![
                        "Port must be a valid TCP port between 1 and 65535",
                        "Common Prometheus port is 9090 (default if not specified)",
                    ]),
                ));
            }
        }

        // Validate URL format if provided
        if let Some(ref url) = args.url {
            if let Err(e) = Self::validate_url(url) {
                return Ok(format_error_for_llm(
                    "prometheus_connect",
                    ErrorCategory::ValidationFailed,
                    &e,
                    Some(vec![
                        "URL must start with http:// or https://",
                        "Example: http://prometheus.example.com or https://prometheus.example.com",
                    ]),
                ));
            }
        }

        let target_port = args.port.unwrap_or(9090);

        // PREFERRED: Port-forward (no auth needed)
        if let (Some(service), Some(namespace)) = (&args.service, &args.namespace) {
            let resource = format!("svc/{}", service);
            let display = PrometheusConnectionDisplay::new(ConnectionMode::PortForward);
            let target = format!("{}/{}", namespace, service);
            display.start(&target);

            // Start port-forward in background
            match self
                .bg_manager
                .start_port_forward("prometheus-port-forward", &resource, namespace, target_port)
                .await
            {
                Ok(local_port) => {
                    let url = format!("http://localhost:{}", local_port);
                    display.port_forward_established(local_port, service, namespace);

                    // Wait for port-forward to fully establish (tunnel needs time)
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    display.testing_connection();

                    // Test connection with retries (port-forward can take time to be ready)
                    let mut connected = false;
                    for attempt in 0..6 {
                        if Self::test_connection(&url, PrometheusAuth::None).await {
                            connected = true;
                            break;
                        }
                        // Backoff: 1s, 1s, 2s, 2s, 3s
                        let delay = match attempt {
                            0 | 1 => 1000,
                            2 | 3 => 2000,
                            _ => 3000,
                        };
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }

                    if connected {
                        display.connected(&url, false);
                        display.background_process_info("prometheus-port-forward");
                        display.ready_for_use(&url);

                        let response = json!({
                            "connected": true,
                            "url": url,
                            "mode": "port-forward",
                            "local_port": local_port,
                            "service": service,
                            "namespace": namespace,
                            "process_id": "prometheus-port-forward",
                            "note": "Port-forward established. No authentication needed.",
                            "usage": {
                                "k8s_optimize": {
                                    "prometheus": url
                                }
                            }
                        });
                        return Ok(serde_json::to_string_pretty(&response)
                            .unwrap_or_else(|_| "{}".to_string()));
                    } else {
                        // Still can't connect - stop the failed port-forward
                        let _ = self.bg_manager.stop("prometheus-port-forward").await;

                        display.connection_failed(
                            "Port-forward started but Prometheus not responding",
                            &[
                                "Verify the service is correct",
                                "Check if Prometheus pod is running",
                                "The service might need more time to start",
                            ],
                        );

                        return Ok(format_error_for_llm(
                            "prometheus_connect",
                            ErrorCategory::NetworkError,
                            "Port-forward started but Prometheus not responding",
                            Some(vec![
                                &format!(
                                    "Verify the service is correct: kubectl get svc -n {}",
                                    namespace
                                ),
                                &format!(
                                    "Check if Prometheus pod is running: kubectl get pods -n {} | grep prometheus",
                                    namespace
                                ),
                                "The service might need more time to start - try again in a few seconds",
                            ]),
                        ));
                    }
                }
                Err(e) => {
                    // Port-forward failed
                    display.connection_failed(
                        &format!("Port-forward failed: {}", e),
                        &[
                            "Check if kubectl is configured correctly",
                            "Verify the service exists",
                            "Try providing an external URL instead",
                        ],
                    );

                    return Ok(format_error_for_llm(
                        "prometheus_connect",
                        ErrorCategory::ExternalCommandFailed,
                        &format!("Port-forward failed: {}", e),
                        Some(vec![
                            "Check if kubectl is configured correctly: kubectl config current-context",
                            &format!(
                                "Verify the service exists: kubectl get svc -n {}",
                                namespace
                            ),
                            "Try providing an external URL instead",
                        ]),
                    ));
                }
            }
        }

        // FALLBACK: External URL
        if let Some(url) = &args.url {
            let display = PrometheusConnectionDisplay::new(ConnectionMode::DirectUrl);
            display.start(url);
            display.testing_connection();

            // First try without auth
            if Self::test_connection(url, PrometheusAuth::None).await {
                display.connected(url, false);
                display.ready_for_use(url);

                let response = json!({
                    "connected": true,
                    "url": url,
                    "mode": "direct",
                    "authenticated": false,
                    "note": "Connected without authentication",
                    "usage": {
                        "k8s_optimize": {
                            "prometheus": url
                        }
                    }
                });
                return Ok(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string())
                );
            }

            // If that fails and auth was provided, try with auth
            let auth = Self::build_auth(&args);
            if !matches!(auth, PrometheusAuth::None) && Self::test_connection(url, auth).await {
                display.connected(url, true);
                display.ready_for_use(url);

                let response = json!({
                    "connected": true,
                    "url": url,
                    "mode": "direct",
                    "authenticated": true,
                    "auth_type": args.auth_type,
                    "note": "Connected with authentication",
                    "usage": {
                        "k8s_optimize": {
                            "prometheus": url,
                            "auth_type": args.auth_type,
                            "username": args.username,
                            // Don't include password/token in response for security
                        }
                    }
                });
                return Ok(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string())
                );
            }

            // Connection failed - show auth hint if no auth was tried
            if args.auth_type.is_none() {
                display.auth_required();

                display.connection_failed(
                    "Connection failed - URL may require authentication",
                    &[
                        "Try with auth_type='basic' and username/password",
                        "Or try auth_type='bearer' with a token",
                        "Verify the URL is correct and accessible",
                    ],
                );

                let test_url_suggestion =
                    format!("Test URL manually: curl -s {}/api/v1/status/config", url);
                return Ok(format_error_for_llm(
                    "prometheus_connect",
                    ErrorCategory::NetworkError,
                    "Connection failed - URL may require authentication",
                    Some(vec![
                        "Try with auth_type='basic' and username/password",
                        "Or try auth_type='bearer' with a token",
                        "Verify the URL is correct and accessible",
                        &test_url_suggestion,
                    ]),
                ));
            } else {
                display.connection_failed(
                    "Connection failed - authentication credentials may be incorrect",
                    &[
                        "Verify the username/password or token",
                        "Check if the auth_type matches what the server expects",
                        "Ensure the user has permission to access Prometheus API",
                    ],
                );

                return Ok(format_error_for_llm(
                    "prometheus_connect",
                    ErrorCategory::NetworkError,
                    "Connection failed - authentication credentials may be incorrect",
                    Some(vec![
                        "Verify the username/password or token",
                        "Check if the auth_type matches what the server expects",
                        "Ensure the user has permission to access Prometheus API",
                    ]),
                ));
            }
        }

        // No service or URL provided
        Ok(format_error_for_llm(
            "prometheus_connect",
            ErrorCategory::ValidationFailed,
            "No service or URL provided",
            Some(vec![
                "Provide service + namespace for port-forward: {\"service\": \"prometheus-server\", \"namespace\": \"monitoring\"}",
                "Or provide url for external Prometheus: {\"url\": \"http://prometheus.example.com\"}",
                "Use prometheus_discover to find available Prometheus instances",
            ]),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(PrometheusConnectTool::NAME, "prometheus_connect");
    }

    #[test]
    fn test_validate_port_valid() {
        assert!(PrometheusConnectTool::validate_port(9090).is_ok());
        assert!(PrometheusConnectTool::validate_port(1).is_ok());
        assert!(PrometheusConnectTool::validate_port(65535).is_ok());
    }

    #[test]
    fn test_validate_port_invalid() {
        let result = PrometheusConnectTool::validate_port(0);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Port must be between 1 and 65535")
        );
    }

    #[test]
    fn test_validate_url_valid() {
        assert!(PrometheusConnectTool::validate_url("http://prometheus.example.com").is_ok());
        assert!(PrometheusConnectTool::validate_url("https://prometheus.example.com").is_ok());
        assert!(PrometheusConnectTool::validate_url("HTTP://PROMETHEUS.EXAMPLE.COM").is_ok());
        assert!(PrometheusConnectTool::validate_url("HTTPS://prometheus.example.com").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        // Missing protocol
        let result = PrometheusConnectTool::validate_url("prometheus.example.com");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("must start with http:// or https://")
        );

        // Wrong protocol
        let result = PrometheusConnectTool::validate_url("ftp://prometheus.example.com");
        assert!(result.is_err());

        // Just a path
        let result = PrometheusConnectTool::validate_url("/api/v1/query");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_service_and_url_error() {
        // Test that calling with no service and no URL returns structured error
        let bg_manager = Arc::new(BackgroundProcessManager::new());
        let tool = PrometheusConnectTool::new(bg_manager);

        let args = PrometheusConnectArgs {
            service: None,
            namespace: None,
            url: None,
            port: None,
            auth_type: None,
            username: None,
            password: None,
            token: None,
        };

        let result = tool.call(args).await.unwrap();

        // Verify the result is a structured error
        assert!(result.contains("\"error\": true"));
        assert!(result.contains("VALIDATION_FAILED"));
        assert!(result.contains("No service or URL provided"));
        assert!(result.contains("suggestions"));
    }

    #[tokio::test]
    async fn test_invalid_port_validation() {
        // Test that invalid port (0) returns validation error
        let bg_manager = Arc::new(BackgroundProcessManager::new());
        let tool = PrometheusConnectTool::new(bg_manager);

        let args = PrometheusConnectArgs {
            service: Some("prometheus".to_string()),
            namespace: Some("monitoring".to_string()),
            url: None,
            port: Some(0), // Invalid port
            auth_type: None,
            username: None,
            password: None,
            token: None,
        };

        let result = tool.call(args).await.unwrap();

        // Verify the result is a structured error
        assert!(result.contains("\"error\": true"));
        assert!(result.contains("VALIDATION_FAILED"));
        assert!(result.contains("Port must be between 1 and 65535"));
    }

    #[tokio::test]
    async fn test_malformed_url_validation() {
        // Test that URL without http(s):// returns helpful error
        let bg_manager = Arc::new(BackgroundProcessManager::new());
        let tool = PrometheusConnectTool::new(bg_manager);

        let args = PrometheusConnectArgs {
            service: None,
            namespace: None,
            url: Some("prometheus.example.com".to_string()), // Missing protocol
            port: None,
            auth_type: None,
            username: None,
            password: None,
            token: None,
        };

        let result = tool.call(args).await.unwrap();

        // Verify the result is a structured error
        assert!(result.contains("\"error\": true"));
        assert!(result.contains("VALIDATION_FAILED"));
        assert!(result.contains("must start with http:// or https://"));
        assert!(result.contains("suggestions"));
    }

    #[test]
    fn test_build_auth_none() {
        let args = PrometheusConnectArgs {
            service: None,
            namespace: None,
            url: Some("http://localhost".to_string()),
            port: None,
            auth_type: None,
            username: None,
            password: None,
            token: None,
        };

        let auth = PrometheusConnectTool::build_auth(&args);
        assert!(matches!(auth, PrometheusAuth::None));
    }

    #[test]
    fn test_build_auth_basic() {
        let args = PrometheusConnectArgs {
            service: None,
            namespace: None,
            url: Some("http://localhost".to_string()),
            port: None,
            auth_type: Some("basic".to_string()),
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
            token: None,
        };

        let auth = PrometheusConnectTool::build_auth(&args);
        match auth {
            PrometheusAuth::Basic { username, password } => {
                assert_eq!(username, "admin");
                assert_eq!(password, "secret");
            }
            _ => panic!("Expected Basic auth"),
        }
    }

    #[test]
    fn test_build_auth_bearer() {
        let args = PrometheusConnectArgs {
            service: None,
            namespace: None,
            url: Some("http://localhost".to_string()),
            port: None,
            auth_type: Some("bearer".to_string()),
            username: None,
            password: None,
            token: Some("mytoken".to_string()),
        };

        let auth = PrometheusConnectTool::build_auth(&args);
        match auth {
            PrometheusAuth::Bearer(token) => {
                assert_eq!(token, "mytoken");
            }
            _ => panic!("Expected Bearer auth"),
        }
    }
}
