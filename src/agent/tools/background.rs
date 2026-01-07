//! Background Process Manager
//!
//! Manages long-running background processes like `kubectl port-forward`.
//! Processes run asynchronously and can be started, stopped, and listed.
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::agent::tools::background::BackgroundProcessManager;
//!
//! let manager = BackgroundProcessManager::new();
//!
//! // Start a port-forward in the background
//! let port = manager.start_port_forward(
//!     "prometheus",
//!     "svc/prometheus-server",
//!     "monitoring",
//!     9090
//! ).await?;
//!
//! println!("Port-forward running on localhost:{}", port);
//!
//! // Later, stop it
//! manager.stop("prometheus").await?;
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Error type for background process operations.
#[derive(Debug, thiserror::Error)]
pub enum BackgroundProcessError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    #[error("Process not found: {0}")]
    NotFound(String),

    #[error("Failed to parse port from output: {0}")]
    PortParseFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Process exited unexpectedly: {0}")]
    ProcessExited(String),
}

/// Information about a running background process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Unique identifier for this process
    pub id: String,
    /// The command that was executed
    pub command: String,
    /// When the process was started
    pub started_at: Instant,
    /// Local port (for port-forwards)
    pub local_port: Option<u16>,
    /// Whether the process is still running
    pub is_running: bool,
}

/// Internal state for a background process.
struct BackgroundProcess {
    id: String,
    command: String,
    started_at: Instant,
    local_port: Option<u16>,
    child: Child,
}

/// Manages background processes like port-forwards.
///
/// Thread-safe and designed to be shared across the agent session.
pub struct BackgroundProcessManager {
    processes: Arc<Mutex<HashMap<String, BackgroundProcess>>>,
}

impl Default for BackgroundProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundProcessManager {
    /// Create a new background process manager.
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a kubectl port-forward in the background.
    ///
    /// Returns the local port that was allocated.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this port-forward
    /// * `resource` - Kubernetes resource (e.g., "svc/prometheus-server" or "pod/prometheus-0")
    /// * `namespace` - Kubernetes namespace
    /// * `target_port` - The port on the remote resource
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let port = manager.start_port_forward(
    ///     "prometheus",
    ///     "svc/prometheus-server",
    ///     "monitoring",
    ///     9090
    /// ).await?;
    /// ```
    pub async fn start_port_forward(
        &self,
        id: &str,
        resource: &str,
        namespace: &str,
        target_port: u16,
    ) -> Result<u16, BackgroundProcessError> {
        // Check if already running
        {
            let processes = self.processes.lock().await;
            if processes.contains_key(id) {
                if let Some(proc) = processes.get(id) {
                    if let Some(port) = proc.local_port {
                        return Ok(port);
                    }
                }
            }
        }

        // Build the port-forward command
        // Using :0 to let kubectl pick a random available port
        let port_spec = format!(":{}", target_port);
        let command = format!(
            "kubectl port-forward {} {} -n {}",
            resource, port_spec, namespace
        );

        // Spawn kubectl directly (not through sh) to avoid process hierarchy issues
        let mut child = Command::new("kubectl")
            .arg("port-forward")
            .arg(resource)
            .arg(&port_spec)
            .arg("-n")
            .arg(namespace)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| BackgroundProcessError::SpawnFailed(e.to_string()))?;

        // Take stderr for error capturing
        let stderr = child.stderr.take();

        // Read stdout to get the port
        // kubectl outputs: "Forwarding from 127.0.0.1:XXXXX -> 9090" to stdout
        let local_port = if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout).lines();
            let mut port = None;

            // Read lines with timeout
            let timeout = tokio::time::Duration::from_secs(10);
            let deadline = tokio::time::Instant::now() + timeout;

            while tokio::time::Instant::now() < deadline {
                match tokio::time::timeout(
                    deadline - tokio::time::Instant::now(),
                    reader.next_line(),
                )
                .await
                {
                    Ok(Ok(Some(line))) => {
                        // Parse port from "Forwarding from 127.0.0.1:XXXXX -> 9090"
                        if line.contains("Forwarding from") {
                            if let Some(port_str) = line
                                .split(':')
                                .nth(1)
                                .and_then(|s| s.split_whitespace().next())
                            {
                                port = port_str.parse().ok();
                                // Keep draining stdout in background to prevent SIGPIPE
                                tokio::spawn(async move {
                                    while let Ok(Some(_)) = reader.next_line().await {}
                                });
                                break;
                            }
                        }
                    }
                    Ok(Ok(None)) => break, // EOF
                    Ok(Err(e)) => {
                        return Err(BackgroundProcessError::IoError(e));
                    }
                    Err(_) => {
                        // Timeout - process may still be starting
                        break;
                    }
                }
            }

            port
        } else {
            None
        };

        // If we couldn't get the port, try to capture stderr for better error messages
        let local_port = match local_port {
            Some(p) => p,
            None => {
                // Try to read stderr for error messages
                let error_msg = if let Some(stderr) = stderr {
                    let mut reader = BufReader::new(stderr).lines();
                    let mut errors = Vec::new();
                    while let Ok(Ok(Some(line))) = tokio::time::timeout(
                        tokio::time::Duration::from_millis(100),
                        reader.next_line(),
                    )
                    .await
                    {
                        if !line.is_empty() {
                            errors.push(line);
                        }
                    }
                    if errors.is_empty() {
                        "Could not determine local port (no output from kubectl)".to_string()
                    } else {
                        errors.join("; ")
                    }
                } else {
                    "Could not determine local port".to_string()
                };
                return Err(BackgroundProcessError::PortParseFailed(error_msg));
            }
        };

        // Store the process
        let mut processes = self.processes.lock().await;
        processes.insert(
            id.to_string(),
            BackgroundProcess {
                id: id.to_string(),
                command,
                started_at: Instant::now(),
                local_port: Some(local_port),
                child,
            },
        );

        Ok(local_port)
    }

    /// Start a generic background command.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this process
    /// * `command` - The command to execute
    /// * `working_dir` - Working directory for the command
    pub async fn start(
        &self,
        id: &str,
        command: &str,
        working_dir: &Path,
    ) -> Result<(), BackgroundProcessError> {
        // Check if already running
        {
            let processes = self.processes.lock().await;
            if processes.contains_key(id) {
                return Ok(()); // Already running
            }
        }

        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| BackgroundProcessError::SpawnFailed(e.to_string()))?;

        let mut processes = self.processes.lock().await;
        processes.insert(
            id.to_string(),
            BackgroundProcess {
                id: id.to_string(),
                command: command.to_string(),
                started_at: Instant::now(),
                local_port: None,
                child,
            },
        );

        Ok(())
    }

    /// Stop a background process by ID.
    pub async fn stop(&self, id: &str) -> Result<(), BackgroundProcessError> {
        let mut processes = self.processes.lock().await;
        if let Some(mut proc) = processes.remove(id) {
            // Try graceful shutdown first
            let _ = proc.child.kill().await;
        }
        Ok(())
    }

    /// Check if a process is running.
    pub async fn is_running(&self, id: &str) -> bool {
        let mut processes = self.processes.lock().await;
        if let Some(proc) = processes.get_mut(id) {
            // Check if process is still alive
            match proc.child.try_wait() {
                Ok(None) => true, // Still running
                Ok(Some(_)) => {
                    // Process exited, clean up
                    processes.remove(id);
                    false
                }
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Get information about a specific process.
    pub async fn get(&self, id: &str) -> Option<ProcessInfo> {
        let mut processes = self.processes.lock().await;
        if let Some(proc) = processes.get_mut(id) {
            let is_running = proc
                .child
                .try_wait()
                .ok()
                .map(|s| s.is_none())
                .unwrap_or(false);
            Some(ProcessInfo {
                id: proc.id.clone(),
                command: proc.command.clone(),
                started_at: proc.started_at,
                local_port: proc.local_port,
                is_running,
            })
        } else {
            None
        }
    }

    /// List all background processes.
    pub async fn list(&self) -> Vec<ProcessInfo> {
        let mut processes = self.processes.lock().await;
        let mut infos = Vec::new();
        let mut to_remove = Vec::new();

        for (id, proc) in processes.iter_mut() {
            let is_running = proc
                .child
                .try_wait()
                .ok()
                .map(|s| s.is_none())
                .unwrap_or(false);
            if !is_running {
                to_remove.push(id.clone());
            }
            infos.push(ProcessInfo {
                id: proc.id.clone(),
                command: proc.command.clone(),
                started_at: proc.started_at,
                local_port: proc.local_port,
                is_running,
            });
        }

        // Clean up exited processes
        for id in to_remove {
            processes.remove(&id);
        }

        infos
    }

    /// Stop all background processes.
    pub async fn stop_all(&self) {
        let mut processes = self.processes.lock().await;
        for (_, mut proc) in processes.drain() {
            let _ = proc.child.kill().await;
        }
    }
}

impl Drop for BackgroundProcessManager {
    fn drop(&mut self) {
        // Note: We can't await here, so we use blocking
        // In practice, the manager should be stopped explicitly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager() {
        let manager = BackgroundProcessManager::new();
        assert!(manager.processes.try_lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_empty() {
        let manager = BackgroundProcessManager::new();
        let list = manager.list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_is_running_not_found() {
        let manager = BackgroundProcessManager::new();
        assert!(!manager.is_running("nonexistent").await);
    }
}
