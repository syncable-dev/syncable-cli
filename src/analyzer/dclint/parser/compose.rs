//! Docker Compose file structure types.
//!
//! Defines the structure of a docker-compose.yaml file with support for
//! position tracking.

use std::collections::HashMap;
use yaml_rust2::{Yaml, YamlLoader};

/// Error type for parsing.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("YAML parse error: {0}")]
    YamlError(String),
    #[error("Empty document")]
    EmptyDocument,
    #[error("Invalid structure: {0}")]
    InvalidStructure(String),
}

/// Position in the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

impl Position {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

/// Parsed Docker Compose file.
#[derive(Debug, Clone, Default)]
pub struct ComposeFile {
    /// The deprecated `version` field.
    pub version: Option<String>,
    /// Position of the version field.
    pub version_pos: Option<Position>,
    /// The `name` field (project name).
    pub name: Option<String>,
    /// Position of the name field.
    pub name_pos: Option<Position>,
    /// Services defined in the compose file.
    pub services: HashMap<String, Service>,
    /// Position of the services section.
    pub services_pos: Option<Position>,
    /// Networks defined in the compose file.
    pub networks: HashMap<String, serde_json::Value>,
    /// Volumes defined in the compose file.
    pub volumes: HashMap<String, serde_json::Value>,
    /// Configs defined in the compose file.
    pub configs: HashMap<String, serde_json::Value>,
    /// Secrets defined in the compose file.
    pub secrets: HashMap<String, serde_json::Value>,
    /// Top-level key order (for ordering rules).
    pub top_level_keys: Vec<String>,
    /// Raw source content for position lookups.
    pub source: String,
}

/// A service definition.
#[derive(Debug, Clone, Default)]
pub struct Service {
    /// Service name.
    pub name: String,
    /// Position of the service definition.
    pub position: Position,
    /// The image to use.
    pub image: Option<String>,
    /// Position of the image field.
    pub image_pos: Option<Position>,
    /// Build configuration.
    pub build: Option<ServiceBuild>,
    /// Position of the build field.
    pub build_pos: Option<Position>,
    /// Container name.
    pub container_name: Option<String>,
    /// Position of the container_name field.
    pub container_name_pos: Option<Position>,
    /// Port mappings.
    pub ports: Vec<ServicePort>,
    /// Position of the ports field.
    pub ports_pos: Option<Position>,
    /// Volume mounts.
    pub volumes: Vec<ServiceVolume>,
    /// Position of the volumes field.
    pub volumes_pos: Option<Position>,
    /// Service dependencies.
    pub depends_on: Vec<String>,
    /// Position of the depends_on field.
    pub depends_on_pos: Option<Position>,
    /// Environment variables.
    pub environment: HashMap<String, String>,
    /// Pull policy (for build+image combinations).
    pub pull_policy: Option<String>,
    /// All keys in this service (for ordering rules).
    pub keys: Vec<String>,
    /// Raw YAML for this service.
    pub raw: Option<Yaml>,
}

/// Build configuration for a service.
#[derive(Debug, Clone)]
pub enum ServiceBuild {
    /// Simple build context path.
    Simple(String),
    /// Extended build configuration.
    Extended {
        context: Option<String>,
        dockerfile: Option<String>,
        args: HashMap<String, String>,
        target: Option<String>,
    },
}

impl Default for ServiceBuild {
    fn default() -> Self {
        Self::Simple(".".to_string())
    }
}

/// Port mapping for a service.
#[derive(Debug, Clone)]
pub struct ServicePort {
    /// Raw port string (e.g., "8080:80" or "80").
    pub raw: String,
    /// Position in the source.
    pub position: Position,
    /// Whether the port is quoted in source.
    pub is_quoted: bool,
    /// Host port (if specified).
    pub host_port: Option<u16>,
    /// Container port.
    pub container_port: u16,
    /// Host IP binding (e.g., "127.0.0.1").
    pub host_ip: Option<String>,
    /// Protocol (tcp/udp).
    pub protocol: Option<String>,
}

impl ServicePort {
    /// Parse a port string.
    pub fn parse(raw: &str, position: Position, is_quoted: bool) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }

        // Handle protocol suffix
        let (port_part, protocol) = if raw.contains('/') {
            let parts: Vec<&str> = raw.rsplitn(2, '/').collect();
            (parts[1], Some(parts[0].to_string()))
        } else {
            (raw, None)
        };

        // Handle different formats:
        // - "80" (container only)
        // - "8080:80" (host:container)
        // - "127.0.0.1:8080:80" (ip:host:container)
        // - "80-90:80-90" (range)
        let parts: Vec<&str> = port_part.split(':').collect();

        let (host_ip, host_port, container_port) = match parts.len() {
            1 => {
                // Just container port
                let cp = parts[0].parse().ok()?;
                (None, None, cp)
            }
            2 => {
                // host:container
                let hp = parts[0].parse().ok();
                let cp = parts[1].parse().ok()?;
                (None, hp, cp)
            }
            3 => {
                // ip:host:container
                let ip = Some(parts[0].to_string());
                let hp = parts[1].parse().ok();
                let cp = parts[2].parse().ok()?;
                (ip, hp, cp)
            }
            _ => return None,
        };

        Some(Self {
            raw: raw.to_string(),
            position,
            is_quoted,
            host_port,
            container_port,
            host_ip,
            protocol,
        })
    }

    /// Check if this port has an explicit host binding interface.
    pub fn has_explicit_interface(&self) -> bool {
        self.host_ip.is_some()
    }

    /// Get the exported port (for duplicate checking).
    pub fn exported_port(&self) -> Option<String> {
        self.host_port.map(|p| {
            if let Some(ip) = &self.host_ip {
                format!("{}:{}", ip, p)
            } else {
                p.to_string()
            }
        })
    }
}

/// Volume mount for a service.
#[derive(Debug, Clone)]
pub struct ServiceVolume {
    /// Raw volume string.
    pub raw: String,
    /// Position in the source.
    pub position: Position,
    /// Whether the volume is quoted in source.
    pub is_quoted: bool,
    /// Source path or volume name.
    pub source: Option<String>,
    /// Target mount path in container.
    pub target: String,
    /// Mount options (ro, rw, etc.).
    pub options: Option<String>,
}

impl ServiceVolume {
    /// Parse a volume string.
    pub fn parse(raw: &str, position: Position, is_quoted: bool) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }

        // Handle different formats:
        // - "/path" (anonymous volume at path)
        // - "name:/path" (named volume)
        // - "/host:/container" (bind mount)
        // - "/host:/container:ro" (bind mount with options)
        let parts: Vec<&str> = raw.splitn(3, ':').collect();

        let (source, target, options) = match parts.len() {
            1 => (None, parts[0].to_string(), None),
            2 => (Some(parts[0].to_string()), parts[1].to_string(), None),
            3 => (
                Some(parts[0].to_string()),
                parts[1].to_string(),
                Some(parts[2].to_string()),
            ),
            _ => return None,
        };

        Some(Self {
            raw: raw.to_string(),
            position,
            is_quoted,
            source,
            target,
            options,
        })
    }
}

/// Parse a Docker Compose file from a string.
pub fn parse_compose(content: &str) -> Result<ComposeFile, ParseError> {
    parse_compose_with_positions(content)
}

/// Parse a Docker Compose file with position tracking.
pub fn parse_compose_with_positions(content: &str) -> Result<ComposeFile, ParseError> {
    let docs =
        YamlLoader::load_from_str(content).map_err(|e| ParseError::YamlError(e.to_string()))?;

    let doc = docs.into_iter().next().ok_or(ParseError::EmptyDocument)?;

    let hash = match &doc {
        Yaml::Hash(h) => h,
        _ => {
            return Err(ParseError::InvalidStructure(
                "Root must be a mapping".to_string(),
            ));
        }
    };

    let mut compose = ComposeFile {
        source: content.to_string(),
        ..Default::default()
    };

    // Track top-level key order
    for (key, _) in hash {
        if let Yaml::String(k) = key {
            compose.top_level_keys.push(k.clone());
        }
    }

    // Parse version
    if let Some(Yaml::String(version)) = hash.get(&Yaml::String("version".to_string())) {
        compose.version = Some(version.clone());
        compose.version_pos =
            super::find_line_for_key(content, &["version"]).map(|l| Position::new(l, 1));
    }

    // Parse name
    if let Some(Yaml::String(name)) = hash.get(&Yaml::String("name".to_string())) {
        compose.name = Some(name.clone());
        compose.name_pos =
            super::find_line_for_key(content, &["name"]).map(|l| Position::new(l, 1));
    }

    // Parse services
    if let Some(Yaml::Hash(services)) = hash.get(&Yaml::String("services".to_string())) {
        compose.services_pos =
            super::find_line_for_key(content, &["services"]).map(|l| Position::new(l, 1));

        for (name_yaml, service_yaml) in services {
            if let Yaml::String(name) = name_yaml {
                let service = parse_service(name, service_yaml, content)?;
                compose.services.insert(name.clone(), service);
            }
        }
    }

    // Parse networks (as raw JSON for now)
    if let Some(Yaml::Hash(networks)) = hash.get(&Yaml::String("networks".to_string())) {
        for (name_yaml, value_yaml) in networks {
            if let Yaml::String(name) = name_yaml {
                compose
                    .networks
                    .insert(name.clone(), yaml_to_json(value_yaml));
            }
        }
    }

    // Parse volumes (as raw JSON for now)
    if let Some(Yaml::Hash(volumes)) = hash.get(&Yaml::String("volumes".to_string())) {
        for (name_yaml, value_yaml) in volumes {
            if let Yaml::String(name) = name_yaml {
                compose
                    .volumes
                    .insert(name.clone(), yaml_to_json(value_yaml));
            }
        }
    }

    Ok(compose)
}

/// Parse a service definition.
fn parse_service(name: &str, yaml: &Yaml, source: &str) -> Result<Service, ParseError> {
    let hash = match yaml {
        Yaml::Hash(h) => h,
        Yaml::Null => {
            return Ok(Service {
                name: name.to_string(),
                ..Default::default()
            });
        }
        _ => {
            return Err(ParseError::InvalidStructure(format!(
                "Service '{}' must be a mapping",
                name
            )));
        }
    };

    let position = super::find_line_for_service(source, name)
        .map(|l| Position::new(l, 1))
        .unwrap_or_default();

    let mut service = Service {
        name: name.to_string(),
        position,
        raw: Some(yaml.clone()),
        ..Default::default()
    };

    // Track key order
    for (key, _) in hash {
        if let Yaml::String(k) = key {
            service.keys.push(k.clone());
        }
    }

    // Parse image
    if let Some(Yaml::String(image)) = hash.get(&Yaml::String("image".to_string())) {
        service.image = Some(image.clone());
        service.image_pos =
            super::find_line_for_service_key(source, name, "image").map(|l| Position::new(l, 1));
    }

    // Parse build
    if let Some(build_yaml) = hash.get(&Yaml::String("build".to_string())) {
        service.build_pos =
            super::find_line_for_service_key(source, name, "build").map(|l| Position::new(l, 1));

        service.build = Some(match build_yaml {
            Yaml::String(s) => ServiceBuild::Simple(s.clone()),
            Yaml::Hash(h) => {
                let context = h
                    .get(&Yaml::String("context".to_string()))
                    .and_then(|v| match v {
                        Yaml::String(s) => Some(s.clone()),
                        _ => None,
                    });
                let dockerfile =
                    h.get(&Yaml::String("dockerfile".to_string()))
                        .and_then(|v| match v {
                            Yaml::String(s) => Some(s.clone()),
                            _ => None,
                        });
                let target = h
                    .get(&Yaml::String("target".to_string()))
                    .and_then(|v| match v {
                        Yaml::String(s) => Some(s.clone()),
                        _ => None,
                    });

                ServiceBuild::Extended {
                    context,
                    dockerfile,
                    args: HashMap::new(),
                    target,
                }
            }
            _ => ServiceBuild::Simple(".".to_string()),
        });
    }

    // Parse container_name
    if let Some(Yaml::String(container_name)) =
        hash.get(&Yaml::String("container_name".to_string()))
    {
        service.container_name = Some(container_name.clone());
        service.container_name_pos =
            super::find_line_for_service_key(source, name, "container_name")
                .map(|l| Position::new(l, 1));
    }

    // Parse ports
    if let Some(Yaml::Array(ports)) = hash.get(&Yaml::String("ports".to_string())) {
        service.ports_pos =
            super::find_line_for_service_key(source, name, "ports").map(|l| Position::new(l, 1));

        let ports_start_line = service.ports_pos.map(|p| p.line).unwrap_or(1);

        for (idx, port_yaml) in ports.iter().enumerate() {
            let line = ports_start_line + 1 + idx as u32;
            let position = Position::new(line, 1);

            match port_yaml {
                Yaml::String(s) => {
                    // Check if quoted in source
                    let is_quoted = is_value_quoted_at_line(source, line);
                    if let Some(port) = ServicePort::parse(s, position, is_quoted) {
                        service.ports.push(port);
                    }
                }
                Yaml::Integer(i) => {
                    // Integer ports are unquoted
                    let raw = i.to_string();
                    if let Some(port) = ServicePort::parse(&raw, position, false) {
                        service.ports.push(port);
                    }
                }
                Yaml::Hash(h) => {
                    // Long syntax port
                    let target = h
                        .get(&Yaml::String("target".to_string()))
                        .and_then(|v| match v {
                            Yaml::Integer(i) => Some(*i as u16),
                            Yaml::String(s) => s.parse().ok(),
                            _ => None,
                        });
                    let published =
                        h.get(&Yaml::String("published".to_string()))
                            .and_then(|v| match v {
                                Yaml::Integer(i) => Some(*i as u16),
                                Yaml::String(s) => s.parse().ok(),
                                _ => None,
                            });
                    let host_ip =
                        h.get(&Yaml::String("host_ip".to_string()))
                            .and_then(|v| match v {
                                Yaml::String(s) => Some(s.clone()),
                                _ => None,
                            });

                    if let Some(container_port) = target {
                        service.ports.push(ServicePort {
                            raw: format!(
                                "{}:{}",
                                published.unwrap_or(container_port),
                                container_port
                            ),
                            position,
                            is_quoted: false,
                            host_port: published,
                            container_port,
                            host_ip,
                            protocol: None,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // Parse volumes
    if let Some(Yaml::Array(volumes)) = hash.get(&Yaml::String("volumes".to_string())) {
        service.volumes_pos =
            super::find_line_for_service_key(source, name, "volumes").map(|l| Position::new(l, 1));

        let volumes_start_line = service.volumes_pos.map(|p| p.line).unwrap_or(1);

        for (idx, vol_yaml) in volumes.iter().enumerate() {
            let line = volumes_start_line + 1 + idx as u32;
            let position = Position::new(line, 1);

            if let Yaml::String(s) = vol_yaml {
                let is_quoted = is_value_quoted_at_line(source, line);
                if let Some(vol) = ServiceVolume::parse(s, position, is_quoted) {
                    service.volumes.push(vol);
                }
            }
        }
    }

    // Parse depends_on
    if let Some(depends_on_yaml) = hash.get(&Yaml::String("depends_on".to_string())) {
        service.depends_on_pos = super::find_line_for_service_key(source, name, "depends_on")
            .map(|l| Position::new(l, 1));

        match depends_on_yaml {
            Yaml::Array(arr) => {
                for dep in arr {
                    if let Yaml::String(s) = dep {
                        service.depends_on.push(s.clone());
                    }
                }
            }
            Yaml::Hash(h) => {
                // Long syntax: depends_on: { db: { condition: service_healthy } }
                for (dep_name, _) in h {
                    if let Yaml::String(s) = dep_name {
                        service.depends_on.push(s.clone());
                    }
                }
            }
            _ => {}
        }
    }

    // Parse environment
    if let Some(env_yaml) = hash.get(&Yaml::String("environment".to_string())) {
        match env_yaml {
            Yaml::Hash(h) => {
                for (key, value) in h {
                    if let (Yaml::String(k), v) = (key, value) {
                        let val = match v {
                            Yaml::String(s) => s.clone(),
                            Yaml::Integer(i) => i.to_string(),
                            Yaml::Boolean(b) => b.to_string(),
                            Yaml::Null => String::new(),
                            _ => continue,
                        };
                        service.environment.insert(k.clone(), val);
                    }
                }
            }
            Yaml::Array(arr) => {
                for item in arr {
                    if let Yaml::String(s) = item {
                        if let Some((k, v)) = s.split_once('=') {
                            service.environment.insert(k.to_string(), v.to_string());
                        } else {
                            service.environment.insert(s.clone(), String::new());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Parse pull_policy
    if let Some(Yaml::String(pull_policy)) = hash.get(&Yaml::String("pull_policy".to_string())) {
        service.pull_policy = Some(pull_policy.clone());
    }

    Ok(service)
}

/// Check if a value at a given line is quoted in the source.
fn is_value_quoted_at_line(source: &str, line: u32) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    if let Some(line_content) = lines.get((line - 1) as usize) {
        let trimmed = line_content.trim();
        // Check for list item with quoted value
        if trimmed.starts_with('-') {
            let after_dash = trimmed.trim_start_matches('-').trim();
            return after_dash.starts_with('"') || after_dash.starts_with('\'');
        }
        // Check for key: value with quoted value
        if let Some(pos) = trimmed.find(':') {
            let after_colon = trimmed[pos + 1..].trim();
            return after_colon.starts_with('"') || after_colon.starts_with('\'');
        }
    }
    false
}

/// Convert a YAML value to JSON (for raw storage).
fn yaml_to_json(yaml: &Yaml) -> serde_json::Value {
    match yaml {
        Yaml::Null => serde_json::Value::Null,
        Yaml::Boolean(b) => serde_json::Value::Bool(*b),
        Yaml::Integer(i) => serde_json::json!(i),
        Yaml::Real(r) => {
            if let Ok(f) = r.parse::<f64>() {
                serde_json::json!(f)
            } else {
                serde_json::Value::String(r.clone())
            }
        }
        Yaml::String(s) => serde_json::Value::String(s.clone()),
        Yaml::Array(arr) => serde_json::Value::Array(arr.iter().map(yaml_to_json).collect()),
        Yaml::Hash(h) => {
            let mut map = serde_json::Map::new();
            for (k, v) in h {
                if let Yaml::String(key) = k {
                    map.insert(key.clone(), yaml_to_json(v));
                }
            }
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_compose() {
        let yaml = r#"
version: "3.8"
name: myproject
services:
  web:
    image: nginx:latest
    ports:
      - "8080:80"
  db:
    image: postgres:15
"#;

        let compose = parse_compose(yaml).unwrap();
        assert_eq!(compose.version, Some("3.8".to_string()));
        assert_eq!(compose.name, Some("myproject".to_string()));
        assert_eq!(compose.services.len(), 2);

        let web = compose.services.get("web").unwrap();
        assert_eq!(web.image, Some("nginx:latest".to_string()));
        assert_eq!(web.ports.len(), 1);
        assert_eq!(web.ports[0].container_port, 80);
        assert_eq!(web.ports[0].host_port, Some(8080));
    }

    #[test]
    fn test_parse_build_and_image() {
        let yaml = r#"
services:
  app:
    build: .
    image: myapp:latest
"#;

        let compose = parse_compose(yaml).unwrap();
        let app = compose.services.get("app").unwrap();
        assert!(app.build.is_some());
        assert!(app.image.is_some());
    }

    #[test]
    fn test_parse_port_formats() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - 80
      - "8080:80"
      - "127.0.0.1:8081:80"
"#;

        let compose = parse_compose(yaml).unwrap();
        let web = compose.services.get("web").unwrap();
        assert_eq!(web.ports.len(), 3);

        assert_eq!(web.ports[0].container_port, 80);
        assert_eq!(web.ports[0].host_port, None);

        assert_eq!(web.ports[1].container_port, 80);
        assert_eq!(web.ports[1].host_port, Some(8080));

        assert_eq!(web.ports[2].container_port, 80);
        assert_eq!(web.ports[2].host_port, Some(8081));
        assert_eq!(web.ports[2].host_ip, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn test_parse_depends_on() {
        let yaml = r#"
services:
  web:
    image: nginx
    depends_on:
      - db
      - redis
  db:
    image: postgres
  redis:
    image: redis
"#;

        let compose = parse_compose(yaml).unwrap();
        let web = compose.services.get("web").unwrap();
        assert_eq!(web.depends_on, vec!["db", "redis"]);
    }

    #[test]
    fn test_port_parsing() {
        let pos = Position::new(1, 1);

        let p1 = ServicePort::parse("80", pos, false).unwrap();
        assert_eq!(p1.container_port, 80);
        assert_eq!(p1.host_port, None);

        let p2 = ServicePort::parse("8080:80", pos, true).unwrap();
        assert_eq!(p2.container_port, 80);
        assert_eq!(p2.host_port, Some(8080));
        assert!(p2.is_quoted);

        let p3 = ServicePort::parse("127.0.0.1:8080:80", pos, false).unwrap();
        assert_eq!(p3.container_port, 80);
        assert_eq!(p3.host_port, Some(8080));
        assert_eq!(p3.host_ip, Some("127.0.0.1".to_string()));

        let p4 = ServicePort::parse("80/udp", pos, false).unwrap();
        assert_eq!(p4.container_port, 80);
        assert_eq!(p4.protocol, Some("udp".to_string()));
    }

    #[test]
    fn test_volume_parsing() {
        let pos = Position::new(1, 1);

        let v1 = ServiceVolume::parse("/data", pos, false).unwrap();
        assert_eq!(v1.target, "/data");
        assert_eq!(v1.source, None);

        let v2 = ServiceVolume::parse("./host:/container", pos, false).unwrap();
        assert_eq!(v2.source, Some("./host".to_string()));
        assert_eq!(v2.target, "/container");

        let v3 = ServiceVolume::parse("./host:/container:ro", pos, false).unwrap();
        assert_eq!(v3.source, Some("./host".to_string()));
        assert_eq!(v3.target, "/container");
        assert_eq!(v3.options, Some("ro".to_string()));
    }
}
