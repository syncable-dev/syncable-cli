use crate::analyzer::{
    AnalysisConfig, BuildScript, Port, Protocol, context::helpers::create_regex,
};
use crate::common::file_utils::{is_readable_file, read_file_safe};
use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Analyzes JVM projects (Java/Kotlin)
pub(crate) fn analyze_jvm_project(
    root: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    build_scripts: &mut Vec<BuildScript>,
    config: &AnalysisConfig,
) -> Result<()> {
    // Check for Maven
    let pom_xml = root.join("pom.xml");
    if is_readable_file(&pom_xml) {
        build_scripts.extend(vec![
            BuildScript {
                name: "build".to_string(),
                command: "mvn clean package".to_string(),
                description: Some("Build with Maven".to_string()),
                is_default: false,
            },
            BuildScript {
                name: "test".to_string(),
                command: "mvn test".to_string(),
                description: Some("Run tests".to_string()),
                is_default: false,
            },
            BuildScript {
                name: "run".to_string(),
                command: "mvn spring-boot:run".to_string(),
                description: Some("Run Spring Boot application".to_string()),
                is_default: true,
            },
        ]);
    }

    // Check for Gradle
    let gradle_files = ["build.gradle", "build.gradle.kts"];
    for gradle_file in &gradle_files {
        if is_readable_file(&root.join(gradle_file)) {
            build_scripts.extend(vec![
                BuildScript {
                    name: "build".to_string(),
                    command: "./gradlew build".to_string(),
                    description: Some("Build with Gradle".to_string()),
                    is_default: false,
                },
                BuildScript {
                    name: "test".to_string(),
                    command: "./gradlew test".to_string(),
                    description: Some("Run tests".to_string()),
                    is_default: false,
                },
                BuildScript {
                    name: "run".to_string(),
                    command: "./gradlew bootRun".to_string(),
                    description: Some("Run Spring Boot application".to_string()),
                    is_default: true,
                },
            ]);
            break;
        }
    }

    // Look for application properties
    let app_props_locations = [
        "src/main/resources/application.properties",
        "src/main/resources/application.yml",
        "src/main/resources/application.yaml",
    ];

    for props_path in &app_props_locations {
        let full_path = root.join(props_path);
        if is_readable_file(&full_path) {
            analyze_application_properties(&full_path, ports, env_vars, config)?;
        }
    }

    Ok(())
}

/// Analyzes application properties files
fn analyze_application_properties(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
    config: &AnalysisConfig,
) -> Result<()> {
    let content = read_file_safe(path, config.max_file_size)?;

    // Look for server.port
    let port_regex = create_regex(r"server\.port\s*[=:]\s*(\d{1,5})")?;
    for cap in port_regex.captures_iter(&content) {
        if let Some(port_str) = cap.get(1) {
            if let Ok(port) = port_str.as_str().parse::<u16>() {
                ports.insert(Port {
                    number: port,
                    protocol: Protocol::Http,
                    description: Some("Spring Boot server".to_string()),
                });
            }
        }
    }

    // Look for ${ENV_VAR} placeholders
    let env_regex = create_regex(r"\$\{([A-Z_][A-Z0-9_]*)\}")?;
    for cap in env_regex.captures_iter(&content) {
        if let Some(var_name) = cap.get(1) {
            let name = var_name.as_str().to_string();
            env_vars.entry(name.clone()).or_insert((None, false, None));
        }
    }

    Ok(())
}
