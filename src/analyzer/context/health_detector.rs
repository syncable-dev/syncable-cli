//! Health endpoint detection for deployment recommendations.
//!
//! Detects health check endpoints by analyzing:
//! - Source code patterns (route definitions)
//! - Framework conventions (Spring Actuator, etc.)
//! - Configuration files (K8s manifests)

use crate::analyzer::{DetectedTechnology, HealthEndpoint, HealthEndpointSource, TechnologyCategory};
use crate::common::file_utils::{is_readable_file, read_file_safe};
use crate::error::Result;
use regex::Regex;
use std::path::Path;

/// Common health check paths to scan for
const COMMON_HEALTH_PATHS: &[&str] = &[
    "/health",
    "/healthz",
    "/ready",
    "/readyz",
    "/livez",
    "/live",
    "/api/health",
    "/api/v1/health",
    "/__health",
    "/ping",
    "/status",
];

/// Detects health endpoints from project analysis
pub fn detect_health_endpoints(
    project_root: &Path,
    technologies: &[DetectedTechnology],
    max_file_size: usize,
) -> Vec<HealthEndpoint> {
    let mut endpoints = Vec::new();

    // Check framework-specific defaults first
    for tech in technologies {
        if let Some(endpoint) = get_framework_health_endpoint(tech) {
            endpoints.push(endpoint);
        }
    }

    // Scan source files for health route definitions
    let detected_from_code = scan_for_health_routes(project_root, technologies, max_file_size);
    for endpoint in detected_from_code {
        // Avoid duplicates - prefer code-detected over framework defaults
        if !endpoints.iter().any(|e| e.path == endpoint.path) {
            endpoints.push(endpoint);
        } else {
            // Upgrade existing endpoint if code detection has higher confidence
            if let Some(existing) = endpoints.iter_mut().find(|e| e.path == endpoint.path) {
                if endpoint.confidence > existing.confidence {
                    *existing = endpoint;
                }
            }
        }
    }

    // Sort by confidence (highest first)
    endpoints.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

    endpoints
}

/// Get framework-specific health endpoint defaults
fn get_framework_health_endpoint(tech: &DetectedTechnology) -> Option<HealthEndpoint> {
    match tech.name.as_str() {
        // Java frameworks
        "Spring Boot" => Some(HealthEndpoint::from_framework("/actuator/health", "Spring Boot Actuator")),
        "Quarkus" => Some(HealthEndpoint::from_framework("/q/health", "Quarkus SmallRye Health")),
        "Micronaut" => Some(HealthEndpoint::from_framework("/health", "Micronaut")),

        // Node.js frameworks - no standard, but common patterns
        "Express" | "Fastify" | "Koa" | "Hono" | "Elysia" | "NestJS" => {
            // Return a lower confidence endpoint since these don't have a standard
            Some(HealthEndpoint {
                path: "/health".to_string(),
                confidence: 0.5,
                source: HealthEndpointSource::FrameworkDefault,
                description: Some(format!("{} common health pattern", tech.name)),
            })
        }

        // Python frameworks
        "FastAPI" => Some(HealthEndpoint::from_framework("/health", "FastAPI")),
        "Django" => Some(HealthEndpoint {
            path: "/health/".to_string(), // Django uses trailing slashes
            confidence: 0.5,
            source: HealthEndpointSource::FrameworkDefault,
            description: Some("Django common health pattern".to_string()),
        }),
        "Flask" => Some(HealthEndpoint {
            path: "/health".to_string(),
            confidence: 0.5,
            source: HealthEndpointSource::FrameworkDefault,
            description: Some("Flask common health pattern".to_string()),
        }),

        // Go frameworks
        "Gin" | "Echo" | "Fiber" | "Chi" => Some(HealthEndpoint {
            path: "/health".to_string(),
            confidence: 0.5,
            source: HealthEndpointSource::FrameworkDefault,
            description: Some(format!("{} common health pattern", tech.name)),
        }),

        // Rust frameworks
        "Actix Web" | "Axum" | "Rocket" => Some(HealthEndpoint {
            path: "/health".to_string(),
            confidence: 0.5,
            source: HealthEndpointSource::FrameworkDefault,
            description: Some(format!("{} common health pattern", tech.name)),
        }),

        _ => None,
    }
}

/// Scan source files for health route definitions
fn scan_for_health_routes(
    project_root: &Path,
    technologies: &[DetectedTechnology],
    max_file_size: usize,
) -> Vec<HealthEndpoint> {
    let mut endpoints = Vec::new();

    // Determine which file types to scan based on detected technologies
    let has_js = technologies.iter().any(|t| {
        matches!(t.category, TechnologyCategory::BackendFramework | TechnologyCategory::MetaFramework)
            && (t.name.contains("Express") || t.name.contains("Fastify") || t.name.contains("Koa")
                || t.name.contains("Hono") || t.name.contains("Elysia") || t.name.contains("NestJS")
                || t.name.contains("Next") || t.name.contains("Nuxt"))
    });

    let has_python = technologies.iter().any(|t| {
        matches!(t.category, TechnologyCategory::BackendFramework)
            && (t.name.contains("FastAPI") || t.name.contains("Flask") || t.name.contains("Django"))
    });

    let has_go = technologies.iter().any(|t| {
        matches!(t.category, TechnologyCategory::BackendFramework)
            && (t.name.contains("Gin") || t.name.contains("Echo") || t.name.contains("Fiber") || t.name.contains("Chi"))
    });

    let has_rust = technologies.iter().any(|t| {
        matches!(t.category, TechnologyCategory::BackendFramework)
            && (t.name.contains("Actix") || t.name.contains("Axum") || t.name.contains("Rocket"))
    });

    let has_java = technologies.iter().any(|t| {
        matches!(t.category, TechnologyCategory::BackendFramework)
            && (t.name.contains("Spring") || t.name.contains("Quarkus") || t.name.contains("Micronaut"))
    });

    // Common locations to check
    let locations = [
        "src/",
        "app/",
        "routes/",
        "api/",
        "server/",
        "lib/",
        "handlers/",
        "controllers/",
    ];

    for location in &locations {
        let dir = project_root.join(location);
        if dir.is_dir() {
            if has_js {
                scan_directory_for_patterns(&dir, &["js", "ts", "mjs"], &js_health_patterns(), max_file_size, &mut endpoints);
            }
            if has_python {
                scan_directory_for_patterns(&dir, &["py"], &python_health_patterns(), max_file_size, &mut endpoints);
            }
            if has_go {
                scan_directory_for_patterns(&dir, &["go"], &go_health_patterns(), max_file_size, &mut endpoints);
            }
            if has_rust {
                scan_directory_for_patterns(&dir, &["rs"], &rust_health_patterns(), max_file_size, &mut endpoints);
            }
            if has_java {
                scan_directory_for_patterns(&dir, &["java", "kt"], &java_health_patterns(), max_file_size, &mut endpoints);
            }
        }
    }

    // Also check root-level files
    if has_js {
        for entry in ["index.js", "index.ts", "app.js", "app.ts", "server.js", "server.ts", "main.js", "main.ts"] {
            let path = project_root.join(entry);
            if is_readable_file(&path) {
                scan_file_for_patterns(&path, &js_health_patterns(), max_file_size, &mut endpoints);
            }
        }
    }
    if has_python {
        for entry in ["main.py", "app.py", "wsgi.py", "asgi.py"] {
            let path = project_root.join(entry);
            if is_readable_file(&path) {
                scan_file_for_patterns(&path, &python_health_patterns(), max_file_size, &mut endpoints);
            }
        }
    }
    if has_go {
        let main_go = project_root.join("main.go");
        if is_readable_file(&main_go) {
            scan_file_for_patterns(&main_go, &go_health_patterns(), max_file_size, &mut endpoints);
        }
    }
    if has_rust {
        let main_rs = project_root.join("src/main.rs");
        if is_readable_file(&main_rs) {
            scan_file_for_patterns(&main_rs, &rust_health_patterns(), max_file_size, &mut endpoints);
        }
    }

    endpoints
}

/// Scan a directory for health route patterns
fn scan_directory_for_patterns(
    dir: &Path,
    extensions: &[&str],
    patterns: &[(&str, f32)],
    max_file_size: usize,
    endpoints: &mut Vec<HealthEndpoint>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if extensions.iter().any(|e| ext == *e) {
                        scan_file_for_patterns(&path, patterns, max_file_size, endpoints);
                    }
                }
            } else if path.is_dir() {
                // Skip common non-source directories
                let dir_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if !["node_modules", ".git", "target", "build", "dist", "__pycache__", ".next", "vendor"].contains(&dir_name.as_str()) {
                    scan_directory_for_patterns(&path, extensions, patterns, max_file_size, endpoints);
                }
            }
        }
    }
}

/// Scan a single file for health route patterns
fn scan_file_for_patterns(
    path: &Path,
    patterns: &[(&str, f32)],
    max_file_size: usize,
    endpoints: &mut Vec<HealthEndpoint>,
) {
    if let Ok(content) = read_file_safe(path, max_file_size) {
        for (pattern, confidence) in patterns {
            if let Ok(regex) = Regex::new(pattern) {
                for cap in regex.captures_iter(&content) {
                    if let Some(path_match) = cap.get(1) {
                        let health_path = path_match.as_str().to_string();
                        // Only add if it looks like a health endpoint
                        if COMMON_HEALTH_PATHS.iter().any(|p| health_path.contains(p) || p.contains(&health_path)) {
                            if !endpoints.iter().any(|e| e.path == health_path) {
                                endpoints.push(HealthEndpoint {
                                    path: health_path,
                                    confidence: *confidence,
                                    source: HealthEndpointSource::CodePattern,
                                    description: Some(format!("Found in {}", path.display())),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

/// JavaScript/TypeScript health route patterns
fn js_health_patterns() -> Vec<(&'static str, f32)> {
    vec![
        // Express/Fastify/Koa style: app.get('/health', ...)
        (r#"\.(?:get|route)\s*\(\s*['"]([^'"]*(?:health|ready|live|status|ping)[^'"]*)['"]"#, 0.9),
        // NestJS style: @Get('health')
        (r#"@Get\s*\(\s*['"]([^'"]*(?:health|ready|live|status|ping)[^'"]*)['"]"#, 0.9),
        // Hono/Elysia style: .get('/health', ...)
        (r#"\.get\s*\(\s*['"]([^'"]*(?:health|ready|live|status|ping)[^'"]*)['"]"#, 0.9),
    ]
}

/// Python health route patterns
fn python_health_patterns() -> Vec<(&'static str, f32)> {
    vec![
        // FastAPI/Flask style: @app.get("/health")
        (r#"@\w+\.(?:get|route)\s*\(\s*['"]([^'"]*(?:health|ready|live|status|ping)[^'"]*)['"]"#, 0.9),
        // Django URL patterns: path('health/', ...)
        (r#"path\s*\(\s*['"]([^'"]*(?:health|ready|live|status|ping)[^'"]*)['"]"#, 0.85),
    ]
}

/// Go health route patterns
fn go_health_patterns() -> Vec<(&'static str, f32)> {
    vec![
        // http.HandleFunc("/health", ...)
        (r#"HandleFunc\s*\(\s*"([^"]*(?:health|ready|live|status|ping)[^"]*)"#, 0.9),
        // Gin/Echo: r.GET("/health", ...)
        (r#"\.(?:GET|Handle)\s*\(\s*"([^"]*(?:health|ready|live|status|ping)[^"]*)"#, 0.9),
    ]
}

/// Rust health route patterns
fn rust_health_patterns() -> Vec<(&'static str, f32)> {
    vec![
        // Actix: .route("/health", ...)
        (r#"\.route\s*\(\s*"([^"]*(?:health|ready|live|status|ping)[^"]*)"#, 0.9),
        // Axum: .route("/health", get(...))
        (r#"\.route\s*\(\s*"([^"]*(?:health|ready|live|status|ping)[^"]*)"#, 0.9),
    ]
}

/// Java health route patterns
fn java_health_patterns() -> Vec<(&'static str, f32)> {
    vec![
        // Spring: @GetMapping("/health")
        (r#"@(?:Get|Request)Mapping\s*\(\s*(?:value\s*=\s*)?["']([^"']*(?:health|ready|live|status|ping)[^"']*)["']"#, 0.9),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_boot_health_endpoint() {
        let tech = DetectedTechnology {
            name: "Spring Boot".to_string(),
            version: None,
            category: TechnologyCategory::BackendFramework,
            confidence: 0.9,
            requires: vec![],
            conflicts_with: vec![],
            is_primary: true,
            file_indicators: vec![],
        };

        let endpoint = get_framework_health_endpoint(&tech).unwrap();
        assert_eq!(endpoint.path, "/actuator/health");
        assert_eq!(endpoint.confidence, 0.7);
    }

    #[test]
    fn test_express_health_endpoint() {
        let tech = DetectedTechnology {
            name: "Express".to_string(),
            version: None,
            category: TechnologyCategory::BackendFramework,
            confidence: 0.9,
            requires: vec![],
            conflicts_with: vec![],
            is_primary: true,
            file_indicators: vec![],
        };

        let endpoint = get_framework_health_endpoint(&tech).unwrap();
        assert_eq!(endpoint.path, "/health");
        assert_eq!(endpoint.confidence, 0.5); // Lower confidence for non-standard
    }

    #[test]
    fn test_unknown_framework_no_endpoint() {
        let tech = DetectedTechnology {
            name: "UnknownFramework".to_string(),
            version: None,
            category: TechnologyCategory::BackendFramework,
            confidence: 0.9,
            requires: vec![],
            conflicts_with: vec![],
            is_primary: true,
            file_indicators: vec![],
        };

        assert!(get_framework_health_endpoint(&tech).is_none());
    }
}
