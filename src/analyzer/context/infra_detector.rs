//! Infrastructure detection for deployment recommendations.
//!
//! Detects existing infrastructure configurations:
//! - Kubernetes manifests (k8s/, deploy/, manifests/)
//! - Helm charts (Chart.yaml)
//! - Terraform files (*.tf)
//! - Docker Compose files
//! - Syncable deployment configs (.syncable/)

use crate::analyzer::InfrastructurePresence;
use crate::common::file_utils::is_readable_file;
use std::path::{Path, PathBuf};

/// Common directories where K8s manifests might be found
const K8S_DIRECTORIES: &[&str] = &[
    "k8s",
    "kubernetes",
    "deploy",
    "deployment",
    "deployments",
    "manifests",
    "kube",
    "charts",
    ".k8s",
];

/// Docker compose file variants
const COMPOSE_FILES: &[&str] = &[
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
    "docker-compose.dev.yml",
    "docker-compose.prod.yml",
    "docker-compose.local.yml",
];

/// Detect infrastructure presence in a project
pub fn detect_infrastructure(project_root: &Path) -> InfrastructurePresence {
    let mut infra = InfrastructurePresence::default();

    // Detect Docker Compose
    for compose_file in COMPOSE_FILES {
        if is_readable_file(&project_root.join(compose_file)) {
            infra.has_docker_compose = true;
            break;
        }
    }

    // Detect Kubernetes manifests
    let k8s_paths = detect_kubernetes_manifests(project_root);
    if !k8s_paths.is_empty() {
        infra.has_kubernetes = true;
        infra.kubernetes_paths = k8s_paths;
    }

    // Detect Helm charts
    let helm_paths = detect_helm_charts(project_root);
    if !helm_paths.is_empty() {
        infra.has_helm = true;
        infra.helm_chart_paths = helm_paths;
    }

    // Detect Terraform
    let tf_paths = detect_terraform(project_root);
    if !tf_paths.is_empty() {
        infra.has_terraform = true;
        infra.terraform_paths = tf_paths;
    }

    // Detect Syncable deployment config
    infra.has_deployment_config = project_root.join(".syncable").is_dir()
        || is_readable_file(&project_root.join("syncable.json"))
        || is_readable_file(&project_root.join("syncable.yaml"))
        || is_readable_file(&project_root.join("syncable.yml"));

    // Generate summary
    if infra.has_any() {
        let types = infra.detected_types();
        infra.summary = Some(format!("Detected: {}", types.join(", ")));
    }

    infra
}

/// Detect Kubernetes manifest directories and files
fn detect_kubernetes_manifests(project_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Check common K8s directories
    for dir_name in K8S_DIRECTORIES {
        let dir_path = project_root.join(dir_name);
        if dir_path.is_dir() && has_kubernetes_files(&dir_path) {
            paths.push(dir_path);
        }
    }

    // Check root-level YAML files that might be K8s manifests
    if let Ok(entries) = std::fs::read_dir(project_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if (ext == "yaml" || ext == "yml") && is_kubernetes_manifest(&path) {
                        paths.push(path);
                    }
                }
            }
        }
    }

    paths
}

/// Check if a directory contains Kubernetes files
fn has_kubernetes_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if (ext == "yaml" || ext == "yml") && is_kubernetes_manifest(&path) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if a YAML file is a Kubernetes manifest (quick check without full parsing)
fn is_kubernetes_manifest(path: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(path) {
        // Check first 2KB of file for K8s markers (fast check)
        let check_content = if content.len() > 2048 {
            &content[..2048]
        } else {
            &content
        };

        // K8s manifest indicators
        let k8s_kinds = [
            "kind: Deployment",
            "kind: Service",
            "kind: Pod",
            "kind: ConfigMap",
            "kind: Secret",
            "kind: Ingress",
            "kind: StatefulSet",
            "kind: DaemonSet",
            "kind: Job",
            "kind: CronJob",
            "kind: PersistentVolumeClaim",
            "kind: ServiceAccount",
            "kind: Role",
            "kind: RoleBinding",
            "kind: ClusterRole",
            "kind: ClusterRoleBinding",
            "kind: NetworkPolicy",
            "kind: HorizontalPodAutoscaler",
            "kind: PodDisruptionBudget",
            "kind: Namespace",
        ];

        // Check for apiVersion + kind pattern (most K8s manifests)
        if check_content.contains("apiVersion:") {
            for kind in &k8s_kinds {
                if check_content.contains(*kind) {
                    return true;
                }
            }
        }
    }
    false
}

/// Detect Helm chart directories
fn detect_helm_charts(project_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Check if root is a Helm chart
    if is_readable_file(&project_root.join("Chart.yaml")) {
        paths.push(project_root.to_path_buf());
    }

    // Check common locations
    let helm_locations = ["charts", "helm", "deploy/helm", "deployment/helm"];
    for location in &helm_locations {
        let dir = project_root.join(location);
        if dir.is_dir() {
            // Check if it's a chart itself
            if is_readable_file(&dir.join("Chart.yaml")) {
                paths.push(dir.clone());
            }
            // Check subdirectories for charts
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && is_readable_file(&path.join("Chart.yaml")) {
                        paths.push(path);
                    }
                }
            }
        }
    }

    paths
}

/// Detect Terraform directories
fn detect_terraform(project_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Check common Terraform locations
    let tf_locations = ["terraform", "infra", "infrastructure", "tf", "iac"];
    for location in &tf_locations {
        let dir = project_root.join(location);
        if dir.is_dir() && has_terraform_files(&dir) {
            paths.push(dir);
        }
    }

    // Check root for Terraform files
    if has_terraform_files(project_root) {
        paths.push(project_root.to_path_buf());
    }

    paths
}

/// Check if a directory contains Terraform files
fn has_terraform_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "tf" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_empty_project() {
        let temp_dir = TempDir::new().unwrap();
        let infra = detect_infrastructure(temp_dir.path());
        assert!(!infra.has_any());
    }

    #[test]
    fn test_detect_docker_compose() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("docker-compose.yml"),
            "version: '3'\nservices:\n  app:\n    build: .",
        )
        .unwrap();

        let infra = detect_infrastructure(temp_dir.path());
        assert!(infra.has_docker_compose);
        assert!(infra.has_any());
    }

    #[test]
    fn test_detect_kubernetes_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let k8s_dir = temp_dir.path().join("k8s");
        fs::create_dir(&k8s_dir).unwrap();
        fs::write(
            k8s_dir.join("deployment.yaml"),
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: test",
        )
        .unwrap();

        let infra = detect_infrastructure(temp_dir.path());
        assert!(infra.has_kubernetes);
        assert_eq!(infra.kubernetes_paths.len(), 1);
    }

    #[test]
    fn test_detect_helm_chart() {
        let temp_dir = TempDir::new().unwrap();
        let helm_dir = temp_dir.path().join("charts").join("myapp");
        fs::create_dir_all(&helm_dir).unwrap();
        fs::write(
            helm_dir.join("Chart.yaml"),
            "apiVersion: v2\nname: myapp\nversion: 1.0.0",
        )
        .unwrap();

        let infra = detect_infrastructure(temp_dir.path());
        assert!(infra.has_helm);
        assert!(!infra.helm_chart_paths.is_empty());
    }

    #[test]
    fn test_detect_terraform() {
        let temp_dir = TempDir::new().unwrap();
        let tf_dir = temp_dir.path().join("terraform");
        fs::create_dir(&tf_dir).unwrap();
        fs::write(
            tf_dir.join("main.tf"),
            "provider \"aws\" {\n  region = \"us-east-1\"\n}",
        )
        .unwrap();

        let infra = detect_infrastructure(temp_dir.path());
        assert!(infra.has_terraform);
        assert!(!infra.terraform_paths.is_empty());
    }

    #[test]
    fn test_detect_syncable_config() {
        let temp_dir = TempDir::new().unwrap();
        let syncable_dir = temp_dir.path().join(".syncable");
        fs::create_dir(&syncable_dir).unwrap();

        let infra = detect_infrastructure(temp_dir.path());
        assert!(infra.has_deployment_config);
    }

    #[test]
    fn test_infrastructure_summary() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("docker-compose.yml"), "version: '3'").unwrap();
        let tf_dir = temp_dir.path().join("terraform");
        fs::create_dir(&tf_dir).unwrap();
        fs::write(tf_dir.join("main.tf"), "provider \"aws\" {}").unwrap();

        let infra = detect_infrastructure(temp_dir.path());
        assert!(infra.has_docker_compose);
        assert!(infra.has_terraform);
        assert!(infra.summary.is_some());
        let summary = infra.summary.unwrap();
        assert!(summary.contains("Docker Compose"));
        assert!(summary.contains("Terraform"));
    }
}
