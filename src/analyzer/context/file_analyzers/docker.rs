use crate::analyzer::{Port, Protocol, context::helpers::create_regex};
use crate::common::file_utils::is_readable_file;
use crate::error::{AnalysisError, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Analyzes Docker files for ports and environment variables
pub(crate) fn analyze_docker_files(
    root: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
) -> Result<()> {
    let dockerfile = root.join("Dockerfile");

    if is_readable_file(&dockerfile) {
        let content = std::fs::read_to_string(&dockerfile)?;

        // Look for EXPOSE directives
        let expose_regex = create_regex(r"EXPOSE\s+(\d{1,5})(?:/(\w+))?")?;
        for cap in expose_regex.captures_iter(&content) {
            if let Some(port_str) = cap.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    let protocol = cap
                        .get(2)
                        .and_then(|p| match p.as_str().to_lowercase().as_str() {
                            "tcp" => Some(Protocol::Tcp),
                            "udp" => Some(Protocol::Udp),
                            _ => None,
                        })
                        .unwrap_or(Protocol::Tcp);

                    ports.insert(Port {
                        number: port,
                        protocol,
                        description: Some("Exposed in Dockerfile".to_string()),
                    });
                }
            }
        }

        // Look for ENV directives
        let env_regex = create_regex(r"ENV\s+([A-Z_][A-Z0-9_]*)\s+(.+)")?;
        for cap in env_regex.captures_iter(&content) {
            if let (Some(name), Some(value)) = (cap.get(1), cap.get(2)) {
                let var_name = name.as_str().to_string();
                let var_value = value.as_str().trim().to_string();
                env_vars
                    .entry(var_name)
                    .or_insert((Some(var_value), false, None));
            }
        }
    }

    // Check docker-compose files
    let compose_files = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    for compose_file in &compose_files {
        let path = root.join(compose_file);
        if is_readable_file(&path) {
            analyze_docker_compose(&path, ports, env_vars)?;
            break;
        }
    }

    Ok(())
}

/// Analyzes docker-compose files
fn analyze_docker_compose(
    path: &Path,
    ports: &mut HashSet<Port>,
    env_vars: &mut HashMap<String, (Option<String>, bool, Option<String>)>,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| AnalysisError::InvalidStructure(format!("Invalid YAML: {}", e)))?;

    if let Some(services) = value.get("services").and_then(|s| s.as_mapping()) {
        for (service_name, service) in services {
            let service_name_str = service_name.as_str().unwrap_or("unknown");

            // Determine service type based on image, name, and other indicators
            let service_type = determine_service_type(service_name_str, service);

            // Extract ports
            if let Some(service_ports) = service.get("ports").and_then(|p| p.as_sequence()) {
                for port_entry in service_ports {
                    if let Some(port_str) = port_entry.as_str() {
                        // Parse port mappings like "8080:80" or just "80"
                        let parts: Vec<&str> = port_str.split(':').collect();

                        let (external_port, internal_port, protocol_suffix) = if parts.len() >= 2 {
                            // Format: "external:internal" or "external:internal/protocol"
                            let external = parts[0].trim();
                            let internal_parts: Vec<&str> = parts[1].split('/').collect();
                            let internal = internal_parts[0].trim();
                            let protocol = internal_parts.get(1).map(|p| p.trim());
                            (external, internal, protocol)
                        } else {
                            // Format: just "port" or "port/protocol"
                            let port_parts: Vec<&str> = parts[0].split('/').collect();
                            let port = port_parts[0].trim();
                            let protocol = port_parts.get(1).map(|p| p.trim());
                            (port, port, protocol)
                        };

                        // Determine protocol
                        let protocol = match protocol_suffix {
                            Some("udp") => Protocol::Udp,
                            _ => Protocol::Tcp,
                        };

                        // Create descriptive port entry
                        if let Ok(port) = external_port.parse::<u16>() {
                            let description = create_port_description(
                                &service_type,
                                service_name_str,
                                external_port,
                                internal_port,
                            );

                            ports.insert(Port {
                                number: port,
                                protocol,
                                description: Some(description),
                            });
                        }
                    }
                }
            }

            // Extract environment variables with context
            if let Some(env) = service.get("environment") {
                let env_context = format!(" ({})", service_type.as_str());

                if let Some(env_map) = env.as_mapping() {
                    for (key, value) in env_map {
                        if let Some(key_str) = key.as_str() {
                            let val_str = value.as_str().map(|s| s.to_string());
                            let description = get_env_var_description(key_str, &service_type);
                            env_vars.entry(key_str.to_string()).or_insert((
                                val_str,
                                false,
                                description.or_else(|| Some(env_context.clone())),
                            ));
                        }
                    }
                } else if let Some(env_list) = env.as_sequence() {
                    for item in env_list {
                        if let Some(env_str) = item.as_str() {
                            if let Some(eq_pos) = env_str.find('=') {
                                let (key, value) = env_str.split_at(eq_pos);
                                let value = &value[1..]; // Skip the '='
                                let description = get_env_var_description(key, &service_type);
                                env_vars.entry(key.to_string()).or_insert((
                                    Some(value.to_string()),
                                    false,
                                    description.or_else(|| Some(env_context.clone())),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Service types found in Docker Compose
#[derive(Debug, Clone)]
enum ServiceType {
    PostgreSQL,
    MySQL,
    MongoDB,
    Redis,
    RabbitMQ,
    Kafka,
    Elasticsearch,
    Application,
    Nginx,
    Unknown,
}

impl ServiceType {
    fn as_str(&self) -> &'static str {
        match self {
            ServiceType::PostgreSQL => "PostgreSQL database",
            ServiceType::MySQL => "MySQL database",
            ServiceType::MongoDB => "MongoDB database",
            ServiceType::Redis => "Redis cache",
            ServiceType::RabbitMQ => "RabbitMQ message broker",
            ServiceType::Kafka => "Kafka message broker",
            ServiceType::Elasticsearch => "Elasticsearch search engine",
            ServiceType::Application => "Application service",
            ServiceType::Nginx => "Nginx web server",
            ServiceType::Unknown => "Service",
        }
    }
}

/// Determines the type of service based on various indicators
fn determine_service_type(name: &str, service: &serde_yaml::Value) -> ServiceType {
    let name_lower = name.to_lowercase();

    // Check service name
    if name_lower.contains("postgres") || name_lower.contains("pg") || name_lower.contains("psql") {
        return ServiceType::PostgreSQL;
    } else if name_lower.contains("mysql") || name_lower.contains("mariadb") {
        return ServiceType::MySQL;
    } else if name_lower.contains("mongo") {
        return ServiceType::MongoDB;
    } else if name_lower.contains("redis") {
        return ServiceType::Redis;
    } else if name_lower.contains("rabbit") || name_lower.contains("amqp") {
        return ServiceType::RabbitMQ;
    } else if name_lower.contains("kafka") {
        return ServiceType::Kafka;
    } else if name_lower.contains("elastic") || name_lower.contains("es") {
        return ServiceType::Elasticsearch;
    } else if name_lower.contains("nginx") || name_lower.contains("proxy") {
        return ServiceType::Nginx;
    }

    // Check image name
    if let Some(image) = service.get("image").and_then(|i| i.as_str()) {
        let image_lower = image.to_lowercase();
        if image_lower.contains("postgres") {
            return ServiceType::PostgreSQL;
        } else if image_lower.contains("mysql") || image_lower.contains("mariadb") {
            return ServiceType::MySQL;
        } else if image_lower.contains("mongo") {
            return ServiceType::MongoDB;
        } else if image_lower.contains("redis") {
            return ServiceType::Redis;
        } else if image_lower.contains("rabbitmq") {
            return ServiceType::RabbitMQ;
        } else if image_lower.contains("kafka") {
            return ServiceType::Kafka;
        } else if image_lower.contains("elastic") {
            return ServiceType::Elasticsearch;
        } else if image_lower.contains("nginx") {
            return ServiceType::Nginx;
        }
    }

    // Check environment variables for clues
    if let Some(env) = service.get("environment") {
        if let Some(env_map) = env.as_mapping() {
            for (key, _) in env_map {
                if let Some(key_str) = key.as_str() {
                    if key_str.contains("POSTGRES") || key_str.contains("PGPASSWORD") {
                        return ServiceType::PostgreSQL;
                    } else if key_str.contains("MYSQL") {
                        return ServiceType::MySQL;
                    } else if key_str.contains("MONGO") {
                        return ServiceType::MongoDB;
                    }
                }
            }
        }
    }

    // Check if it has a build context (likely application)
    if service.get("build").is_some() {
        return ServiceType::Application;
    }

    ServiceType::Unknown
}

/// Creates a descriptive port description based on service type
fn create_port_description(
    service_type: &ServiceType,
    service_name: &str,
    external: &str,
    internal: &str,
) -> String {
    let base_desc = match service_type {
        ServiceType::PostgreSQL => format!("PostgreSQL database ({})", service_name),
        ServiceType::MySQL => format!("MySQL database ({})", service_name),
        ServiceType::MongoDB => format!("MongoDB database ({})", service_name),
        ServiceType::Redis => format!("Redis cache ({})", service_name),
        ServiceType::RabbitMQ => format!("RabbitMQ message broker ({})", service_name),
        ServiceType::Kafka => format!("Kafka message broker ({})", service_name),
        ServiceType::Elasticsearch => format!("Elasticsearch ({})", service_name),
        ServiceType::Nginx => format!("Nginx proxy ({})", service_name),
        ServiceType::Application => format!("Application service ({})", service_name),
        ServiceType::Unknown => format!("Docker service ({})", service_name),
    };

    if external != internal {
        format!(
            "{} - external:{}, internal:{}",
            base_desc, external, internal
        )
    } else {
        format!("{} - port {}", base_desc, external)
    }
}

/// Gets a descriptive context for environment variables based on service type
fn get_env_var_description(var_name: &str, _service_type: &ServiceType) -> Option<String> {
    match var_name {
        "POSTGRES_PASSWORD" | "POSTGRES_USER" | "POSTGRES_DB" => {
            Some("PostgreSQL configuration".to_string())
        }
        "MYSQL_ROOT_PASSWORD" | "MYSQL_PASSWORD" | "MYSQL_USER" | "MYSQL_DATABASE" => {
            Some("MySQL configuration".to_string())
        }
        "MONGO_INITDB_ROOT_USERNAME" | "MONGO_INITDB_ROOT_PASSWORD" => {
            Some("MongoDB configuration".to_string())
        }
        "REDIS_PASSWORD" => Some("Redis configuration".to_string()),
        "RABBITMQ_DEFAULT_USER" | "RABBITMQ_DEFAULT_PASS" => {
            Some("RabbitMQ configuration".to_string())
        }
        "DATABASE_URL" | "DB_CONNECTION_STRING" => Some("Database connection string".to_string()),
        "GOOGLE_APPLICATION_CREDENTIALS" => {
            Some("Google Cloud service account credentials".to_string())
        }
        _ => None,
    }
}
