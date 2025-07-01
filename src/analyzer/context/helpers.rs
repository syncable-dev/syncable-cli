use crate::analyzer::{Port, Protocol};
use crate::error::{AnalysisError, Result};
use regex::Regex;
use std::collections::HashSet;

/// Helper function to create a regex with proper error handling
pub fn create_regex(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).map_err(|e| {
        AnalysisError::InvalidStructure(format!("Invalid regex pattern '{}': {}", pattern, e)).into()
    })
}

/// Extracts ports from command strings
pub fn extract_ports_from_command(command: &str, ports: &mut HashSet<Port>) {
    // Look for common port patterns in commands
    let patterns = [
        r"-p\s+(\d{1,5})",
        r"--port\s+(\d{1,5})",
        r"--port=(\d{1,5})",
        r"PORT=(\d{1,5})",
    ];

    for pattern in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            for cap in regex.captures_iter(command) {
                if let Some(port_str) = cap.get(1) {
                    if let Ok(port) = port_str.as_str().parse::<u16>() {
                        ports.insert(Port {
                            number: port,
                            protocol: Protocol::Http,
                            description: Some("Port from command".to_string()),
                        });
                    }
                }
            }
        }
    }
}

/// Helper function to get script description
pub fn get_script_description(name: &str) -> Option<String> {
    match name {
        "start" => Some("Start the application".to_string()),
        "dev" => Some("Start development server".to_string()),
        "build" => Some("Build the application".to_string()),
        "test" => Some("Run tests".to_string()),
        "lint" => Some("Run linter".to_string()),
        "format" => Some("Format code".to_string()),
        _ => None,
    }
} 