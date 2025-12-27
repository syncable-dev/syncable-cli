use crate::analyzer::{
    AnalysisConfig, BuildScript, EntryPoint, Port, Protocol, context::helpers::create_regex,
};
use crate::common::file_utils::{is_readable_file, read_file_safe};
use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Analyzes Go projects
pub(crate) fn analyze_go_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    // Check for main.go
    let main_go = root.join("main.go");
    if is_readable_file(&main_go) {
        entry_points.push(EntryPoint {
            file: main_go.clone(),
            function: Some("main".to_string()),
            command: Some("go run main.go".to_string()),
        });

        scan_go_file_for_context(&main_go, ports, env_vars, config)?;
    }

    // Check cmd directory for multiple binaries
    let cmd_dir = root.join("cmd");
    if cmd_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&cmd_dir)
    {
        for entry in entries.flatten() {
            if entry.file_type()?.is_dir() {
                let main_file = entry.path().join("main.go");
                if is_readable_file(&main_file) {
                    let cmd_name = entry.file_name().to_string_lossy().to_string();
                    entry_points.push(EntryPoint {
                        file: main_file.clone(),
                        function: Some("main".to_string()),
                        command: Some(format!("go run ./cmd/{}", cmd_name)),
                    });

                    scan_go_file_for_context(&main_file, ports, env_vars, config)?;
                }
            }
        }
    }

    // Common Go build commands
    build_scripts.extend(vec![
        BuildScript {
            name: "build".to_string(),
            command: "go build".to_string(),
            description: Some("Build the project".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "test".to_string(),
            command: "go test ./...".to_string(),
            description: Some("Run tests".to_string()),
            is_default: false,
        },
        BuildScript {
            name: "run".to_string(),
            command: "go run .".to_string(),
            description: Some("Run the application".to_string()),
            is_default: true,
        },
        BuildScript {
            name: "mod-download".to_string(),
            command: "go mod download".to_string(),
            description: Some("Download dependencies".to_string()),
            is_default: false,
        },
    ]);

    Ok(())
}

/// Scans Go files for context information
fn scan_go_file_for_context(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;

    // Look for port bindings
    let port_patterns = [
        r#"Listen\s*\(\s*":(\d{1,5})"\s*\)"#,
        r#"ListenAndServe\s*\(\s*":(\d{1,5})"\s*,"#,
        r#"Addr:\s*":(\d{1,5})""#,
        r"PORT[^=]*=\s*(\d{1,5})",
    ];

    for pattern in &port_patterns {
        let regex = create_regex(pattern)?;
        for cap in regex.captures_iter(&content) {
            if let Some(port_str) = cap.get(1)
                && let Ok(port) = port_str.as_str().parse::<u16>()
            {
                ports.insert(Port {
                    number: port,
                    protocol: Protocol::Http,
                    description: Some("Go web server".to_string()),
                });
            }
        }
    }

    // Look for environment variable usage
    let env_patterns = [
        r#"os\.Getenv\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
        r#"os\.LookupEnv\s*\(\s*"([A-Z_][A-Z0-9_]*)"\s*\)"#,
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
