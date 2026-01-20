use crate::analyzer::{
    AnalysisConfig, BuildScript, EntryPoint, Port, PortSource, Protocol,
    context::helpers::{create_regex, extract_ports_from_command, get_script_description},
};
use crate::common::file_utils::{is_readable_file, read_file_safe};
use crate::error::{AnalysisError, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Analyzes Node.js/JavaScript/TypeScript projects
pub(crate) fn analyze_node_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    let package_json_path = root.join("package.json");

    if is_readable_file(&package_json_path) {
        let content = read_file_safe(&package_json_path, config.max_file_size)?;
        let package_json: Value = serde_json::from_str(&content)?;

        // Extract scripts
        if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
            for (name, command) in scripts {
                if let Some(cmd) = command.as_str() {
                    build_scripts.push(BuildScript {
                        name: name.clone(),
                        command: cmd.to_string(),
                        description: get_script_description(name),
                        is_default: name == "start" || name == "dev",
                    });

                    // Look for ports in scripts
                    extract_ports_from_command(cmd, ports);
                }
            }
        }

        // Find main entry point
        if let Some(main) = package_json.get("main").and_then(|m| m.as_str()) {
            entry_points.push(EntryPoint {
                file: root.join(main),
                function: None,
                command: Some(format!("node {}", main)),
            });
        }

        // Check common entry files
        let common_entries = [
            "index.js",
            "index.ts",
            "app.js",
            "app.ts",
            "server.js",
            "server.ts",
            "main.js",
            "main.ts",
        ];
        for entry in &common_entries {
            let path = root.join(entry);
            if is_readable_file(&path) {
                scan_js_file_for_context(&path, ports, env_vars, config)?;
            }
        }

        // Check src directory
        let src_dir = root.join("src");
        if src_dir.is_dir() {
            for entry in &common_entries {
                let path = src_dir.join(entry);
                if is_readable_file(&path) {
                    scan_js_file_for_context(&path, ports, env_vars, config)?;
                }
            }
        }
    }

    Ok(())
}

/// Scans JavaScript/TypeScript files for context information
fn scan_js_file_for_context(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;

    // Look for port assignments
    let port_regex =
        Regex::new(r"(?:PORT|port)\s*[=:]\s*(?:process\.env\.PORT\s*\|\|\s*)?(\d{1,5})")
            .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid regex: {}", e)))?;
    for cap in port_regex.captures_iter(&content) {
        if let Some(port_str) = cap.get(1)
            && let Ok(port) = port_str.as_str().parse::<u16>()
        {
            ports.insert(Port {
                number: port,
                protocol: Protocol::Http,
                description: Some("HTTP server port".to_string()),
                source: Some(PortSource::SourceCode),
            });
        }
    }

    // Look for app.listen() calls
    let listen_regex = Regex::new(r"\.listen\s*\(\s*(\d{1,5})")
        .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid regex: {}", e)))?;
    for cap in listen_regex.captures_iter(&content) {
        if let Some(port_str) = cap.get(1)
            && let Ok(port) = port_str.as_str().parse::<u16>()
        {
            ports.insert(Port {
                number: port,
                protocol: Protocol::Http,
                description: Some("Express/HTTP server".to_string()),
                source: Some(PortSource::SourceCode),
            });
        }
    }

    // Look for environment variable usage
    let env_regex = Regex::new(r"process\.env\.([A-Z_][A-Z0-9_]*)")
        .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid regex: {}", e)))?;
    for cap in env_regex.captures_iter(&content) {
        if let Some(var_name) = cap.get(1) {
            let name = var_name.as_str().to_string();
            if !name.starts_with("NODE_") {
                // Skip Node.js internal vars
                env_vars.entry(name.clone()).or_insert((None, false, None));
            }
        }
    }

    // Look for Encore.dev imports and patterns
    if content.contains("encore.dev") {
        // Encore uses specific patterns for config and database
        let encore_patterns = [
            (
                r#"secret\s*\(\s*['"]([A-Z_][A-Z0-9_]*)['"]"#,
                "Encore secret configuration",
            ),
            (r#"SQLDatabase\s*\(\s*['"](\w+)['"]"#, "Encore database"),
        ];

        for (pattern, description) in &encore_patterns {
            let regex = create_regex(pattern)?;
            for cap in regex.captures_iter(&content) {
                if let Some(match_str) = cap.get(1) {
                    let name = match_str.as_str();
                    if pattern.contains("secret") {
                        env_vars.entry(name.to_string()).or_insert((
                            None,
                            true,
                            Some(description.to_string()),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}
