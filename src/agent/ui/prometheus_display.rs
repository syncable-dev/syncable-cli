//! Prometheus Discovery & Connection Display
//!
//! Elegant terminal UI for Prometheus operations:
//! - Service discovery in Kubernetes cluster
//! - Port-forward connection establishment
//! - Connection status and health checks
//!
//! Uses a visual style consistent with other tool displays.

use crate::agent::ui::colors::{ansi, icons};
use colored::Colorize;
use std::io::{self, Write};

/// Icon for Prometheus (fire/metrics theme)
pub const PROMETHEUS_ICON: &str = "🔥";
/// Icon for Kubernetes
pub const K8S_ICON: &str = "☸";
/// Icon for network/connection
pub const NETWORK_ICON: &str = "🔗";
/// Icon for port-forward
pub const PORT_FORWARD_ICON: &str = "🚇";
/// Icon for search/discovery
pub const SEARCH_ICON: &str = "🔍";

/// Display for Prometheus discovery operations
pub struct PrometheusDiscoveryDisplay {
    started: bool,
}

impl PrometheusDiscoveryDisplay {
    pub fn new() -> Self {
        Self { started: false }
    }

    /// Show discovery started
    pub fn start(&mut self, namespace: Option<&str>) {
        self.started = true;
        let scope = namespace.unwrap_or("all namespaces");

        println!();
        println!(
            "{}{}  Prometheus Discovery{}",
            ansi::BOLD,
            PROMETHEUS_ICON,
            ansi::RESET
        );
        println!(
            "{}├─{} {} Searching for Prometheus services in {}...{}",
            ansi::DIM,
            ansi::RESET,
            SEARCH_ICON,
            scope.cyan(),
            ansi::RESET
        );
        let _ = io::stdout().flush();
    }

    /// Show services found
    pub fn found_services(&self, services: &[DiscoveredService]) {
        if services.is_empty() {
            println!(
                "{}├─{} {} {}{}",
                ansi::DIM,
                ansi::RESET,
                icons::WARNING.yellow(),
                "No Prometheus services found".yellow(),
                ansi::RESET
            );
        } else {
            println!(
                "{}├─{} {} Found {} service(s):{}",
                ansi::DIM,
                ansi::RESET,
                icons::SUCCESS.green(),
                services.len().to_string().green().bold(),
                ansi::RESET
            );

            for (i, svc) in services.iter().enumerate() {
                let is_last = i == services.len() - 1;
                let prefix = if is_last { "└─" } else { "├─" };

                println!(
                    "{}│  {}─{} {} {}/{} {}:{}{}",
                    ansi::DIM,
                    prefix,
                    ansi::RESET,
                    K8S_ICON,
                    svc.namespace.cyan(),
                    svc.name.cyan().bold(),
                    "port".dimmed(),
                    svc.port.to_string().yellow(),
                    ansi::RESET
                );
            }
        }
        let _ = io::stdout().flush();
    }

    /// Show suggestion for next step
    pub fn show_suggestion(&self, service: &DiscoveredService) {
        println!("{}│{}", ansi::DIM, ansi::RESET);
        println!(
            "{}└─{} {} Next: Use {} to connect{}",
            ansi::DIM,
            ansi::RESET,
            icons::ARROW.cyan(),
            "prometheus_connect".cyan().bold(),
            ansi::RESET
        );
        println!(
            "   {} service: {}, namespace: {}, port: {}",
            "→".dimmed(),
            service.name.green(),
            service.namespace.green(),
            service.port.to_string().yellow()
        );
        let _ = io::stdout().flush();
    }

    /// Show fallback to all namespaces
    pub fn searching_all_namespaces(&self) {
        println!(
            "{}├─{} {} {}{}",
            ansi::DIM,
            ansi::RESET,
            SEARCH_ICON,
            "Not found in specified namespace, searching all namespaces...".yellow(),
            ansi::RESET
        );
        let _ = io::stdout().flush();
    }

    /// Show error
    pub fn error(&self, message: &str) {
        println!(
            "{}└─{} {} {}{}",
            ansi::DIM,
            ansi::RESET,
            icons::ERROR.red(),
            message.red(),
            ansi::RESET
        );
        let _ = io::stdout().flush();
    }
}

impl Default for PrometheusDiscoveryDisplay {
    fn default() -> Self {
        Self::new()
    }
}

/// A discovered Prometheus service (for display)
#[derive(Debug, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub namespace: String,
    pub port: u16,
    pub service_type: String,
}

/// Display for Prometheus connection operations
pub struct PrometheusConnectionDisplay {
    mode: ConnectionMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionMode {
    PortForward,
    DirectUrl,
}

impl PrometheusConnectionDisplay {
    pub fn new(mode: ConnectionMode) -> Self {
        Self { mode }
    }

    /// Show connection started
    pub fn start(&self, target: &str) {
        println!();
        println!(
            "{}{}  Prometheus Connection{}",
            ansi::BOLD,
            NETWORK_ICON,
            ansi::RESET
        );

        match self.mode {
            ConnectionMode::PortForward => {
                println!(
                    "{}├─{} {} Establishing port-forward to {}...{}",
                    ansi::DIM,
                    ansi::RESET,
                    PORT_FORWARD_ICON,
                    target.cyan(),
                    ansi::RESET
                );
            }
            ConnectionMode::DirectUrl => {
                println!(
                    "{}├─{} {} Connecting to {}...{}",
                    ansi::DIM,
                    ansi::RESET,
                    NETWORK_ICON,
                    target.cyan(),
                    ansi::RESET
                );
            }
        }
        let _ = io::stdout().flush();
    }

    /// Show port-forward established
    pub fn port_forward_established(&self, local_port: u16, service: &str, namespace: &str) {
        println!(
            "{}├─{} {} Port-forward active on localhost:{}{}",
            ansi::DIM,
            ansi::RESET,
            icons::SUCCESS.green(),
            local_port.to_string().green().bold(),
            ansi::RESET
        );
        println!(
            "{}│  {} {} {}/{} {}",
            ansi::DIM,
            ansi::RESET,
            "→".dimmed(),
            namespace.dimmed(),
            service.dimmed(),
            "(no auth needed)".dimmed()
        );
        let _ = io::stdout().flush();
    }

    /// Show testing connection
    pub fn testing_connection(&self) {
        print!(
            "{}├─{} {} Testing Prometheus API...{}",
            ansi::DIM,
            ansi::RESET,
            icons::EXECUTING.cyan(),
            ansi::RESET
        );
        let _ = io::stdout().flush();
    }

    /// Show connection successful
    pub fn connected(&self, url: &str, authenticated: bool) {
        // Clear the "Testing..." line
        print!("\r{}", ansi::CLEAR_LINE);

        println!(
            "{}├─{} {} Connection established{}",
            ansi::DIM,
            ansi::RESET,
            icons::SUCCESS.green(),
            ansi::RESET
        );

        let auth_status = if authenticated {
            "(authenticated)".green()
        } else {
            "(no auth)".dimmed()
        };

        println!(
            "{}│  {} URL: {} {}{}",
            ansi::DIM,
            ansi::RESET,
            url.cyan(),
            auth_status,
            ansi::RESET
        );
        let _ = io::stdout().flush();
    }

    /// Show connection ready for use
    pub fn ready_for_use(&self, url: &str) {
        println!("{}│{}", ansi::DIM, ansi::RESET);
        println!(
            "{}└─{} {} Ready! Use with {}{}",
            ansi::DIM,
            ansi::RESET,
            PROMETHEUS_ICON,
            "k8s_optimize".cyan().bold(),
            ansi::RESET
        );
        println!("   {} prometheus: \"{}\"", "→".dimmed(), url.green());
        let _ = io::stdout().flush();
    }

    /// Show connection failed
    pub fn connection_failed(&self, error: &str, suggestions: &[&str]) {
        // Clear any pending line
        print!("\r{}", ansi::CLEAR_LINE);

        println!(
            "{}├─{} {} Connection failed: {}{}",
            ansi::DIM,
            ansi::RESET,
            icons::ERROR.red(),
            error.red(),
            ansi::RESET
        );

        if !suggestions.is_empty() {
            println!("{}│{}", ansi::DIM, ansi::RESET);
            println!("{}├─{} Suggestions:{}", ansi::DIM, ansi::RESET, ansi::RESET);

            for (i, suggestion) in suggestions.iter().enumerate() {
                let is_last = i == suggestions.len() - 1;
                let prefix = if is_last { "└─" } else { "├─" };

                println!(
                    "{}│  {}─{} {}{}",
                    ansi::DIM,
                    prefix,
                    ansi::RESET,
                    suggestion.yellow(),
                    ansi::RESET
                );
            }
        }
        let _ = io::stdout().flush();
    }

    /// Show auth required message
    pub fn auth_required(&self) {
        println!(
            "{}├─{} {} {}{}",
            ansi::DIM,
            ansi::RESET,
            icons::SECURITY.yellow(),
            "Authentication may be required for external Prometheus".yellow(),
            ansi::RESET
        );
        println!(
            "{}│  {} Provide auth_type: \"basic\" or \"bearer\"{}",
            ansi::DIM,
            "→".dimmed(),
            ansi::RESET
        );
        let _ = io::stdout().flush();
    }

    /// Show background process info
    pub fn background_process_info(&self, process_id: &str) {
        println!(
            "{}│  {} Background process: {} {}",
            ansi::DIM,
            ansi::RESET,
            process_id.dimmed(),
            "(will auto-cleanup)".dimmed()
        );
        let _ = io::stdout().flush();
    }
}

/// Compact inline display for tool calls
pub struct PrometheusInlineDisplay;

impl PrometheusInlineDisplay {
    /// Show discovery inline
    pub fn discovery_start() {
        print!(
            "{} {} Discovering Prometheus services...",
            icons::EXECUTING.cyan(),
            PROMETHEUS_ICON
        );
        let _ = io::stdout().flush();
    }

    /// Update discovery result
    pub fn discovery_result(count: usize) {
        print!("\r{}", ansi::CLEAR_LINE);
        if count > 0 {
            println!(
                "{} {} Found {} Prometheus service(s)",
                icons::SUCCESS.green(),
                PROMETHEUS_ICON,
                count.to_string().green().bold()
            );
        } else {
            println!(
                "{} {} No Prometheus services found",
                icons::WARNING.yellow(),
                PROMETHEUS_ICON
            );
        }
        let _ = io::stdout().flush();
    }

    /// Show connection inline
    pub fn connect_start(target: &str) {
        print!(
            "{} {} Connecting to {}...",
            icons::EXECUTING.cyan(),
            NETWORK_ICON,
            target.cyan()
        );
        let _ = io::stdout().flush();
    }

    /// Update connection result
    pub fn connect_result(success: bool, url: &str) {
        print!("\r{}", ansi::CLEAR_LINE);
        if success {
            println!(
                "{} {} Connected: {}",
                icons::SUCCESS.green(),
                NETWORK_ICON,
                url.green()
            );
        } else {
            println!(
                "{} {} Connection failed to {}",
                icons::ERROR.red(),
                NETWORK_ICON,
                url
            );
        }
        let _ = io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_service() {
        let svc = DiscoveredService {
            name: "prometheus-server".to_string(),
            namespace: "monitoring".to_string(),
            port: 9090,
            service_type: "ClusterIP".to_string(),
        };
        assert_eq!(svc.name, "prometheus-server");
        assert_eq!(svc.port, 9090);
    }

    #[test]
    fn test_connection_mode() {
        let display = PrometheusConnectionDisplay::new(ConnectionMode::PortForward);
        assert_eq!(display.mode, ConnectionMode::PortForward);
    }
}
