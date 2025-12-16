//! Service/Package discovery tool for monorepo exploration
//!
//! Helps the agent discover and understand the structure of monorepos.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ============================================================================
// Discover Services Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DiscoverServicesArgs {
    /// Optional subdirectory to search within
    pub path: Option<String>,
    /// Include detailed package info (dependencies, scripts)
    pub detailed: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("Discovery error: {0}")]
pub struct DiscoverServicesError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverServicesTool {
    project_path: PathBuf,
}

impl DiscoverServicesTool {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    fn should_skip_dir(name: &str) -> bool {
        matches!(
            name,
            "node_modules"
                | ".git"
                | "target"
                | "__pycache__"
                | ".venv"
                | "dist"
                | "build"
                | ".next"
                | ".nuxt"
                | "vendor"
                | ".cache"
                | "coverage"
                | "tmp"
                | "temp"
                | ".turbo"
                | ".pnpm"
        )
    }

    fn detect_package_type(path: &Path) -> Option<(&'static str, PathBuf)> {
        let indicators = [
            ("package.json", "node"),
            ("Cargo.toml", "rust"),
            ("go.mod", "go"),
            ("pyproject.toml", "python"),
            ("requirements.txt", "python"),
            ("pom.xml", "java"),
            ("build.gradle", "java"),
            ("build.gradle.kts", "kotlin"),
            ("composer.json", "php"),
            ("Gemfile", "ruby"),
            ("pubspec.yaml", "dart"),
        ];

        for (file, pkg_type) in indicators {
            let manifest = path.join(file);
            if manifest.exists() {
                return Some((pkg_type, manifest));
            }
        }
        None
    }

    fn parse_package_json(path: &Path, detailed: bool) -> Option<serde_json::Value> {
        let content = fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;

        let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let version = json.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");
        let description = json.get("description").and_then(|v| v.as_str());
        let private = json.get("private").and_then(|v| v.as_bool()).unwrap_or(false);
        
        // Detect project type from dependencies
        let deps = json.get("dependencies").and_then(|v| v.as_object());
        let dev_deps = json.get("devDependencies").and_then(|v| v.as_object());
        
        let mut project_type = "unknown";
        let mut frameworks: Vec<&str> = Vec::new();
        
        if let Some(d) = deps {
            if d.contains_key("next") {
                project_type = "Next.js App";
                frameworks.push("Next.js");
            } else if d.contains_key("react") {
                project_type = "React App";
                frameworks.push("React");
            } else if d.contains_key("vue") {
                project_type = "Vue App";
                frameworks.push("Vue");
            } else if d.contains_key("svelte") || d.contains_key("@sveltejs/kit") {
                project_type = "Svelte App";
                frameworks.push("Svelte");
            } else if d.contains_key("express") {
                project_type = "Express API";
                frameworks.push("Express");
            } else if d.contains_key("fastify") {
                project_type = "Fastify API";
                frameworks.push("Fastify");
            } else if d.contains_key("hono") {
                project_type = "Hono API";
                frameworks.push("Hono");
            } else if d.contains_key("@nestjs/core") {
                project_type = "NestJS API";
                frameworks.push("NestJS");
            }
            
            // Detect additional frameworks
            if d.contains_key("prisma") || d.contains_key("@prisma/client") {
                frameworks.push("Prisma");
            }
            if d.contains_key("drizzle-orm") {
                frameworks.push("Drizzle");
            }
            if d.contains_key("tailwindcss") {
                frameworks.push("Tailwind");
            }
            if d.contains_key("trpc") || d.contains_key("@trpc/server") {
                frameworks.push("tRPC");
            }
        }

        let mut result = json!({
            "name": name,
            "version": version,
            "type": project_type,
            "frameworks": frameworks,
            "private": private,
        });

        if let Some(desc) = description {
            result["description"] = json!(desc);
        }

        if detailed {
            // Add scripts
            if let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) {
                let script_names: Vec<&str> = scripts.keys().map(|s| s.as_str()).collect();
                result["scripts"] = json!(script_names);
            }

            // Add key dependencies count
            if let Some(d) = deps {
                result["dependencies_count"] = json!(d.len());
            }
            if let Some(d) = dev_deps {
                result["dev_dependencies_count"] = json!(d.len());
            }

            // Check for workspaces
            if let Some(workspaces) = json.get("workspaces") {
                result["workspaces"] = workspaces.clone();
            }
        }

        Some(result)
    }

    fn parse_cargo_toml(path: &Path, detailed: bool) -> Option<serde_json::Value> {
        let content = fs::read_to_string(path).ok()?;
        let toml: toml::Value = toml::from_str(&content).ok()?;

        let package = toml.get("package")?;
        let name = package.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let version = package.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");
        let description = package.get("description").and_then(|v| v.as_str());

        // Detect project type
        let project_type = if path.parent().map(|p| p.join("src/main.rs").exists()).unwrap_or(false) {
            "binary"
        } else if path.parent().map(|p| p.join("src/lib.rs").exists()).unwrap_or(false) {
            "library"
        } else {
            "unknown"
        };

        let mut frameworks: Vec<&str> = Vec::new();
        
        // Check dependencies for frameworks
        if let Some(deps) = toml.get("dependencies").and_then(|v| v.as_table()) {
            if deps.contains_key("actix-web") {
                frameworks.push("Actix-web");
            }
            if deps.contains_key("axum") {
                frameworks.push("Axum");
            }
            if deps.contains_key("rocket") {
                frameworks.push("Rocket");
            }
            if deps.contains_key("tokio") {
                frameworks.push("Tokio");
            }
            if deps.contains_key("sqlx") {
                frameworks.push("SQLx");
            }
            if deps.contains_key("diesel") {
                frameworks.push("Diesel");
            }
        }

        let mut result = json!({
            "name": name,
            "version": version,
            "type": project_type,
            "frameworks": frameworks,
        });

        if let Some(desc) = description {
            result["description"] = json!(desc);
        }

        if detailed {
            // Check for workspace members
            if let Some(workspace) = toml.get("workspace") {
                if let Some(members) = workspace.get("members").and_then(|v| v.as_array()) {
                    let member_strs: Vec<&str> = members
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect();
                    result["workspace_members"] = json!(member_strs);
                }
            }

            // Count dependencies
            if let Some(deps) = toml.get("dependencies").and_then(|v| v.as_table()) {
                result["dependencies_count"] = json!(deps.len());
            }
        }

        Some(result)
    }

    fn parse_go_mod(path: &Path, _detailed: bool) -> Option<serde_json::Value> {
        let content = fs::read_to_string(path).ok()?;
        
        // Extract module name from first line
        let module_name = content
            .lines()
            .find(|l| l.starts_with("module "))
            .map(|l| l.trim_start_matches("module ").trim())
            .unwrap_or("unknown");

        // Extract Go version
        let go_version = content
            .lines()
            .find(|l| l.starts_with("go "))
            .map(|l| l.trim_start_matches("go ").trim());

        let mut result = json!({
            "name": module_name,
            "type": "go module",
        });

        if let Some(v) = go_version {
            result["go_version"] = json!(v);
        }

        Some(result)
    }
}

#[derive(Debug, Serialize)]
struct ServiceInfo {
    name: String,
    path: String,
    package_type: String,
    info: serde_json::Value,
}

impl Tool for DiscoverServicesTool {
    const NAME: &'static str = "discover_services";

    type Error = DiscoverServicesError;
    type Args = DiscoverServicesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Discover all services, packages, and projects in a monorepo. 
Returns a list of all packages with their names, types, frameworks, and locations.
Use this FIRST when exploring a monorepo to understand its structure.
Then use analyze_project with specific paths to deep-dive into individual services."#.to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Subdirectory to search within (e.g., 'apps', 'packages', 'services')"
                    },
                    "detailed": {
                        "type": "boolean",
                        "description": "Include detailed info like scripts, workspace config. Default: true"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let search_root = if let Some(ref subpath) = args.path {
            self.project_path.join(subpath)
        } else {
            self.project_path.clone()
        };

        if !search_root.exists() {
            return Err(DiscoverServicesError(format!(
                "Path does not exist: {}",
                args.path.unwrap_or_default()
            )));
        }

        let detailed = args.detailed.unwrap_or(true);
        let mut services: Vec<ServiceInfo> = Vec::new();
        let mut workspace_roots: HashMap<String, serde_json::Value> = HashMap::new();

        // First check root for workspace config
        if let Some((pkg_type, manifest_path)) = Self::detect_package_type(&search_root) {
            let info = match pkg_type {
                "node" => Self::parse_package_json(&manifest_path, true),
                "rust" => Self::parse_cargo_toml(&manifest_path, true),
                "go" => Self::parse_go_mod(&manifest_path, detailed),
                _ => None,
            };

            if let Some(info) = info {
                // Check if this is a workspace root
                if info.get("workspaces").is_some() || info.get("workspace_members").is_some() {
                    workspace_roots.insert("root".to_string(), info);
                }
            }
        }

        // Walk the directory tree
        for entry in WalkDir::new(&search_root)
            .max_depth(6)  // Deep enough for nested monorepos
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        return !Self::should_skip_dir(name);
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Skip the root - we already checked it
            if path == search_root {
                continue;
            }

            if let Some((pkg_type, manifest_path)) = Self::detect_package_type(path) {
                let info = match pkg_type {
                    "node" => Self::parse_package_json(&manifest_path, detailed),
                    "rust" => Self::parse_cargo_toml(&manifest_path, detailed),
                    "go" => Self::parse_go_mod(&manifest_path, detailed),
                    _ => Some(json!({"type": pkg_type})),
                };

                if let Some(info) = info {
                    // Skip template placeholders
                    if let Some(name) = info.get("name").and_then(|v| v.as_str()) {
                        if name.contains("${") || name.contains("{{") {
                            continue;
                        }
                    }

                    let relative_path = path
                        .strip_prefix(&self.project_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();

                    let name = info
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                        })
                        .to_string();

                    services.push(ServiceInfo {
                        name,
                        path: relative_path,
                        package_type: pkg_type.to_string(),
                        info,
                    });
                }
            }
        }

        // Sort by path for consistent output
        services.sort_by(|a, b| a.path.cmp(&b.path));

        // Categorize services
        let mut categorized: HashMap<&str, Vec<&ServiceInfo>> = HashMap::new();
        for service in &services {
            let category = if service.path.starts_with("apps/") || service.path.starts_with("packages/apps/") {
                "apps"
            } else if service.path.starts_with("packages/") || service.path.starts_with("libs/") {
                "packages"
            } else if service.path.starts_with("services/") {
                "services"
            } else if service.path.starts_with("tools/") {
                "tools"
            } else {
                "other"
            };
            categorized.entry(category).or_default().push(service);
        }

        let result = json!({
            "total_services": services.len(),
            "categorized": {
                "apps": categorized.get("apps").map(|v| v.len()).unwrap_or(0),
                "packages": categorized.get("packages").map(|v| v.len()).unwrap_or(0),
                "services": categorized.get("services").map(|v| v.len()).unwrap_or(0),
                "tools": categorized.get("tools").map(|v| v.len()).unwrap_or(0),
                "other": categorized.get("other").map(|v| v.len()).unwrap_or(0),
            },
            "workspace_config": workspace_roots,
            "services": services,
            "tip": "Use analyze_project with path='<service_path>' to get detailed analysis of each service"
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| DiscoverServicesError(format!("Serialization error: {}", e)))
    }
}
