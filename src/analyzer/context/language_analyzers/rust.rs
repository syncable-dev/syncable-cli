use crate::analyzer::{context::helpers::create_regex, AnalysisConfig, BuildScript, EntryPoint, Port, Protocol};
use crate::common::file_utils::{is_readable_file, read_file_safe};
use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Analyzes Rust projects
pub(crate) fn analyze_rust_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    let cargo_toml = root.join("Cargo.toml");

    if is_readable_file(&cargo_toml) {
        let content = read_file_safe(&cargo_toml, config.max_file_size)?;
        if let Ok(toml_value) = toml::from_str::<toml::Value>(&content) {
            // Check for binary targets
            if let Some(bins) = toml_value.get("bin").and_then(|b| b.as_array()) {
                for bin in bins {
                    if let Some(name) = bin.get("name").and_then(|n| n.as_str()) {
                        let path = bin.get("path")
                            .and_then(|p| p.as_str())
                            .map(PathBuf::from)
                            .unwrap_or_else(|| root.join("src").join("bin").join(format!("{}.rs", name)));

                        entry_points.push(EntryPoint {
                            file: path,
                            function: Some("main".to_string()),
                            command: Some(format!("cargo run --bin {}", name)),
                        });
                    }
                }
            }

            // Default binary
            if let Some(_package_name) = toml_value.get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str()) {
                let main_rs = root.join("src").join("main.rs");
                if is_readable_file(&main_rs) {
                    entry_points.push(EntryPoint {
                        file: main_rs.clone(),
                        function: Some("main".to_string()),
                        command: Some("cargo run".to_string()),
                    });

                    // Scan main.rs for context
                    scan_rust_file_for_context(&main_rs, ports, env_vars, config)?;
                }
            }
        }
    }

    // Common Rust build commands
    build_scripts.extend(vec![
        BuildScript {
            name: "build".to_string(),
            command: "cargo build".to_string(),
            description: Some("Build the project".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "build-release".to_string(),
            command: "cargo build --release".to_string(),
            description: Some("Build optimized release version".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "test".to_string(),
            command: "cargo test".to_string(),
            description: Some("Run tests".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "run".to_string(),
            command: "cargo run".to_string(),
            description: Some("Run the application".to_string()),
            is_default: true,
        },
    ]);

    Ok(())
}

/// Scans Rust files for context information
fn scan_rust_file_for_context(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;

    // Look for port bindings
    let port_patterns = [
        r#"bind\s*\(\s*"[^"]*:(\d{1,5})"\s*\)"#,
        r#"bind\s*\(\s*\([^,]+,\s*(\d{1,5})\)\s*\)"#,
        r#"listen\s*\(\s*"[^"]*:(\d{1,5})"\s*\)"#,
        r"PORT[^=]*=\s*(\d{1,5})",
    ];

    for pattern in &port_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(port_str) = cap.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    ports.insert(Port {
                        number: port,
                        protocol: Protocol::Http,
                        description: Some("Rust web server".to_string()),
                    });
                }
            }
        }
    }

    // Look for environment variable usage
    let env_patterns = [
        r#"env::var\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
        r#"std::env::var\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
        r#"env!\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
    ];

    for pattern in &env_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(var_name) = cap.get(1) {
                let name = var_name.as_str().to_string();
                env_vars.entry(name.clone()).or_insert((None, false, None));
            }
        }
    }

    Ok(())
} 