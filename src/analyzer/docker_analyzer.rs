//! # Docker Analyzer Module
//! 
//! This module provides Docker infrastructure analysis capabilities for detecting:
//! - Dockerfiles and their variants (dockerfile.dev, dockerfile.prod, etc.)
//! - Docker Compose files and their variants (docker-compose.dev.yaml, etc.)
//! - Port mappings and networking configuration
//! - Service discovery and inter-service communication
//! - Container orchestration patterns

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;

/// Represents a Docker infrastructure analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockerAnalysis {
    /// All Dockerfiles found in the project
    pub dockerfiles: Vec<DockerfileInfo>,
    /// All Docker Compose files found in the project
    pub compose_files: Vec<ComposeFileInfo>,
    /// Analyzed services from compose files
    pub services: Vec<DockerService>,
    /// Network configuration and service discovery
    pub networking: NetworkingConfig,
    /// Overall container orchestration pattern
    pub orchestration_pattern: OrchestrationPattern,
    /// Environment-specific configurations
    pub environments: Vec<DockerEnvironment>,
}

/// Information about a Dockerfile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockerfileInfo {
    /// Path to the Dockerfile
    pub path: PathBuf,
    /// Environment this Dockerfile is for (dev, prod, staging, etc.)
    pub environment: Option<String>,
    /// Base image used
    pub base_image: Option<String>,
    /// Exposed ports from EXPOSE instructions
    pub exposed_ports: Vec<u16>,
    /// Working directory
    pub workdir: Option<String>,
    /// Entry point or CMD
    pub entrypoint: Option<String>,
    /// Environment variables defined
    pub env_vars: Vec<String>,
    /// Multi-stage build stages
    pub build_stages: Vec<String>,
    /// Whether it's a multi-stage build
    pub is_multistage: bool,
    /// Dockerfile instructions count (complexity indicator)
    pub instruction_count: usize,
}

/// Information about a Docker Compose file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComposeFileInfo {
    /// Path to the compose file
    pub path: PathBuf,
    /// Environment this compose file is for
    pub environment: Option<String>,
    /// Compose file version
    pub version: Option<String>,
    /// Services defined in the compose file
    pub service_names: Vec<String>,
    /// Networks defined
    pub networks: Vec<String>,
    /// Volumes defined
    pub volumes: Vec<String>,
    /// External dependencies (external networks, volumes)
    pub external_dependencies: Vec<String>,
}

/// Container orchestration patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OrchestrationPattern {
    /// Single container application
    SingleContainer,
    /// Multiple containers with docker-compose
    DockerCompose,
    /// Microservices architecture
    Microservices,
    /// Event-driven architecture
    EventDriven,
    /// Service mesh
    ServiceMesh,
    /// Mixed or complex pattern
    Mixed,
}

/// Represents a Docker service from compose files
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockerService {
    /// Service name
    pub name: String,
    /// Which compose file this service is defined in
    pub compose_file: PathBuf,
    /// Docker image or build context
    pub image_or_build: ImageOrBuild,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Service dependencies
    pub depends_on: Vec<String>,
    /// Networks this service is connected to
    pub networks: Vec<String>,
    /// Volumes mounted
    pub volumes: Vec<VolumeMount>,
    /// Health check configuration
    pub health_check: Option<HealthCheck>,
    /// Restart policy
    pub restart_policy: Option<String>,
    /// Resource limits
    pub resource_limits: Option<ResourceLimits>,
}

/// Image or build configuration for a service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImageOrBuild {
    /// Uses a pre-built image
    Image(String),
    /// Builds from a Dockerfile
    Build {
        context: String,
        dockerfile: Option<String>,
        args: HashMap<String, String>,
    },
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortMapping {
    /// Host port (external)
    pub host_port: Option<u16>,
    /// Container port (internal)
    pub container_port: u16,
    /// Protocol (tcp, udp)
    pub protocol: String,
    /// Whether this port is exposed to the host
    pub exposed_to_host: bool,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeMount {
    /// Source (host path or volume name)
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Mount type (bind, volume, tmpfs)
    pub mount_type: String,
    /// Whether it's read-only
    pub read_only: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthCheck {
    /// Test command
    pub test: String,
    /// Interval between checks
    pub interval: Option<String>,
    /// Timeout for each check
    pub timeout: Option<String>,
    /// Number of retries
    pub retries: Option<u32>,
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLimits {
    /// CPU limit
    pub cpu_limit: Option<String>,
    /// Memory limit
    pub memory_limit: Option<String>,
    /// CPU reservation
    pub cpu_reservation: Option<String>,
    /// Memory reservation
    pub memory_reservation: Option<String>,
}

/// Networking configuration analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkingConfig {
    /// Custom networks defined
    pub custom_networks: Vec<NetworkInfo>,
    /// Service discovery patterns
    pub service_discovery: ServiceDiscoveryConfig,
    /// Load balancing configuration
    pub load_balancing: Vec<LoadBalancerConfig>,
    /// External connectivity patterns
    pub external_connectivity: ExternalConnectivity,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkInfo {
    /// Network name
    pub name: String,
    /// Network driver
    pub driver: Option<String>,
    /// Whether it's external
    pub external: bool,
    /// Connected services
    pub connected_services: Vec<String>,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceDiscoveryConfig {
    /// Whether services can discover each other by name
    pub internal_dns: bool,
    /// External service discovery tools
    pub external_tools: Vec<String>,
    /// Service mesh indicators
    pub service_mesh: bool,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadBalancerConfig {
    /// Service name
    pub service: String,
    /// Load balancer type (nginx, traefik, etc.)
    pub lb_type: String,
    /// Backend services
    pub backends: Vec<String>,
}

/// External connectivity patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalConnectivity {
    /// Services exposed to external traffic
    pub exposed_services: Vec<ExposedService>,
    /// Ingress patterns
    pub ingress_patterns: Vec<String>,
    /// API gateways
    pub api_gateways: Vec<String>,
}

/// Service exposed to external traffic
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExposedService {
    /// Service name
    pub service: String,
    /// External ports
    pub external_ports: Vec<u16>,
    /// Protocols
    pub protocols: Vec<String>,
    /// Whether it has SSL/TLS
    pub ssl_enabled: bool,
}

/// Environment-specific Docker configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockerEnvironment {
    /// Environment name (dev, prod, staging, etc.)
    pub name: String,
    /// Dockerfiles for this environment
    pub dockerfiles: Vec<PathBuf>,
    /// Compose files for this environment
    pub compose_files: Vec<PathBuf>,
    /// Environment-specific configurations
    pub config_overrides: HashMap<String, String>,
}

/// Analyzes Docker infrastructure in a project
pub fn analyze_docker_infrastructure(project_root: &Path) -> Result<DockerAnalysis> {
    log::info!("Starting Docker infrastructure analysis for: {}", project_root.display());
    
    // Find all Docker-related files
    let dockerfiles = find_dockerfiles(project_root)?;
    let compose_files = find_compose_files(project_root)?;
    
    log::debug!("Found {} Dockerfiles and {} Compose files", dockerfiles.len(), compose_files.len());
    
    // Parse Dockerfiles
    let parsed_dockerfiles: Vec<DockerfileInfo> = dockerfiles.into_iter()
        .filter_map(|path| parse_dockerfile(&path).ok())
        .collect();
    
    // Parse Compose files
    let parsed_compose_files: Vec<ComposeFileInfo> = compose_files.into_iter()
        .filter_map(|path| parse_compose_file(&path).ok())
        .collect();
    
    // Extract services from compose files
    let services = extract_services_from_compose(&parsed_compose_files)?;
    
    // Analyze networking
    let networking = analyze_networking(&services, &parsed_compose_files)?;
    
    // Determine orchestration pattern
    let orchestration_pattern = determine_orchestration_pattern(&services, &networking);
    
    // Analyze environments
    let environments = analyze_environments(&parsed_dockerfiles, &parsed_compose_files);
    
    Ok(DockerAnalysis {
        dockerfiles: parsed_dockerfiles,
        compose_files: parsed_compose_files,
        services,
        networking,
        orchestration_pattern,
        environments,
    })
}

/// Finds all Dockerfiles in the project, including variants
fn find_dockerfiles(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut dockerfiles = Vec::new();
    
    fn collect_dockerfiles_recursive(dir: &Path, dockerfiles: &mut Vec<PathBuf>) -> Result<()> {
        if dir.file_name().map_or(false, |name| {
            name == "node_modules" || name == ".git" || name == "target" || name == ".next"
        }) {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                collect_dockerfiles_recursive(&path, dockerfiles)?;
            } else if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if is_dockerfile_name(filename) {
                    dockerfiles.push(path);
                }
            }
        }
        Ok(())
    }
    
    collect_dockerfiles_recursive(project_root, &mut dockerfiles)?;
    
    Ok(dockerfiles)
}

/// Checks if a filename matches Dockerfile patterns
fn is_dockerfile_name(filename: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    
    // Exact matches
    if filename_lower == "dockerfile" {
        return true;
    }
    
    // Pattern matches
    if filename_lower.starts_with("dockerfile.") {
        return true;
    }
    
    if filename_lower.ends_with(".dockerfile") {
        return true;
    }
    
    false
}

/// Finds all Docker Compose files in the project
fn find_compose_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut compose_files = Vec::new();
    
    fn collect_compose_files_recursive(dir: &Path, compose_files: &mut Vec<PathBuf>) -> Result<()> {
        if dir.file_name().map_or(false, |name| {
            name == "node_modules" || name == ".git" || name == "target" || name == ".next"
        }) {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                collect_compose_files_recursive(&path, compose_files)?;
            } else if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if is_compose_file_name(filename) {
                    compose_files.push(path);
                }
            }
        }
        Ok(())
    }
    
    collect_compose_files_recursive(project_root, &mut compose_files)?;
    
    Ok(compose_files)
}

/// Checks if a filename matches Docker Compose patterns
fn is_compose_file_name(filename: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    
    // Common compose file patterns
    let patterns = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    
    // Exact matches
    for pattern in &patterns {
        if filename_lower == *pattern {
            return true;
        }
    }
    
    // Environment-specific patterns
    if filename_lower.starts_with("docker-compose.") && 
       (filename_lower.ends_with(".yml") || filename_lower.ends_with(".yaml")) {
        return true;
    }
    
    if filename_lower.starts_with("compose.") && 
       (filename_lower.ends_with(".yml") || filename_lower.ends_with(".yaml")) {
        return true;
    }
    
    false
}

/// Parses a Dockerfile and extracts information
fn parse_dockerfile(path: &PathBuf) -> Result<DockerfileInfo> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    let mut info = DockerfileInfo {
        path: path.clone(),
        environment: extract_environment_from_filename(path),
        base_image: None,
        exposed_ports: Vec::new(),
        workdir: None,
        entrypoint: None,
        env_vars: Vec::new(),
        build_stages: Vec::new(),
        is_multistage: false,
        instruction_count: 0,
    };
    
    // Regex patterns for Dockerfile instructions
    let from_regex = Regex::new(r"(?i)^FROM\s+(.+?)(?:\s+AS\s+(.+))?$").unwrap();
    let expose_regex = Regex::new(r"(?i)^EXPOSE\s+(.+)$").unwrap();
    let workdir_regex = Regex::new(r"(?i)^WORKDIR\s+(.+)$").unwrap();
    let cmd_regex = Regex::new(r"(?i)^CMD\s+(.+)$").unwrap();
    let entrypoint_regex = Regex::new(r"(?i)^ENTRYPOINT\s+(.+)$").unwrap();
    let env_regex = Regex::new(r"(?i)^ENV\s+(.+)$").unwrap();
    
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        info.instruction_count += 1;
        
        // Parse FROM instructions
        if let Some(captures) = from_regex.captures(line) {
            if info.base_image.is_none() {
                info.base_image = Some(captures.get(1).unwrap().as_str().trim().to_string());
            }
            if let Some(stage_name) = captures.get(2) {
                info.build_stages.push(stage_name.as_str().trim().to_string());
                info.is_multistage = true;
            }
        }
        
        // Parse EXPOSE instructions
        if let Some(captures) = expose_regex.captures(line) {
            let ports_str = captures.get(1).unwrap().as_str();
            for port in ports_str.split_whitespace() {
                if let Ok(port_num) = port.parse::<u16>() {
                    info.exposed_ports.push(port_num);
                }
            }
        }
        
        // Parse WORKDIR
        if let Some(captures) = workdir_regex.captures(line) {
            info.workdir = Some(captures.get(1).unwrap().as_str().trim().to_string());
        }
        
        // Parse CMD and ENTRYPOINT
        if let Some(captures) = cmd_regex.captures(line) {
            if info.entrypoint.is_none() {
                info.entrypoint = Some(captures.get(1).unwrap().as_str().trim().to_string());
            }
        }
        
        if let Some(captures) = entrypoint_regex.captures(line) {
            info.entrypoint = Some(captures.get(1).unwrap().as_str().trim().to_string());
        }
        
        // Parse ENV
        if let Some(captures) = env_regex.captures(line) {
            info.env_vars.push(captures.get(1).unwrap().as_str().trim().to_string());
        }
    }
    
    Ok(info)
}

/// Parses a Docker Compose file and extracts information
fn parse_compose_file(path: &PathBuf) -> Result<ComposeFileInfo> {
    let content = fs::read_to_string(path)?;
    
    // Parse YAML content
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| crate::error::AnalysisError::DependencyParsing {
            file: path.display().to_string(),
            reason: format!("YAML parsing error: {}", e),
        })?;
    
    let mut info = ComposeFileInfo {
        path: path.clone(),
        environment: extract_environment_from_filename(path),
        version: None,
        service_names: Vec::new(),
        networks: Vec::new(),
        volumes: Vec::new(),
        external_dependencies: Vec::new(),
    };
    
    // Extract version
    if let Some(version) = yaml_value.get("version").and_then(|v| v.as_str()) {
        info.version = Some(version.to_string());
    }
    
    // Extract service names
    if let Some(services) = yaml_value.get("services").and_then(|s| s.as_mapping()) {
        for (service_name, _) in services {
            if let Some(name) = service_name.as_str() {
                info.service_names.push(name.to_string());
            }
        }
    }
    
    // Extract networks
    if let Some(networks) = yaml_value.get("networks").and_then(|n| n.as_mapping()) {
        for (network_name, network_config) in networks {
            if let Some(name) = network_name.as_str() {
                info.networks.push(name.to_string());
                
                // Check if it's external
                if let Some(config) = network_config.as_mapping() {
                    if config.get("external").and_then(|e| e.as_bool()).unwrap_or(false) {
                        info.external_dependencies.push(format!("network:{}", name));
                    }
                }
            }
        }
    }
    
    // Extract volumes
    if let Some(volumes) = yaml_value.get("volumes").and_then(|v| v.as_mapping()) {
        for (volume_name, volume_config) in volumes {
            if let Some(name) = volume_name.as_str() {
                info.volumes.push(name.to_string());
                
                // Check if it's external
                if let Some(config) = volume_config.as_mapping() {
                    if config.get("external").and_then(|e| e.as_bool()).unwrap_or(false) {
                        info.external_dependencies.push(format!("volume:{}", name));
                    }
                }
            }
        }
    }
    
    Ok(info)
}

/// Extracts environment from filename (e.g., "dev" from "dockerfile.dev")
fn extract_environment_from_filename(path: &PathBuf) -> Option<String> {
    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
        let filename_lower = filename.to_lowercase();
        
        // Extract environment from patterns like "dockerfile.dev", "docker-compose.prod.yml"
        if let Some(dot_pos) = filename_lower.rfind('.') {
            let before_ext = &filename_lower[..dot_pos];
            if let Some(env_dot_pos) = before_ext.rfind('.') {
                let env = &before_ext[env_dot_pos + 1..];
                
                // Common environment names
                match env {
                    "dev" | "development" | "local" => return Some("development".to_string()),
                    "prod" | "production" => return Some("production".to_string()),
                    "test" | "testing" => return Some("test".to_string()),
                    "stage" | "staging" => return Some("staging".to_string()),
                    _ if env.len() <= 10 => return Some(env.to_string()), // Reasonable env name length
                    _ => {}
                }
            }
        }
    }
    None
}

/// Helper functions for parsing compose files
fn extract_services_from_compose(compose_files: &[ComposeFileInfo]) -> Result<Vec<DockerService>> {
    let mut services = Vec::new();
    
    for compose_file in compose_files {
        let content = fs::read_to_string(&compose_file.path)?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| crate::error::AnalysisError::DependencyParsing {
                file: compose_file.path.display().to_string(),
                reason: format!("YAML parsing error: {}", e),
            })?;
        
        if let Some(services_yaml) = yaml_value.get("services").and_then(|s| s.as_mapping()) {
            for (service_name, service_config) in services_yaml {
                if let (Some(name), Some(config)) = (service_name.as_str(), service_config.as_mapping()) {
                    let service = parse_docker_service(name, config, &compose_file.path)?;
                    services.push(service);
                }
            }
        }
    }
    
    Ok(services)
}

/// Parses a Docker service from compose configuration
fn parse_docker_service(
    name: &str,
    config: &serde_yaml::Mapping,
    compose_file: &PathBuf,
) -> Result<DockerService> {
    let mut service = DockerService {
        name: name.to_string(),
        compose_file: compose_file.clone(),
        image_or_build: ImageOrBuild::Image("unknown".to_string()),
        ports: Vec::new(),
        environment: HashMap::new(),
        depends_on: Vec::new(),
        networks: Vec::new(),
        volumes: Vec::new(),
        health_check: None,
        restart_policy: None,
        resource_limits: None,
    };
    
    // Parse image or build
    if let Some(image) = config.get("image").and_then(|i| i.as_str()) {
        service.image_or_build = ImageOrBuild::Image(image.to_string());
    } else if let Some(build_config) = config.get("build") {
        if let Some(context) = build_config.as_str() {
            service.image_or_build = ImageOrBuild::Build {
                context: context.to_string(),
                dockerfile: None,
                args: HashMap::new(),
            };
        } else if let Some(build_mapping) = build_config.as_mapping() {
            let context = build_mapping.get("context")
                .and_then(|c| c.as_str())
                .unwrap_or(".")
                .to_string();
            
            let dockerfile = build_mapping.get("dockerfile")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            
            let mut args = HashMap::new();
            if let Some(args_config) = build_mapping.get("args").and_then(|a| a.as_mapping()) {
                for (key, value) in args_config {
                    if let (Some(k), Some(v)) = (key.as_str(), value.as_str()) {
                        args.insert(k.to_string(), v.to_string());
                    }
                }
            }
            
            service.image_or_build = ImageOrBuild::Build {
                context,
                dockerfile,
                args,
            };
        }
    }
    
    // Parse ports
    if let Some(ports_config) = config.get("ports").and_then(|p| p.as_sequence()) {
        for port_item in ports_config {
            if let Some(port_mapping) = parse_port_mapping(port_item) {
                service.ports.push(port_mapping);
            }
        }
    }
    
    // Parse environment variables
    if let Some(env_config) = config.get("environment") {
        parse_environment_variables(env_config, &mut service.environment);
    }
    
    // Parse depends_on
    if let Some(depends_config) = config.get("depends_on") {
        if let Some(depends_sequence) = depends_config.as_sequence() {
            for dep in depends_sequence {
                if let Some(dep_name) = dep.as_str() {
                    service.depends_on.push(dep_name.to_string());
                }
            }
        } else if let Some(depends_mapping) = depends_config.as_mapping() {
            for (dep_name, _) in depends_mapping {
                if let Some(name) = dep_name.as_str() {
                    service.depends_on.push(name.to_string());
                }
            }
        }
    }
    
    // Parse networks
    if let Some(networks_config) = config.get("networks") {
        if let Some(networks_sequence) = networks_config.as_sequence() {
            for network in networks_sequence {
                if let Some(network_name) = network.as_str() {
                    service.networks.push(network_name.to_string());
                }
            }
        } else if let Some(networks_mapping) = networks_config.as_mapping() {
            for (network_name, _) in networks_mapping {
                if let Some(name) = network_name.as_str() {
                    service.networks.push(name.to_string());
                }
            }
        }
    }
    
    // Parse volumes
    if let Some(volumes_config) = config.get("volumes").and_then(|v| v.as_sequence()) {
        for volume_item in volumes_config {
            if let Some(volume_mount) = parse_volume_mount(volume_item) {
                service.volumes.push(volume_mount);
            }
        }
    }
    
    // Parse restart policy
    if let Some(restart) = config.get("restart").and_then(|r| r.as_str()) {
        service.restart_policy = Some(restart.to_string());
    }
    
    // Parse health check
    if let Some(healthcheck_config) = config.get("healthcheck").and_then(|h| h.as_mapping()) {
        if let Some(test) = healthcheck_config.get("test").and_then(|t| t.as_str()) {
            service.health_check = Some(HealthCheck {
                test: test.to_string(),
                interval: healthcheck_config.get("interval").and_then(|i| i.as_str()).map(|s| s.to_string()),
                timeout: healthcheck_config.get("timeout").and_then(|t| t.as_str()).map(|s| s.to_string()),
                retries: healthcheck_config.get("retries").and_then(|r| r.as_u64()).map(|r| r as u32),
            });
        }
    }
    
    Ok(service)
}

/// Parses port mapping from YAML value
fn parse_port_mapping(port_value: &serde_yaml::Value) -> Option<PortMapping> {
    if let Some(port_str) = port_value.as_str() {
        // Handle string format like "8080:80" or "80"
        if let Some(colon_pos) = port_str.find(':') {
            let host_part = &port_str[..colon_pos];
            let container_part = &port_str[colon_pos + 1..];
            
            if let (Ok(host_port), Ok(container_port)) = (host_part.parse::<u16>(), container_part.parse::<u16>()) {
                return Some(PortMapping {
                    host_port: Some(host_port),
                    container_port,
                    protocol: "tcp".to_string(),
                    exposed_to_host: true,
                });
            }
        } else if let Ok(container_port) = port_str.parse::<u16>() {
            return Some(PortMapping {
                host_port: None,
                container_port,
                protocol: "tcp".to_string(),
                exposed_to_host: false,
            });
        }
    } else if let Some(port_num) = port_value.as_u64() {
        return Some(PortMapping {
            host_port: None,
            container_port: port_num as u16,
            protocol: "tcp".to_string(),
            exposed_to_host: false,
        });
    }
    
    None
}

/// Parses volume mount from YAML value
fn parse_volume_mount(volume_value: &serde_yaml::Value) -> Option<VolumeMount> {
    if let Some(volume_str) = volume_value.as_str() {
        // Handle string format like "./data:/app/data:ro" or "./data:/app/data"
        let parts: Vec<&str> = volume_str.split(':').collect();
        if parts.len() >= 2 {
            return Some(VolumeMount {
                source: parts[0].to_string(),
                target: parts[1].to_string(),
                mount_type: if parts[0].starts_with('/') || parts[0].starts_with('.') {
                    "bind".to_string()
                } else {
                    "volume".to_string()
                },
                read_only: parts.get(2).map_or(false, |&opt| opt == "ro"),
            });
        }
    }
    None
}

/// Parses environment variables from YAML
fn parse_environment_variables(env_value: &serde_yaml::Value, env_map: &mut HashMap<String, String>) {
    if let Some(env_mapping) = env_value.as_mapping() {
        for (key, value) in env_mapping {
            if let Some(key_str) = key.as_str() {
                let value_str = value.as_str().unwrap_or("").to_string();
                env_map.insert(key_str.to_string(), value_str);
            }
        }
    } else if let Some(env_sequence) = env_value.as_sequence() {
        for env_item in env_sequence {
            if let Some(env_str) = env_item.as_str() {
                if let Some(eq_pos) = env_str.find('=') {
                    let key = env_str[..eq_pos].to_string();
                    let value = env_str[eq_pos + 1..].to_string();
                    env_map.insert(key, value);
                } else {
                    env_map.insert(env_str.to_string(), String::new());
                }
            }
        }
    }
}

fn analyze_networking(
    services: &[DockerService],
    compose_files: &[ComposeFileInfo],
) -> Result<NetworkingConfig> {
    let mut custom_networks = Vec::new();
    let mut connected_services: HashMap<String, Vec<String>> = HashMap::new();
    
    // Collect networks from compose files
    for compose_file in compose_files {
        for network_name in &compose_file.networks {
            let network_info = NetworkInfo {
                name: network_name.clone(),
                driver: None, // TODO: Parse driver from compose file
                external: compose_file.external_dependencies.contains(&format!("network:{}", network_name)),
                connected_services: Vec::new(),
            };
            custom_networks.push(network_info);
        }
    }
    
    // Map services to networks
    for service in services {
        for network in &service.networks {
            connected_services
                .entry(network.clone())
                .or_insert_with(Vec::new)
                .push(service.name.clone());
        }
    }
    
    // Update network info with connected services
    for network in &mut custom_networks {
        if let Some(services) = connected_services.get(&network.name) {
            network.connected_services = services.clone();
        }
    }
    
    // Analyze service discovery
    let service_discovery = ServiceDiscoveryConfig {
        internal_dns: !services.is_empty(), // Docker Compose provides internal DNS
        external_tools: detect_service_discovery_tools(services),
        service_mesh: detect_service_mesh(services),
    };
    
    // Analyze load balancing
    let load_balancing = detect_load_balancers(services);
    
    // Analyze external connectivity
    let external_connectivity = analyze_external_connectivity(services);
    
    Ok(NetworkingConfig {
        custom_networks,
        service_discovery,
        load_balancing,
        external_connectivity,
    })
}

fn determine_orchestration_pattern(
    services: &[DockerService],
    networking: &NetworkingConfig,
) -> OrchestrationPattern {
    if services.is_empty() {
        return OrchestrationPattern::SingleContainer;
    }
    
    if services.len() == 1 {
        return OrchestrationPattern::SingleContainer;
    }
    
    // Check for microservices patterns
    let has_multiple_backends = services.iter()
        .filter(|s| match &s.image_or_build {
            ImageOrBuild::Image(img) => !img.contains("nginx") && !img.contains("proxy") && !img.contains("traefik"),
            _ => true,
        })
        .count() > 2;
    
    let has_service_discovery = networking.service_discovery.internal_dns || 
                               !networking.service_discovery.external_tools.is_empty();
    
    let has_load_balancing = !networking.load_balancing.is_empty();
    
    let has_message_queues = services.iter().any(|s| match &s.image_or_build {
        ImageOrBuild::Image(img) => {
            img.contains("redis") || img.contains("rabbitmq") || 
            img.contains("kafka") || img.contains("nats")
        },
        _ => false,
    });
    
    if networking.service_discovery.service_mesh {
        OrchestrationPattern::ServiceMesh
    } else if has_message_queues && has_multiple_backends {
        OrchestrationPattern::EventDriven
    } else if has_multiple_backends && has_service_discovery {
        OrchestrationPattern::Microservices
    } else if has_load_balancing || services.len() > 3 {
        OrchestrationPattern::DockerCompose
    } else {
        OrchestrationPattern::DockerCompose
    }
}

/// Detects service discovery tools in the services
fn detect_service_discovery_tools(services: &[DockerService]) -> Vec<String> {
    let mut tools = Vec::new();
    
    for service in services {
        if let ImageOrBuild::Image(image) = &service.image_or_build {
            if image.contains("consul") {
                tools.push("consul".to_string());
            }
            if image.contains("etcd") {
                tools.push("etcd".to_string());
            }
            if image.contains("zookeeper") {
                tools.push("zookeeper".to_string());
            }
        }
    }
    
    tools.sort();
    tools.dedup();
    tools
}

/// Detects service mesh presence
fn detect_service_mesh(services: &[DockerService]) -> bool {
    services.iter().any(|s| {
        if let ImageOrBuild::Image(image) = &s.image_or_build {
            image.contains("istio") || image.contains("linkerd") || 
            image.contains("envoy") || image.contains("consul-connect")
        } else {
            false
        }
    })
}

/// Detects load balancers in the services
fn detect_load_balancers(services: &[DockerService]) -> Vec<LoadBalancerConfig> {
    let mut load_balancers = Vec::new();
    
    for service in services {
        // Check if service image indicates a load balancer
        let is_load_balancer = match &service.image_or_build {
            ImageOrBuild::Image(image) => {
                image.contains("nginx") || 
                image.contains("traefik") || 
                image.contains("haproxy") ||
                image.contains("envoy") ||
                image.contains("kong")
            },
            _ => false,
        };
        
        if is_load_balancer {
            // Find potential backend services (services this one doesn't depend on)
            let backends: Vec<String> = services
                .iter()
                .filter(|s| s.name != service.name && !service.depends_on.contains(&s.name))
                .map(|s| s.name.clone())
                .collect();
            
            if !backends.is_empty() {
                let lb_type = match &service.image_or_build {
                    ImageOrBuild::Image(image) => {
                        if image.contains("nginx") { "nginx" }
                        else if image.contains("traefik") { "traefik" }
                        else if image.contains("haproxy") { "haproxy" }
                        else if image.contains("envoy") { "envoy" }
                        else if image.contains("kong") { "kong" }
                        else { "unknown" }
                    },
                    _ => "unknown",
                };
                
                load_balancers.push(LoadBalancerConfig {
                    service: service.name.clone(),
                    lb_type: lb_type.to_string(),
                    backends,
                });
            }
        }
    }
    
    load_balancers
}

/// Analyzes external connectivity patterns
fn analyze_external_connectivity(services: &[DockerService]) -> ExternalConnectivity {
    let mut exposed_services = Vec::new();
    let mut ingress_patterns = Vec::new();
    let mut api_gateways = Vec::new();
    
    for service in services {
        let mut external_ports = Vec::new();
        let mut protocols = Vec::new();
        
        // Check for exposed ports
        for port in &service.ports {
            if port.exposed_to_host {
                if let Some(host_port) = port.host_port {
                    external_ports.push(host_port);
                }
                protocols.push(port.protocol.clone());
            }
        }
        
        if !external_ports.is_empty() {
            // Check for SSL/TLS indicators
            let ssl_enabled = external_ports.contains(&443) || 
                            external_ports.contains(&8443) ||
                            service.environment.keys().any(|k| k.to_lowercase().contains("ssl") || k.to_lowercase().contains("tls"));
            
            exposed_services.push(ExposedService {
                service: service.name.clone(),
                external_ports,
                protocols: protocols.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect(),
                ssl_enabled,
            });
        }
        
        // Detect API gateways
        if service.name.to_lowercase().contains("gateway") || 
           service.name.to_lowercase().contains("api") ||
           service.name.to_lowercase().contains("proxy") {
            api_gateways.push(service.name.clone());
        }
        
        // Also check image for API gateway patterns
        if let ImageOrBuild::Image(image) = &service.image_or_build {
            if image.contains("kong") || image.contains("zuul") || 
               image.contains("ambassador") || image.contains("traefik") {
                if !api_gateways.contains(&service.name) {
                    api_gateways.push(service.name.clone());
                }
            }
        }
    }
    
    // Detect ingress patterns
    if exposed_services.len() == 1 && api_gateways.len() == 1 {
        ingress_patterns.push("Single API Gateway".to_string());
    } else if exposed_services.len() > 1 && api_gateways.is_empty() {
        ingress_patterns.push("Multiple Direct Entry Points".to_string());
    } else if !api_gateways.is_empty() {
        ingress_patterns.push("API Gateway Pattern".to_string());
    }
    
    // Detect reverse proxy patterns
    let has_reverse_proxy = services.iter().any(|s| {
        if let ImageOrBuild::Image(image) = &s.image_or_build {
            image.contains("nginx") || image.contains("apache") || image.contains("caddy")
        } else {
            false
        }
    });
    
    if has_reverse_proxy {
        ingress_patterns.push("Reverse Proxy".to_string());
    }
    
    ExternalConnectivity {
        exposed_services,
        ingress_patterns,
        api_gateways,
    }
}

fn analyze_environments(
    dockerfiles: &[DockerfileInfo],
    compose_files: &[ComposeFileInfo],
) -> Vec<DockerEnvironment> {
    let mut environments: HashMap<String, DockerEnvironment> = HashMap::new();
    
    // Collect environments from Dockerfiles
    for dockerfile in dockerfiles {
        let env_name = dockerfile.environment.clone().unwrap_or_else(|| "default".to_string());
        environments
            .entry(env_name.clone())
            .or_insert_with(|| DockerEnvironment {
                name: env_name,
                dockerfiles: Vec::new(),
                compose_files: Vec::new(),
                config_overrides: HashMap::new(),
            })
            .dockerfiles
            .push(dockerfile.path.clone());
    }
    
    // Collect environments from Compose files
    for compose_file in compose_files {
        let env_name = compose_file.environment.clone().unwrap_or_else(|| "default".to_string());
        environments
            .entry(env_name.clone())
            .or_insert_with(|| DockerEnvironment {
                name: env_name,
                dockerfiles: Vec::new(),
                compose_files: Vec::new(),
                config_overrides: HashMap::new(),
            })
            .compose_files
            .push(compose_file.path.clone());
    }
    
    environments.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_dockerfile_name() {
        assert!(is_dockerfile_name("Dockerfile"));
        assert!(is_dockerfile_name("dockerfile"));
        assert!(is_dockerfile_name("Dockerfile.dev"));
        assert!(is_dockerfile_name("dockerfile.prod"));
        assert!(is_dockerfile_name("api.dockerfile"));
        assert!(!is_dockerfile_name("README.md"));
        assert!(!is_dockerfile_name("package.json"));
    }
    
    #[test]
    fn test_is_compose_file_name() {
        assert!(is_compose_file_name("docker-compose.yml"));
        assert!(is_compose_file_name("docker-compose.yaml"));
        assert!(is_compose_file_name("docker-compose.dev.yml"));
        assert!(is_compose_file_name("docker-compose.prod.yaml"));
        assert!(is_compose_file_name("compose.yml"));
        assert!(is_compose_file_name("compose.yaml"));
        assert!(!is_compose_file_name("README.md"));
        assert!(!is_compose_file_name("package.json"));
    }
    
    #[test]
    fn test_extract_environment_from_filename() {
        assert_eq!(
            extract_environment_from_filename(&PathBuf::from("Dockerfile.dev")),
            Some("development".to_string())
        );
        assert_eq!(
            extract_environment_from_filename(&PathBuf::from("docker-compose.prod.yml")),
            Some("production".to_string())
        );
        assert_eq!(
            extract_environment_from_filename(&PathBuf::from("Dockerfile")),
            None
        );
    }
} 