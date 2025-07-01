use crate::analyzer::{
    AnalysisConfig, BuildScript, DetectedLanguage, DetectedTechnology, EntryPoint, EnvVar, Port,
    ProjectType,
};
use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::file_analyzers::{docker, env, makefile};
use super::language_analyzers::{go, javascript, jvm, python, rust};
use super::microservices;
use super::project_type;
use super::tech_specific;

/// Project context information
pub struct ProjectContext {
    pub entry_points: Vec<EntryPoint>,
    pub ports: Vec<Port>,
    pub environment_variables: Vec<EnvVar>,
    pub project_type: ProjectType,
    pub build_scripts: Vec<BuildScript>,
}

/// Analyzes project context including entry points, ports, and environment variables
pub fn analyze_context(
    project_root: &Path,
    languages: &[DetectedLanguage],
    technologies: &[DetectedTechnology],
    config: &AnalysisConfig,
) -> Result<ProjectContext> {
    log::info!("Analyzing project context");

    let mut entry_points = Vec::new();
    let mut ports = HashSet::new();
    let mut env_vars = HashMap::new();
    let mut build_scripts = Vec::new();

    // Analyze based on detected languages
    for language in languages {
        match language.name.as_str() {
            "JavaScript" | "TypeScript" => {
                javascript::analyze_node_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Python" => {
                python::analyze_python_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Rust" => {
                rust::analyze_rust_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Go" => {
                go::analyze_go_project(project_root, &mut entry_points, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            "Java" | "Kotlin" => {
                jvm::analyze_jvm_project(project_root, &mut ports, &mut env_vars, &mut build_scripts, config)?;
            }
            _ => {}
        }
    }

    // Analyze common configuration files
    docker::analyze_docker_files(project_root, &mut ports, &mut env_vars)?;
    env::analyze_env_files(project_root, &mut env_vars)?;
    makefile::analyze_makefile(project_root, &mut build_scripts)?;

    // Technology-specific analysis
    for technology in technologies {
        tech_specific::analyze_technology_specifics(technology, project_root, &mut entry_points, &mut ports)?;
    }

    // Detect microservices structure
    let microservices = microservices::detect_microservices_structure(project_root)?;

    // Determine project type
    let ports_vec: Vec<Port> = ports.iter().cloned().collect();
    let project_type = project_type::determine_project_type_with_structure(
        languages,
        technologies,
        &entry_points,
        &ports_vec,
        &microservices,
    );

    // Convert collections to vectors
    let ports: Vec<Port> = ports.into_iter().collect();
    let environment_variables: Vec<EnvVar> = env_vars
        .into_iter()
        .map(|(name, (default, required, desc))| EnvVar {
            name,
            default_value: default,
            required,
            description: desc,
        })
        .collect();

    Ok(ProjectContext {
        entry_points,
        ports,
        environment_variables,
        project_type,
        build_scripts,
    })
} 