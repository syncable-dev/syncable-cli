//! Deployment recommendation engine
//!
//! Generates intelligent deployment recommendations based on project analysis.
//! Takes analyzer output and produces actionable suggestions with reasoning.

use crate::analyzer::{PortSource, ProjectAnalysis, TechnologyCategory};
use crate::platform::api::types::{CloudProvider, DeploymentTarget};
use crate::wizard::cloud_provider_data::{
    get_default_machine_type, get_default_region, get_machine_types_for_provider,
    get_regions_for_provider,
};
use serde::{Deserialize, Serialize};

/// A deployment recommendation with reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecommendation {
    /// Recommended cloud provider
    pub provider: CloudProvider,
    /// Why this provider was recommended
    pub provider_reasoning: String,

    /// Recommended deployment target
    pub target: DeploymentTarget,
    /// Why this target was recommended
    pub target_reasoning: String,

    /// Recommended machine type (provider-specific)
    pub machine_type: String,
    /// Why this machine type was recommended
    pub machine_reasoning: String,

    /// Recommended region
    pub region: String,
    /// Why this region was recommended
    pub region_reasoning: String,

    /// Detected port to expose
    pub port: u16,
    /// Where the port was detected from
    pub port_source: String,

    /// Recommended health check path (if detected)
    pub health_check_path: Option<String>,

    /// Overall confidence in recommendation (0.0-1.0)
    pub confidence: f32,

    /// Alternative recommendations if user wants to customize
    pub alternatives: RecommendationAlternatives,
}

/// Alternative options for customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationAlternatives {
    pub providers: Vec<ProviderOption>,
    pub machine_types: Vec<MachineOption>,
    pub regions: Vec<RegionOption>,
}

/// Provider option with availability info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOption {
    pub provider: CloudProvider,
    pub available: bool,
    pub reason_if_unavailable: Option<String>,
}

/// Machine type option with specs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineOption {
    pub machine_type: String,
    pub vcpu: String,
    pub memory_gb: String,
    pub description: String,
}

/// Region option with display name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionOption {
    pub region: String,
    pub display_name: String,
}

/// Input for generating recommendations
#[derive(Debug, Clone)]
pub struct RecommendationInput {
    pub analysis: ProjectAnalysis,
    pub available_providers: Vec<CloudProvider>,
    pub has_existing_k8s: bool,
    pub user_region_hint: Option<String>,
}

/// Generate deployment recommendation based on project analysis
pub fn recommend_deployment(input: RecommendationInput) -> DeploymentRecommendation {
    // 1. Select provider
    let (provider, provider_reasoning) = select_provider(&input);

    // 2. Select target (K8s vs Cloud Runner)
    let (target, target_reasoning) = select_target(&input);

    // 3. Select machine type based on detected framework
    let (machine_type, machine_reasoning) = select_machine_type(&input.analysis, &provider);

    // 4. Select region
    let (region, region_reasoning) = select_region(&provider, input.user_region_hint.as_deref());

    // 5. Select port
    let (port, port_source) = select_port(&input.analysis);

    // 6. Select health check path
    let health_check_path = select_health_endpoint(&input.analysis);

    // 7. Calculate confidence
    let confidence = calculate_confidence(&input.analysis, &port_source, health_check_path.is_some());

    // 8. Build alternatives
    let alternatives = build_alternatives(&provider, &input.available_providers);

    DeploymentRecommendation {
        provider,
        provider_reasoning,
        target,
        target_reasoning,
        machine_type,
        machine_reasoning,
        region,
        region_reasoning,
        port,
        port_source,
        health_check_path,
        confidence,
        alternatives,
    }
}

/// Select the best provider based on available options and project characteristics
fn select_provider(input: &RecommendationInput) -> (CloudProvider, String) {
    // Check if infrastructure suggests a specific provider
    if let Some(ref infra) = input.analysis.infrastructure {
        // If they have existing K8s clusters, prefer the provider they're already using
        if infra.has_kubernetes || input.has_existing_k8s {
            // For now, default to Hetzner for K8s unless GCP clusters detected
            if input.available_providers.contains(&CloudProvider::Gcp) {
                return (
                    CloudProvider::Gcp,
                    "GCP recommended: Existing Kubernetes infrastructure detected".to_string(),
                );
            }
        }
    }

    // Check which providers are available
    let has_hetzner = input.available_providers.contains(&CloudProvider::Hetzner);
    let has_gcp = input.available_providers.contains(&CloudProvider::Gcp);

    if has_hetzner && has_gcp {
        // Both available - prefer Hetzner for cost-effectiveness
        (
            CloudProvider::Hetzner,
            "Hetzner recommended: Cost-effective for web services, European data centers".to_string(),
        )
    } else if has_hetzner {
        (
            CloudProvider::Hetzner,
            "Hetzner selected: Only available connected provider".to_string(),
        )
    } else if has_gcp {
        (
            CloudProvider::Gcp,
            "GCP selected: Only available connected provider".to_string(),
        )
    } else {
        // Fallback - shouldn't happen in practice
        (
            CloudProvider::Hetzner,
            "Hetzner selected: Default provider".to_string(),
        )
    }
}

/// Select deployment target based on existing infrastructure
fn select_target(input: &RecommendationInput) -> (DeploymentTarget, String) {
    // Check for existing Kubernetes infrastructure
    if let Some(ref infra) = input.analysis.infrastructure {
        if infra.has_kubernetes && input.has_existing_k8s {
            return (
                DeploymentTarget::Kubernetes,
                "Kubernetes recommended: Existing K8s manifests detected and clusters available".to_string(),
            );
        }
    }

    // Default to Cloud Runner for simplicity
    (
        DeploymentTarget::CloudRunner,
        "Cloud Runner recommended: Simpler deployment, no cluster management required".to_string(),
    )
}

/// Select machine type based on detected framework characteristics
fn select_machine_type(analysis: &ProjectAnalysis, provider: &CloudProvider) -> (String, String) {
    // Detect framework type to determine resource needs
    let framework_info = get_framework_resource_hint(analysis);

    let (machine_type, reasoning) = match provider {
        CloudProvider::Hetzner => {
            match framework_info.memory_requirement {
                MemoryRequirement::Low => (
                    "cx23".to_string(),
                    format!("cx23 (2 vCPU, 4GB) recommended: {} services are memory-efficient", framework_info.name),
                ),
                MemoryRequirement::Medium => (
                    "cx33".to_string(),
                    format!("cx33 (4 vCPU, 8GB) recommended: {} may benefit from more resources", framework_info.name),
                ),
                MemoryRequirement::High => (
                    "cx43".to_string(),
                    format!("cx43 (8 vCPU, 16GB) recommended: {} requires significant memory (JVM, ML, etc.)", framework_info.name),
                ),
            }
        }
        CloudProvider::Gcp => {
            match framework_info.memory_requirement {
                MemoryRequirement::Low => (
                    "e2-small".to_string(),
                    format!("e2-small (0.5 vCPU, 2GB) recommended: {} services are lightweight", framework_info.name),
                ),
                MemoryRequirement::Medium => (
                    "e2-medium".to_string(),
                    format!("e2-medium (1 vCPU, 4GB) recommended: {} may need moderate resources", framework_info.name),
                ),
                MemoryRequirement::High => (
                    "e2-standard-2".to_string(),
                    format!("e2-standard-2 (2 vCPU, 8GB) recommended: {} requires significant memory", framework_info.name),
                ),
            }
        }
        _ => {
            // Fallback for unsupported providers
            (
                get_default_machine_type(provider).to_string(),
                "Default machine type selected".to_string(),
            )
        }
    };

    (machine_type, reasoning)
}

/// Memory requirement categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoryRequirement {
    Low,    // Node.js, Go, Rust - efficient runtimes
    Medium, // Python, Ruby - moderate memory
    High,   // Java/JVM, ML frameworks - memory intensive
}

/// Framework resource hint for machine selection
struct FrameworkResourceHint {
    name: String,
    memory_requirement: MemoryRequirement,
}

/// Analyze project to determine framework resource requirements
fn get_framework_resource_hint(analysis: &ProjectAnalysis) -> FrameworkResourceHint {
    // Check for JVM-based frameworks (high memory)
    for tech in &analysis.technologies {
        if matches!(tech.category, TechnologyCategory::BackendFramework) {
            let name_lower = tech.name.to_lowercase();

            // JVM frameworks - high memory
            if name_lower.contains("spring") || name_lower.contains("quarkus")
                || name_lower.contains("micronaut") || name_lower.contains("ktor") {
                return FrameworkResourceHint {
                    name: tech.name.clone(),
                    memory_requirement: MemoryRequirement::High,
                };
            }

            // Go, Rust frameworks - low memory
            if name_lower.contains("gin") || name_lower.contains("echo")
                || name_lower.contains("fiber") || name_lower.contains("chi")
                || name_lower.contains("actix") || name_lower.contains("axum")
                || name_lower.contains("rocket") {
                return FrameworkResourceHint {
                    name: tech.name.clone(),
                    memory_requirement: MemoryRequirement::Low,
                };
            }

            // Node.js frameworks - low memory
            if name_lower.contains("express") || name_lower.contains("fastify")
                || name_lower.contains("koa") || name_lower.contains("hono")
                || name_lower.contains("elysia") || name_lower.contains("nest") {
                return FrameworkResourceHint {
                    name: tech.name.clone(),
                    memory_requirement: MemoryRequirement::Low,
                };
            }

            // Python frameworks - medium memory
            if name_lower.contains("fastapi") || name_lower.contains("flask")
                || name_lower.contains("django") {
                return FrameworkResourceHint {
                    name: tech.name.clone(),
                    memory_requirement: MemoryRequirement::Medium,
                };
            }
        }
    }

    // Check languages if no framework detected
    for lang in &analysis.languages {
        let name_lower = lang.name.to_lowercase();

        if name_lower.contains("java") || name_lower.contains("kotlin") || name_lower.contains("scala") {
            return FrameworkResourceHint {
                name: lang.name.clone(),
                memory_requirement: MemoryRequirement::High,
            };
        }

        if name_lower.contains("go") || name_lower.contains("rust") {
            return FrameworkResourceHint {
                name: lang.name.clone(),
                memory_requirement: MemoryRequirement::Low,
            };
        }

        if name_lower.contains("javascript") || name_lower.contains("typescript") {
            return FrameworkResourceHint {
                name: lang.name.clone(),
                memory_requirement: MemoryRequirement::Low,
            };
        }

        if name_lower.contains("python") {
            return FrameworkResourceHint {
                name: lang.name.clone(),
                memory_requirement: MemoryRequirement::Medium,
            };
        }
    }

    // Default fallback
    FrameworkResourceHint {
        name: "Unknown".to_string(),
        memory_requirement: MemoryRequirement::Medium,
    }
}

/// Select region based on user hint or defaults
fn select_region(provider: &CloudProvider, user_hint: Option<&str>) -> (String, String) {
    if let Some(hint) = user_hint {
        // Validate hint is a valid region for this provider
        let regions = get_regions_for_provider(provider);
        if regions.iter().any(|r| r.id == hint) {
            return (
                hint.to_string(),
                format!("{} selected: User preference", hint),
            );
        }
    }

    let default_region = get_default_region(provider);
    let reasoning = match provider {
        CloudProvider::Hetzner => format!("{} (Nuremberg) selected: Default EU region, low latency for European users", default_region),
        CloudProvider::Gcp => format!("{} (Iowa) selected: Default US region, good general-purpose choice", default_region),
        _ => format!("{} selected: Default region for provider", default_region),
    };

    (default_region.to_string(), reasoning)
}

/// Select the best port from analysis results
fn select_port(analysis: &ProjectAnalysis) -> (u16, String) {
    // Priority: SourceCode > PackageJson > ConfigFile > FrameworkDefault > Dockerfile > DockerCompose > EnvVar
    let port_priority = |source: &Option<PortSource>| -> u8 {
        match source {
            Some(PortSource::SourceCode) => 7,
            Some(PortSource::PackageJson) => 6,
            Some(PortSource::ConfigFile) => 5,
            Some(PortSource::FrameworkDefault) => 4,
            Some(PortSource::Dockerfile) => 3,
            Some(PortSource::DockerCompose) => 2,
            Some(PortSource::EnvVar) => 1,
            None => 0,
        }
    };

    // Find the highest priority port
    let best_port = analysis.ports.iter()
        .max_by_key(|p| port_priority(&p.source));

    if let Some(port) = best_port {
        let source_desc = match &port.source {
            Some(PortSource::SourceCode) => "Detected from source code analysis",
            Some(PortSource::PackageJson) => "Detected from package.json scripts",
            Some(PortSource::ConfigFile) => "Detected from configuration file",
            Some(PortSource::FrameworkDefault) => {
                // Try to get framework name
                let framework_name = analysis.technologies.iter()
                    .find(|t| matches!(t.category, TechnologyCategory::BackendFramework | TechnologyCategory::MetaFramework))
                    .map(|t| t.name.as_str())
                    .unwrap_or("framework");
                return (port.number, format!("Framework default ({}: {})", framework_name, port.number));
            }
            Some(PortSource::Dockerfile) => "Detected from Dockerfile EXPOSE",
            Some(PortSource::DockerCompose) => "Detected from docker-compose.yml",
            Some(PortSource::EnvVar) => "Detected from environment variable reference",
            None => "Detected from project analysis",
        };
        return (port.number, source_desc.to_string());
    }

    // Fallback to 8080
    (8080, "Default port 8080: No port detected in project".to_string())
}

/// Select the best health endpoint from analysis
fn select_health_endpoint(analysis: &ProjectAnalysis) -> Option<String> {
    // Find highest confidence health endpoint
    analysis.health_endpoints.iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
        .map(|e| e.path.clone())
}

/// Calculate overall confidence in the recommendation
fn calculate_confidence(analysis: &ProjectAnalysis, port_source: &str, has_health_endpoint: bool) -> f32 {
    let mut confidence: f32 = 0.5; // Base confidence

    // Boost for detected port from reliable source
    if port_source.contains("source code") || port_source.contains("package.json") {
        confidence += 0.2;
    } else if port_source.contains("Dockerfile") || port_source.contains("framework") {
        confidence += 0.1;
    }

    // Boost for detected framework
    let has_framework = analysis.technologies.iter()
        .any(|t| matches!(t.category, TechnologyCategory::BackendFramework | TechnologyCategory::MetaFramework));
    if has_framework {
        confidence += 0.15;
    }

    // Boost for health endpoint
    if has_health_endpoint {
        confidence += 0.1;
    }

    // Penalty if using fallback port
    if port_source.contains("No port detected") || port_source.contains("Default port") {
        confidence -= 0.2;
    }

    confidence.clamp(0.0, 1.0)
}

/// Build alternative options for user customization
fn build_alternatives(selected_provider: &CloudProvider, available_providers: &[CloudProvider]) -> RecommendationAlternatives {
    // Build provider options
    let providers: Vec<ProviderOption> = CloudProvider::all()
        .iter()
        .map(|p| ProviderOption {
            provider: p.clone(),
            available: available_providers.contains(p) && p.is_available(),
            reason_if_unavailable: if !p.is_available() {
                Some(format!("{} coming soon", p.display_name()))
            } else if !available_providers.contains(p) {
                Some("Not connected".to_string())
            } else {
                None
            },
        })
        .collect();

    // Build machine type options for selected provider
    let machine_types: Vec<MachineOption> = get_machine_types_for_provider(selected_provider)
        .iter()
        .map(|m| MachineOption {
            machine_type: m.id.to_string(),
            vcpu: m.cpu.to_string(),
            memory_gb: m.memory.to_string(),
            description: m.description.map(String::from).unwrap_or_default(),
        })
        .collect();

    // Build region options for selected provider
    let regions: Vec<RegionOption> = get_regions_for_provider(selected_provider)
        .iter()
        .map(|r| RegionOption {
            region: r.id.to_string(),
            display_name: format!("{} ({})", r.name, r.location),
        })
        .collect();

    RecommendationAlternatives {
        providers,
        machine_types,
        regions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{
        AnalysisMetadata, ArchitectureType, DetectedLanguage, DetectedTechnology,
        HealthEndpoint, InfrastructurePresence, Port, ProjectType, TechnologyCategory,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_minimal_analysis() -> ProjectAnalysis {
        #[allow(deprecated)]
        ProjectAnalysis {
            project_root: PathBuf::from("/test"),
            languages: vec![],
            technologies: vec![],
            frameworks: vec![],
            dependencies: HashMap::new(),
            entry_points: vec![],
            ports: vec![],
            health_endpoints: vec![],
            environment_variables: vec![],
            project_type: ProjectType::WebApplication,
            build_scripts: vec![],
            services: vec![],
            architecture_type: ArchitectureType::Monolithic,
            docker_analysis: None,
            infrastructure: None,
            analysis_metadata: AnalysisMetadata {
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                analyzer_version: "0.1.0".to_string(),
                analysis_duration_ms: 100,
                files_analyzed: 10,
                confidence_score: 0.8,
            },
        }
    }

    #[test]
    fn test_nodejs_express_recommendation() {
        let mut analysis = create_minimal_analysis();
        analysis.languages.push(DetectedLanguage {
            name: "JavaScript".to_string(),
            version: Some("18".to_string()),
            confidence: 0.9,
            files: vec![],
            main_dependencies: vec!["express".to_string()],
            dev_dependencies: vec![],
            package_manager: Some("npm".to_string()),
        });
        analysis.technologies.push(DetectedTechnology {
            name: "Express".to_string(),
            version: Some("4.18".to_string()),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.9,
            requires: vec![],
            conflicts_with: vec![],
            is_primary: true,
            file_indicators: vec![],
        });
        analysis.ports.push(Port {
            number: 3000,
            protocol: crate::analyzer::Protocol::Http,
            description: Some("Express default".to_string()),
            source: Some(PortSource::PackageJson),
        });

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner, CloudProvider::Gcp],
            has_existing_k8s: false,
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);

        // Express should get a small machine
        assert!(rec.machine_type == "cx23" || rec.machine_type == "e2-small");
        assert_eq!(rec.port, 3000);
        assert!(rec.machine_reasoning.contains("Express"));
    }

    #[test]
    fn test_java_spring_recommendation() {
        let mut analysis = create_minimal_analysis();
        analysis.languages.push(DetectedLanguage {
            name: "Java".to_string(),
            version: Some("17".to_string()),
            confidence: 0.9,
            files: vec![],
            main_dependencies: vec!["spring-boot".to_string()],
            dev_dependencies: vec![],
            package_manager: Some("maven".to_string()),
        });
        analysis.technologies.push(DetectedTechnology {
            name: "Spring Boot".to_string(),
            version: Some("3.0".to_string()),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.9,
            requires: vec![],
            conflicts_with: vec![],
            is_primary: true,
            file_indicators: vec![],
        });
        analysis.ports.push(Port {
            number: 8080,
            protocol: crate::analyzer::Protocol::Http,
            description: Some("Spring Boot default".to_string()),
            source: Some(PortSource::FrameworkDefault),
        });

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner],
            has_existing_k8s: false,
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);

        // Spring Boot should get a larger machine (JVM needs memory)
        assert!(rec.machine_type == "cx43" || rec.machine_reasoning.contains("memory"));
        assert_eq!(rec.port, 8080);
    }

    #[test]
    fn test_existing_k8s_suggests_kubernetes_target() {
        let mut analysis = create_minimal_analysis();
        analysis.infrastructure = Some(InfrastructurePresence {
            has_kubernetes: true,
            kubernetes_paths: vec![PathBuf::from("k8s/")],
            has_helm: false,
            helm_chart_paths: vec![],
            has_docker_compose: false,
            has_terraform: false,
            terraform_paths: vec![],
            has_deployment_config: false,
            summary: Some("Kubernetes manifests detected".to_string()),
        });

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Gcp],
            has_existing_k8s: true, // User has K8s clusters
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);
        assert_eq!(rec.target, DeploymentTarget::Kubernetes);
        assert!(rec.target_reasoning.contains("Kubernetes"));
    }

    #[test]
    fn test_no_k8s_defaults_to_cloud_runner() {
        let analysis = create_minimal_analysis();

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner],
            has_existing_k8s: false,
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);
        assert_eq!(rec.target, DeploymentTarget::CloudRunner);
        assert!(rec.target_reasoning.contains("Cloud Runner"));
    }

    #[test]
    fn test_port_fallback_to_8080() {
        let analysis = create_minimal_analysis();

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner],
            has_existing_k8s: false,
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);
        assert_eq!(rec.port, 8080);
        assert!(rec.port_source.contains("No port detected") || rec.port_source.contains("Default"));
    }

    #[test]
    fn test_health_endpoint_included_when_detected() {
        let mut analysis = create_minimal_analysis();
        analysis.health_endpoints.push(HealthEndpoint {
            path: "/health".to_string(),
            confidence: 0.9,
            source: crate::analyzer::HealthEndpointSource::CodePattern,
            description: Some("Found in source code".to_string()),
        });

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner],
            has_existing_k8s: false,
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);
        assert_eq!(rec.health_check_path, Some("/health".to_string()));
    }

    #[test]
    fn test_alternatives_populated() {
        let analysis = create_minimal_analysis();

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner, CloudProvider::Gcp],
            has_existing_k8s: false,
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);

        assert!(!rec.alternatives.providers.is_empty());
        assert!(!rec.alternatives.machine_types.is_empty());
        assert!(!rec.alternatives.regions.is_empty());
    }

    #[test]
    fn test_user_region_hint_respected() {
        let analysis = create_minimal_analysis();

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner],
            has_existing_k8s: false,
            user_region_hint: Some("fsn1".to_string()),
        };

        let rec = recommend_deployment(input);
        assert_eq!(rec.region, "fsn1");
        assert!(rec.region_reasoning.contains("User preference"));
    }

    #[test]
    fn test_go_service_gets_small_machine() {
        let mut analysis = create_minimal_analysis();
        analysis.technologies.push(DetectedTechnology {
            name: "Gin".to_string(),
            version: Some("1.9".to_string()),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.9,
            requires: vec![],
            conflicts_with: vec![],
            is_primary: true,
            file_indicators: vec![],
        });

        let input = RecommendationInput {
            analysis,
            available_providers: vec![CloudProvider::Hetzner],
            has_existing_k8s: false,
            user_region_hint: None,
        };

        let rec = recommend_deployment(input);
        // Go services should get small machine
        assert_eq!(rec.machine_type, "cx23");
        assert!(rec.machine_reasoning.contains("memory-efficient") || rec.machine_reasoning.contains("Gin"));
    }
}
