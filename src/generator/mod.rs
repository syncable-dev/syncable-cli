use crate::analyzer::ProjectAnalysis;
use crate::error::Result;

pub mod compose_gen;
pub mod dockerfile_gen;
pub mod templates;
pub mod terraform_gen;

/// Generate a Dockerfile based on project analysis
pub fn generate_dockerfile(analysis: &ProjectAnalysis) -> Result<String> {
    dockerfile_gen::generate(analysis)
}

/// Generate a Docker Compose file based on project analysis
pub fn generate_compose(analysis: &ProjectAnalysis) -> Result<String> {
    compose_gen::generate(analysis)
}

/// Generate Terraform configuration based on project analysis
pub fn generate_terraform(analysis: &ProjectAnalysis) -> Result<String> {
    terraform_gen::generate(analysis)
} 