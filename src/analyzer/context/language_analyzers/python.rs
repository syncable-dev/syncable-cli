use crate::analyzer::{
    AnalysisConfig, BuildScript, EntryPoint, Port, PortSource, Protocol,
    context::helpers::create_regex,
};
use crate::common::file_utils::{is_readable_file, read_file_safe};
use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Analyzes Python projects
pub(crate) fn analyze_python_project(
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    // Check for common Python entry points
    let common_entries = [
        "main.py",
        "app.py",
        "wsgi.py",
        "asgi.py",
        "manage.py",
        "run.py",
        "__main__.py",
    ];

    for entry in &common_entries {
        let path = root.join(entry);
        if is_readable_file(&path) {
            scan_python_file_for_context(&path, entry_points, ports, env_vars, config)?;
        }
    }

    // Check setup.py for entry points
    let setup_py = root.join("setup.py");
    if is_readable_file(&setup_py) {
        let content = read_file_safe(&setup_py, config.max_file_size)?;

        // Look for console_scripts
        let console_regex = create_regex(r#"console_scripts['"]\s*:\s*\[(.*?)\]"#)?;
        if let Some(cap) = console_regex.captures(&content)
            && let Some(scripts) = cap.get(1)
        {
            let script_regex = create_regex(r#"['"](\w+)\s*=\s*([\w\.]+):(\w+)"#)?;
            for script_cap in script_regex.captures_iter(scripts.as_str()) {
                if let (Some(name), Some(module), Some(func)) =
                    (script_cap.get(1), script_cap.get(2), script_cap.get(3))
                {
                    entry_points.push(EntryPoint {
                        file: PathBuf::from(format!("{}.py", module.as_str().replace('.', "/"))),
                        function: Some(func.as_str().to_string()),
                        command: Some(name.as_str().to_string()),
                    });
                }
            }
        }
    }

    // Check pyproject.toml for scripts
    let pyproject = root.join("pyproject.toml");
    if is_readable_file(&pyproject) {
        let content = read_file_safe(&pyproject, config.max_file_size)?;
        if let Ok(toml_value) = toml::from_str::<toml::Value>(&content) {
            // Extract build scripts from poetry
            if let Some(scripts) = toml_value
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("scripts"))
                .and_then(|s| s.as_table())
            {
                for (name, cmd) in scripts {
                    if let Some(command) = cmd.as_str() {
                        build_scripts.push(BuildScript {
                            name: name.clone(),
                            command: command.to_string(),
                            description: None,
                            is_default: name == "start" || name == "run",
                        });
                    }
                }
            }
        }
    }

    // Common Python build commands
    build_scripts.push(BuildScript {
        name: "install".to_string(),
        command: "pip install -r requirements.txt".to_string(),
        description: Some("Install dependencies".to_string()),
        is_default: false,
    });

    Ok(())
}

/// Scans Python files for context information
fn scan_python_file_for_context(
    path: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;

    // Look for Flask/FastAPI/Django port configurations
    let port_patterns = [
        r"port\s*=\s*(\d{1,5})",
        r"PORT\s*=\s*(\d{1,5})",
        r"\.run\s*\([^)]*port\s*=\s*(\d{1,5})",
        r"uvicorn\.run\s*\([^)]*port\s*=\s*(\d{1,5})",
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
                    description: Some("Python web server".to_string()),
                    source: Some(PortSource::SourceCode),
                });
            }
        }
    }

    // Look for environment variable usage
    let env_patterns = [
        r#"os\.environ\.get\s*\(\s*["']([A-Z_][A-Z0-9_]*)["']"#,
        r#"os\.environ\s*\[\s*["']([A-Z_][A-Z0-9_]*)["']\s*\]"#,
        r#"os\.getenv\s*\(\s*["']([A-Z_][A-Z0-9_]*)["']"#,
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

    // Check if this is a main entry point
    if content.contains("if __name__ == '__main__':")
        || content.contains("if __name__ == \"__main__\":")
    {
        entry_points.push(EntryPoint {
            file: path.to_path_buf(),
            function: Some("main".to_string()),
            command: Some(format!(
                "python {}",
                path.file_name().unwrap().to_string_lossy()
            )),
        });
    }

    Ok(())
}
