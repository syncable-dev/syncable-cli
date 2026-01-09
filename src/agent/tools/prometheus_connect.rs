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

                        let response = json!({
                            "connected": false,
                            "url": url,
                            "mode": "port-forward",
                            "local_port": local_port,
                            "error": "Port-forward started but Prometheus not responding",
                            "suggestions": [
                                format!("Verify the service is correct with: kubectl get svc -n {}", namespace),
                                format!("Check if Prometheus pod is running: kubectl get pods -n {} | grep prometheus", namespace),
                                "The service might need more time to start".to_string()
                            ]
                        });
                        return Ok(serde_json::to_string_pretty(&response)
                            .unwrap_or_else(|_| "{}".to_string()));
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

                    let response = json!({
                        "connected": false,
                        "mode": "port-forward",
                        "error": format!("Port-forward failed: {}", e),
                        "suggestions": [
                            "Check if kubectl is configured correctly",
                            format!("Verify the service exists: kubectl get svc -n {}", namespace),
                            "Try providing an external URL instead"
                        ]
                    });
                    return Ok(serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| "{}".to_string()));
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
            if !matches!(auth, PrometheusAuth::None) {
                if Self::test_connection(url, auth).await {
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
                    return Ok(serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| "{}".to_string()));
                }
            }

            // Connection failed - show auth hint if no auth was tried
            if args.auth_type.is_none() {
                display.auth_required();
            }

            display.connection_failed(
                "Connection failed",
                if args.auth_type.is_none() {
                    &[
                        "The URL might require authentication",
                        "Try with auth_type='basic' or 'bearer'",
                        "Verify the URL is correct and accessible",
                    ]
                } else {
                    &[
                        "Authentication credentials might be incorrect",
                        "Verify the username/password or token",
                        "Check if the auth_type matches what the server expects",
                    ]
                },
            );

            let response = json!({
                "connected": false,
                "url": url,
                "mode": "direct",
                "error": "Connection failed",
                "suggestions": if args.auth_type.is_none() {
                    vec![
                        "The URL might require authentication",
                        "Try with auth_type='basic' and username/password",
                        "Or try auth_type='bearer' with a token",
                        "Verify the URL is correct and accessible"
                    ]
                } else {
                    vec![
                        "Authentication credentials might be incorrect",
                        "Verify the username/password or token",
                        "Check if the auth_type matches what the server expects"
                    ]
                }
            });
            return Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string()));
        }

        // No service or URL provided
        let response = json!({
            "connected": false,
            "error": "No service or URL provided",
            "hint": "Either provide service+namespace for port-forward, or provide a URL",
            "examples": [
                {
                    "port-forward": {
                        "service": "prometheus-server",
                        "namespace": "monitoring",
                        "port": 9090
                    }
                },
                {
                    "external": {
                        "url": "http://prometheus.example.com"
                    }
                }
            ]
        });
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string()))
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
