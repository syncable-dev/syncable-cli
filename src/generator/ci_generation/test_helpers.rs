//! Shared test helpers for CI generation unit tests.

use std::collections::HashMap;
use std::path::Path;

use crate::analyzer::{AnalysisMetadata, ProjectAnalysis};
use crate::cli::{CiFormat, CiPlatform};
use crate::generator::ci_generation::context::{CiContext, PackageManager};

/// Constructs a minimal `CiContext` with all defaults for use in unit tests.
///
/// Fields that matter for the test under hand should be overridden by the
/// caller after construction. Using struct-update syntax is idiomatic:
///
/// ```rust
/// let ctx = make_base_ctx(dir.path(), "TypeScript");
/// let ctx = CiContext { package_manager: PackageManager::Npm, ..ctx };
/// ```
#[allow(deprecated)]
pub fn make_base_ctx(root: &Path, primary_language: &str) -> CiContext {
    CiContext {
        analysis: ProjectAnalysis {
            project_root: root.to_path_buf(),
            languages: vec![],
            technologies: vec![],
            frameworks: vec![],
            dependencies: Default::default(),
            entry_points: vec![],
            ports: vec![],
            health_endpoints: vec![],
            environment_variables: vec![],
            project_type: crate::analyzer::ProjectType::Unknown,
            build_scripts: vec![],
            services: vec![],
            architecture_type: crate::analyzer::ArchitectureType::Monolithic,
            docker_analysis: None,
            infrastructure: None,
            analysis_metadata: AnalysisMetadata {
                timestamp: String::new(),
                analyzer_version: String::new(),
                analysis_duration_ms: 0,
                files_analyzed: 0,
                confidence_score: 0.0,
            },
        },
        primary_language: primary_language.to_string(),
        runtime_versions: HashMap::new(),
        package_manager: PackageManager::Unknown,
        lock_file: None,
        test_framework: None,
        linter: None,
        build_command: None,
        has_dockerfile: false,
        monorepo: false,
        monorepo_packages: vec![],
        default_branch: "main".to_string(),
        platform: CiPlatform::Gcp,
        format: CiFormat::GithubActions,
        project_name: "test-project".to_string(),
    }
}
