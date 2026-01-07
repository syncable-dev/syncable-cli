//! Resource specification parsing utilities.
//!
//! Parses Kubernetes resource values (CPU and memory) from their string
//! representations to numeric values for comparison and calculation.

use crate::analyzer::k8s_optimize::types::{ResourceSpec, WorkloadType};
use regex::Regex;
use std::sync::LazyLock;

// ============================================================================
// CPU Parsing
// ============================================================================

/// Regex for parsing CPU values (e.g., "100m", "1", "1.5", "0.1")
static CPU_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+(?:\.\d+)?)(m)?$").unwrap());

/// Parse a CPU value string to millicores.
///
/// # Examples
/// - "100m" -> 100
/// - "1" -> 1000
/// - "1.5" -> 1500
/// - "0.1" -> 100
pub fn parse_cpu_to_millicores(cpu: &str) -> Option<u64> {
    let cpu = cpu.trim();
    if cpu.is_empty() {
        return None;
    }

    if let Some(caps) = CPU_REGEX.captures(cpu) {
        let value: f64 = caps.get(1)?.as_str().parse().ok()?;
        let is_millicores = caps.get(2).is_some();

        if is_millicores {
            Some(value as u64)
        } else {
            Some((value * 1000.0) as u64)
        }
    } else {
        None
    }
}

/// Convert millicores to a human-readable CPU string.
///
/// # Examples
/// - 100 -> "100m"
/// - 1000 -> "1"
/// - 1500 -> "1500m"
pub fn millicores_to_cpu_string(millicores: u64) -> String {
    if millicores >= 1000 && millicores % 1000 == 0 {
        format!("{}", millicores / 1000)
    } else {
        format!("{}m", millicores)
    }
}

// ============================================================================
// Memory Parsing
// ============================================================================

/// Regex for parsing memory values (e.g., "128Mi", "1Gi", "1024Ki", "1000000000")
static MEMORY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+(?:\.\d+)?)(Ki|Mi|Gi|Ti|Pi|Ei|K|M|G|T|P|E)?$").unwrap());

/// Parse a memory value string to bytes.
///
/// # Examples
/// - "128Mi" -> 134217728
/// - "1Gi" -> 1073741824
/// - "1024Ki" -> 1048576
/// - "1000000000" -> 1000000000
pub fn parse_memory_to_bytes(memory: &str) -> Option<u64> {
    let memory = memory.trim();
    if memory.is_empty() {
        return None;
    }

    if let Some(caps) = MEMORY_REGEX.captures(memory) {
        let value: f64 = caps.get(1)?.as_str().parse().ok()?;
        let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let multiplier: f64 = match unit {
            "" => 1.0,
            "Ki" => 1024.0,
            "Mi" => 1024.0 * 1024.0,
            "Gi" => 1024.0 * 1024.0 * 1024.0,
            "Ti" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
            "Pi" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
            "Ei" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
            // Decimal units
            "K" => 1000.0,
            "M" => 1000.0 * 1000.0,
            "G" => 1000.0 * 1000.0 * 1000.0,
            "T" => 1000.0 * 1000.0 * 1000.0 * 1000.0,
            "P" => 1000.0 * 1000.0 * 1000.0 * 1000.0 * 1000.0,
            "E" => 1000.0 * 1000.0 * 1000.0 * 1000.0 * 1000.0 * 1000.0,
            _ => return None,
        };

        Some((value * multiplier) as u64)
    } else {
        None
    }
}

/// Convert bytes to a human-readable memory string (using binary units).
///
/// # Examples
/// - 134217728 -> "128Mi"
/// - 1073741824 -> "1Gi"
pub fn bytes_to_memory_string(bytes: u64) -> String {
    const KI: u64 = 1024;
    const MI: u64 = KI * 1024;
    const GI: u64 = MI * 1024;
    const TI: u64 = GI * 1024;

    if bytes >= TI && bytes % TI == 0 {
        format!("{}Ti", bytes / TI)
    } else if bytes >= GI && bytes % GI == 0 {
        format!("{}Gi", bytes / GI)
    } else if bytes >= MI && bytes % MI == 0 {
        format!("{}Mi", bytes / MI)
    } else if bytes >= KI && bytes % KI == 0 {
        format!("{}Ki", bytes / KI)
    } else if bytes >= MI {
        // Round to Mi for readability
        format!("{}Mi", bytes / MI)
    } else {
        format!("{}", bytes)
    }
}

// ============================================================================
// Resource Spec Parsing from YAML
// ============================================================================

/// Extract resources from a container YAML value.
pub fn extract_resources(container: &serde_yaml::Value) -> ResourceSpec {
    let mut spec = ResourceSpec::new();

    if let Some(resources) = container.get("resources") {
        if let Some(requests) = resources.get("requests") {
            if let Some(cpu) = requests.get("cpu") {
                spec.cpu_request = cpu.as_str().map(String::from);
            }
            if let Some(memory) = requests.get("memory") {
                spec.memory_request = memory.as_str().map(String::from);
            }
        }
        if let Some(limits) = resources.get("limits") {
            if let Some(cpu) = limits.get("cpu") {
                spec.cpu_limit = cpu.as_str().map(String::from);
            }
            if let Some(memory) = limits.get("memory") {
                spec.memory_limit = memory.as_str().map(String::from);
            }
        }
    }

    spec
}

/// Extract container name from a container YAML value.
pub fn extract_container_name(container: &serde_yaml::Value) -> Option<String> {
    container.get("name")?.as_str().map(String::from)
}

/// Extract image from a container YAML value.
pub fn extract_container_image(container: &serde_yaml::Value) -> Option<String> {
    container.get("image")?.as_str().map(String::from)
}

// ============================================================================
// Workload Type Detection
// ============================================================================

/// Detect workload type from container image and name.
pub fn detect_workload_type(
    image: Option<&str>,
    container_name: Option<&str>,
    kind: &str,
) -> WorkloadType {
    let image = image.unwrap_or("").to_lowercase();
    let name = container_name.unwrap_or("").to_lowercase();

    // Database indicators
    const DB_IMAGES: &[&str] = &[
        "postgres",
        "mysql",
        "mariadb",
        "mongodb",
        "mongo",
        "redis",
        "memcached",
        "elasticsearch",
        "cassandra",
        "couchdb",
        "cockroach",
        "timescale",
        "influx",
    ];
    for db in DB_IMAGES {
        if image.contains(db) || name.contains(db) {
            // Redis and Memcached are caches
            if *db == "redis" || *db == "memcached" {
                return WorkloadType::Cache;
            }
            return WorkloadType::Database;
        }
    }

    // Message broker indicators
    const BROKER_IMAGES: &[&str] = &["kafka", "rabbitmq", "nats", "pulsar", "activemq", "zeromq"];
    for broker in BROKER_IMAGES {
        if image.contains(broker) || name.contains(broker) {
            return WorkloadType::MessageBroker;
        }
    }

    // ML/AI indicators
    const ML_IMAGES: &[&str] = &[
        "tensorflow",
        "pytorch",
        "nvidia",
        "cuda",
        "gpu",
        "ml",
        "ai",
        "jupyter",
        "notebook",
        "training",
    ];
    for ml in ML_IMAGES {
        if image.contains(ml) || name.contains(ml) {
            return WorkloadType::MachineLearning;
        }
    }

    // Worker indicators
    const WORKER_PATTERNS: &[&str] = &[
        "worker",
        "consumer",
        "processor",
        "handler",
        "queue",
        "celery",
        "sidekiq",
        "resque",
        "bull",
        "bee",
    ];
    for pattern in WORKER_PATTERNS {
        if name.contains(pattern) {
            return WorkloadType::Worker;
        }
    }

    // Job/CronJob kinds are batch
    if kind == "Job" || kind == "CronJob" {
        return WorkloadType::Batch;
    }

    // Web indicators
    const WEB_IMAGES: &[&str] = &[
        "nginx", "apache", "httpd", "caddy", "traefik", "envoy", "api", "web", "frontend",
        "backend", "gateway",
    ];
    for web in WEB_IMAGES {
        if image.contains(web) || name.contains(web) {
            return WorkloadType::Web;
        }
    }

    // Default to general
    WorkloadType::General
}

// ============================================================================
// Ratio Calculations
// ============================================================================

/// Calculate the limit to request ratio for CPU.
pub fn cpu_limit_to_request_ratio(spec: &ResourceSpec) -> Option<f64> {
    let request = parse_cpu_to_millicores(spec.cpu_request.as_deref()?)?;
    let limit = parse_cpu_to_millicores(spec.cpu_limit.as_deref()?)?;

    if request == 0 {
        return None;
    }

    Some(limit as f64 / request as f64)
}

/// Calculate the limit to request ratio for memory.
pub fn memory_limit_to_request_ratio(spec: &ResourceSpec) -> Option<f64> {
    let request = parse_memory_to_bytes(spec.memory_request.as_deref()?)?;
    let limit = parse_memory_to_bytes(spec.memory_limit.as_deref()?)?;

    if request == 0 {
        return None;
    }

    Some(limit as f64 / request as f64)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_millicores() {
        assert_eq!(parse_cpu_to_millicores("100m"), Some(100));
        assert_eq!(parse_cpu_to_millicores("1"), Some(1000));
        assert_eq!(parse_cpu_to_millicores("1.5"), Some(1500));
        assert_eq!(parse_cpu_to_millicores("0.1"), Some(100));
        assert_eq!(parse_cpu_to_millicores("500m"), Some(500));
        assert_eq!(parse_cpu_to_millicores("2000m"), Some(2000));
    }

    #[test]
    fn test_millicores_to_string() {
        assert_eq!(millicores_to_cpu_string(100), "100m");
        assert_eq!(millicores_to_cpu_string(1000), "1");
        assert_eq!(millicores_to_cpu_string(2000), "2");
        assert_eq!(millicores_to_cpu_string(1500), "1500m");
    }

    #[test]
    fn test_parse_memory_bytes() {
        assert_eq!(parse_memory_to_bytes("128Mi"), Some(128 * 1024 * 1024));
        assert_eq!(parse_memory_to_bytes("1Gi"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_memory_to_bytes("1024Ki"), Some(1024 * 1024));
        assert_eq!(parse_memory_to_bytes("1000000000"), Some(1000000000));
    }

    #[test]
    fn test_bytes_to_memory_string() {
        assert_eq!(bytes_to_memory_string(128 * 1024 * 1024), "128Mi");
        assert_eq!(bytes_to_memory_string(1024 * 1024 * 1024), "1Gi");
        assert_eq!(bytes_to_memory_string(1024 * 1024), "1Mi");
    }

    #[test]
    fn test_detect_workload_type() {
        assert_eq!(
            detect_workload_type(Some("postgres:14"), None, "Deployment"),
            WorkloadType::Database
        );
        assert_eq!(
            detect_workload_type(Some("redis:7"), None, "Deployment"),
            WorkloadType::Cache
        );
        assert_eq!(
            detect_workload_type(Some("nginx:latest"), None, "Deployment"),
            WorkloadType::Web
        );
        assert_eq!(
            detect_workload_type(Some("myapp:v1"), Some("worker"), "Deployment"),
            WorkloadType::Worker
        );
        assert_eq!(
            detect_workload_type(Some("myapp:v1"), None, "Job"),
            WorkloadType::Batch
        );
        assert_eq!(
            detect_workload_type(Some("myapp:v1"), None, "Deployment"),
            WorkloadType::General
        );
    }

    #[test]
    fn test_cpu_ratio() {
        let spec = ResourceSpec {
            cpu_request: Some("100m".to_string()),
            cpu_limit: Some("500m".to_string()),
            memory_request: None,
            memory_limit: None,
        };
        let ratio = cpu_limit_to_request_ratio(&spec).unwrap();
        assert!((ratio - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_memory_ratio() {
        let spec = ResourceSpec {
            cpu_request: None,
            cpu_limit: None,
            memory_request: Some("256Mi".to_string()),
            memory_limit: Some("1Gi".to_string()),
        };
        let ratio = memory_limit_to_request_ratio(&spec).unwrap();
        assert!((ratio - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_resources() {
        let yaml = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
            name: nginx
            image: nginx:1.21
            resources:
              requests:
                cpu: 100m
                memory: 128Mi
              limits:
                cpu: 500m
                memory: 512Mi
            "#,
        )
        .unwrap();

        let spec = extract_resources(&yaml);
        assert_eq!(spec.cpu_request, Some("100m".to_string()));
        assert_eq!(spec.memory_request, Some("128Mi".to_string()));
        assert_eq!(spec.cpu_limit, Some("500m".to_string()));
        assert_eq!(spec.memory_limit, Some("512Mi".to_string()));
    }
}
