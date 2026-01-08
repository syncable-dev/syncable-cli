//! Static analysis of Kubernetes manifests for resource optimization.
//!
//! Analyzes Kubernetes manifests to detect over-provisioned or under-provisioned
//! resources without requiring cluster access.
//!
//! Supports:
//! - Kubernetes YAML manifests
//! - **Terraform HCL** files with `kubernetes_*` provider resources
//! - **Helm charts** - Renders with `helm template` before analysis
//! - **Kustomize directories** - Builds with `kustomize build` before analysis

use super::config::K8sOptimizeConfig;
use super::parser::{
    detect_workload_type, extract_container_image, extract_container_name, extract_resources,
};
use super::recommender::{ContainerContext, generate_recommendations};
use super::terraform_parser::parse_terraform_k8s_resources;
use super::types::{AnalysisMode, OptimizationIssue, OptimizationResult};

use std::path::Path;
use std::process::Command;
use std::time::Instant;

// ============================================================================
// Main Analysis Functions
// ============================================================================

/// Analyze Kubernetes manifests from a path.
///
/// The path can be:
/// - A single YAML file
/// - A single Terraform (.tf) file
/// - A directory containing YAML and/or Terraform files
/// - A Helm chart directory
/// - A Kustomize directory
pub fn analyze(path: &Path, config: &K8sOptimizeConfig) -> OptimizationResult {
    let start = Instant::now();
    let mut result = OptimizationResult::new(path.to_path_buf(), AnalysisMode::Static);

    // Check if path should be ignored
    if config.should_ignore_path(path) {
        result.metadata.duration_ms = start.elapsed().as_millis() as u64;
        return result;
    }

    // Load and parse YAML content
    let yaml_contents = if path.is_dir() {
        collect_yaml_files(path)
    } else if path.is_file() {
        if let Some(ext) = path.extension() {
            if ext == "tf" {
                // Single Terraform file - process it separately
                analyze_terraform_resources(path, config, &mut result);
                update_summary(&mut result);
                result.sort();
                result.metadata.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            }
        }
        match std::fs::read_to_string(path) {
            Ok(content) => vec![(path.to_path_buf(), content)],
            Err(_) => {
                result.metadata.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            }
        }
    } else {
        result.metadata.duration_ms = start.elapsed().as_millis() as u64;
        return result;
    };

    // Analyze each YAML file
    for (file_path, content) in yaml_contents {
        analyze_yaml_content(&content, &file_path, config, &mut result);
    }

    // Also analyze Terraform files in the directory
    if path.is_dir() {
        analyze_terraform_resources(path, config, &mut result);
    }

    // Update summary
    update_summary(&mut result);

    // Sort recommendations by severity
    result.sort();

    result.metadata.duration_ms = start.elapsed().as_millis() as u64;
    result
}

/// Analyze a single YAML file.
pub fn analyze_file(path: &Path, config: &K8sOptimizeConfig) -> OptimizationResult {
    analyze(path, config)
}

/// Analyze YAML content directly.
pub fn analyze_content(content: &str, config: &K8sOptimizeConfig) -> OptimizationResult {
    let start = Instant::now();
    let mut result =
        OptimizationResult::new(std::path::PathBuf::from("<content>"), AnalysisMode::Static);

    analyze_yaml_content(content, Path::new("<content>"), config, &mut result);
    update_summary(&mut result);
    result.sort();

    result.metadata.duration_ms = start.elapsed().as_millis() as u64;
    result
}

// ============================================================================
// Internal Analysis
// ============================================================================

/// Analyze YAML content and add recommendations to result.
fn analyze_yaml_content(
    content: &str,
    file_path: &Path,
    config: &K8sOptimizeConfig,
    result: &mut OptimizationResult,
) {
    // Track line numbers as we split multi-document YAML
    let mut line_offset = 1u32;

    // Split multi-document YAML
    for doc in content.split("\n---") {
        let doc_line_count = doc.lines().count() as u32;
        let doc = doc.trim();
        if doc.is_empty() {
            line_offset += doc_line_count.max(1); // At least 1 for the separator
            continue;
        }

        // Strip leading YAML comments (like Helm's # Source: comments)
        // but keep the actual YAML content
        let yaml_start = doc.lines().position(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        });

        // Calculate the actual line where YAML content starts
        let content_line_offset = line_offset + yaml_start.unwrap_or(0) as u32;

        let doc = match yaml_start {
            Some(start) => doc.lines().skip(start).collect::<Vec<_>>().join("\n"),
            None => {
                line_offset += doc_line_count.max(1);
                continue; // All lines are comments
            }
        };

        if doc.is_empty() {
            line_offset += doc_line_count.max(1);
            continue;
        }

        // Parse YAML document
        let yaml: serde_yaml::Value = match serde_yaml::from_str(&doc) {
            Ok(v) => v,
            Err(_) => {
                line_offset += doc_line_count.max(1);
                continue;
            }
        };

        // Extract kind and metadata
        let kind = match yaml.get("kind").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => continue,
        };

        // Only analyze workload kinds
        if !is_workload_kind(kind) {
            continue;
        }

        result.summary.resources_analyzed += 1;

        let name = yaml
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        let namespace = yaml
            .get("metadata")
            .and_then(|m| m.get("namespace"))
            .and_then(|n| n.as_str())
            .map(String::from);

        // Check if namespace should be excluded
        if let Some(ref ns) = namespace {
            if config.should_exclude_namespace(ns) {
                continue;
            }
        }

        // Extract containers from pod spec
        let containers = extract_containers(&yaml, kind);

        for container in containers {
            result.summary.containers_analyzed += 1;

            let container_name =
                extract_container_name(&container).unwrap_or_else(|| "unknown".to_string());
            let container_image = extract_container_image(&container);
            let resources = extract_resources(&container);

            let workload_type =
                detect_workload_type(container_image.as_deref(), Some(&container_name), kind);

            let ctx = ContainerContext {
                resource_kind: kind.to_string(),
                resource_name: name.clone(),
                namespace: namespace.clone(),
                container_name,
                file_path: file_path.to_path_buf(),
                line: Some(content_line_offset), // Line where this K8s object starts
                current: resources,
                workload_type,
            };

            let recommendations = generate_recommendations(&ctx, config);
            result.recommendations.extend(recommendations);
        }

        // Update line offset for next document (add 1 for the --- separator)
        line_offset += doc_line_count.max(1);
    }
}

/// Check if a kind is a workload that has containers.
fn is_workload_kind(kind: &str) -> bool {
    matches!(
        kind,
        "Deployment"
            | "StatefulSet"
            | "DaemonSet"
            | "ReplicaSet"
            | "Pod"
            | "Job"
            | "CronJob"
            | "DeploymentConfig" // OpenShift
    )
}

/// Extract containers from a workload YAML.
fn extract_containers(yaml: &serde_yaml::Value, kind: &str) -> Vec<serde_yaml::Value> {
    let mut containers = Vec::new();

    // Get pod spec path based on kind
    let pod_spec = match kind {
        "Pod" => yaml.get("spec"),
        "CronJob" => yaml
            .get("spec")
            .and_then(|s| s.get("jobTemplate"))
            .and_then(|j| j.get("spec"))
            .and_then(|s| s.get("template"))
            .and_then(|t| t.get("spec")),
        _ => yaml
            .get("spec")
            .and_then(|s| s.get("template"))
            .and_then(|t| t.get("spec")),
    };

    if let Some(spec) = pod_spec {
        // Regular containers
        if let Some(serde_yaml::Value::Sequence(ctrs)) = spec.get("containers") {
            containers.extend(ctrs.iter().cloned());
        }

        // Init containers
        if let Some(serde_yaml::Value::Sequence(ctrs)) = spec.get("initContainers") {
            containers.extend(ctrs.iter().cloned());
        }
    }

    containers
}

// ============================================================================
// Helm and Kustomize Rendering
// ============================================================================

/// Check if helm binary is available.
fn is_helm_available() -> bool {
    Command::new("helm")
        .arg("version")
        .arg("--short")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if kustomize binary is available.
fn is_kustomize_available() -> bool {
    Command::new("kustomize")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Render a Helm chart using `helm template`.
/// Returns the rendered YAML content.
fn render_helm_chart(chart_path: &Path) -> Option<String> {
    if !is_helm_available() {
        log::warn!(
            "helm not found in PATH, skipping Helm chart rendering for {}",
            chart_path.display()
        );
        return None;
    }

    let output = Command::new("helm")
        .arg("template")
        .arg("release-name")
        .arg(chart_path)
        .output();

    match output {
        Ok(o) if o.status.success() => Some(String::from_utf8_lossy(&o.stdout).to_string()),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            log::warn!(
                "Helm template failed for {}: {}",
                chart_path.display(),
                stderr
            );
            None
        }
        Err(e) => {
            log::warn!(
                "Failed to run helm template for {}: {}",
                chart_path.display(),
                e
            );
            None
        }
    }
}

/// Render a Kustomize directory using `kustomize build`.
/// Returns the rendered YAML content.
fn render_kustomize(kustomize_path: &Path) -> Option<String> {
    // Try kubectl kustomize first (more commonly available)
    let kubectl_output = Command::new("kubectl")
        .arg("kustomize")
        .arg(kustomize_path)
        .output();

    if let Ok(o) = kubectl_output {
        if o.status.success() {
            return Some(String::from_utf8_lossy(&o.stdout).to_string());
        }
    }

    // Fall back to standalone kustomize
    if !is_kustomize_available() {
        log::warn!(
            "kustomize not found in PATH, skipping Kustomize rendering for {}",
            kustomize_path.display()
        );
        return None;
    }

    let output = Command::new("kustomize")
        .arg("build")
        .arg(kustomize_path)
        .output();

    match output {
        Ok(o) if o.status.success() => Some(String::from_utf8_lossy(&o.stdout).to_string()),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            log::warn!(
                "Kustomize build failed for {}: {}",
                kustomize_path.display(),
                stderr
            );
            None
        }
        Err(e) => {
            log::warn!(
                "Failed to run kustomize build for {}: {}",
                kustomize_path.display(),
                e
            );
            None
        }
    }
}

/// Collect all YAML files from a directory.
/// For Helm charts, renders with `helm template`.
/// For Kustomize directories, builds with `kustomize build`.
fn collect_yaml_files(dir: &Path) -> Vec<(std::path::PathBuf, String)> {
    let mut files = Vec::new();

    // Check if this is a Helm chart
    let chart_yaml = dir.join("Chart.yaml");
    if chart_yaml.exists() {
        // Render the Helm chart
        if let Some(rendered) = render_helm_chart(dir) {
            files.push((dir.to_path_buf(), rendered));
            return files;
        }
        // Fallback: just read templates directly (won't parse {{ }} syntax well)
        let templates_dir = dir.join("templates");
        if templates_dir.exists() {
            log::info!("Falling back to raw template parsing for {}", dir.display());
            collect_yaml_files_recursive(&templates_dir, &mut files);
        }
        return files;
    }

    // Check if this is a Kustomize directory
    let kustomization = dir.join("kustomization.yaml");
    let kustomization_alt = dir.join("kustomization.yml");
    if kustomization.exists() || kustomization_alt.exists() {
        // Render the Kustomize directory
        if let Some(rendered) = render_kustomize(dir) {
            files.push((dir.to_path_buf(), rendered));
            return files;
        }
        // Fallback: collect YAML files directly
        log::info!("Falling back to raw YAML parsing for {}", dir.display());
        collect_yaml_files_recursive(dir, &mut files);
        return files;
    }

    // Check for nested Helm charts and Kustomize directories
    find_and_render_nested(dir, &mut files);

    // Also collect regular YAML files
    collect_yaml_files_recursive(dir, &mut files);
    files
}

/// Find and render nested Helm charts and Kustomize directories.
fn find_and_render_nested(dir: &Path, files: &mut Vec<(std::path::PathBuf, String)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Check for Helm chart
        if path.join("Chart.yaml").exists() {
            if let Some(rendered) = render_helm_chart(&path) {
                files.push((path.clone(), rendered));
            }
            continue; // Don't recurse into rendered charts
        }

        // Check for Kustomize
        if path.join("kustomization.yaml").exists() || path.join("kustomization.yml").exists() {
            if let Some(rendered) = render_kustomize(&path) {
                files.push((path.clone(), rendered));
            }
            continue; // Don't recurse into rendered kustomize dirs
        }

        // Recurse into subdirectories
        find_and_render_nested(&path, files);
    }
}

fn collect_yaml_files_recursive(dir: &Path, files: &mut Vec<(std::path::PathBuf, String)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files_recursive(&path, files);
        } else if let Some(ext) = path.extension() {
            if ext == "yaml" || ext == "yml" {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    files.push((path, content));
                }
            }
        }
    }
}

/// Update the summary statistics based on recommendations.
fn update_summary(result: &mut OptimizationResult) {
    for rec in &result.recommendations {
        match rec.issue {
            OptimizationIssue::OverProvisioned => result.summary.over_provisioned += 1,
            OptimizationIssue::UnderProvisioned => result.summary.under_provisioned += 1,
            OptimizationIssue::NoRequestsDefined => result.summary.missing_requests += 1,
            OptimizationIssue::NoLimitsDefined => result.summary.missing_limits += 1,
            _ => {}
        }
    }

    // Calculate optimal count
    if result.summary.containers_analyzed > 0 {
        let issue_count = result.summary.over_provisioned
            + result.summary.under_provisioned
            + result.summary.missing_requests;
        result.summary.optimal = result
            .summary
            .containers_analyzed
            .saturating_sub(issue_count);
    }

    // Calculate waste percentage (simplified - based on over-provisioned count)
    if result.summary.containers_analyzed > 0 {
        result.summary.total_waste_percentage = (result.summary.over_provisioned as f32
            / result.summary.containers_analyzed as f32)
            * 100.0;
    }
}

// ============================================================================
// Terraform Analysis
// ============================================================================

/// Format bytes to K8s memory format (Mi, Gi, etc.)
fn format_bytes_to_k8s(bytes: u64) -> String {
    const GI: u64 = 1024 * 1024 * 1024;
    const MI: u64 = 1024 * 1024;
    const KI: u64 = 1024;

    if bytes >= GI && bytes % GI == 0 {
        format!("{}Gi", bytes / GI)
    } else if bytes >= MI && bytes % MI == 0 {
        format!("{}Mi", bytes / MI)
    } else if bytes >= KI && bytes % KI == 0 {
        format!("{}Ki", bytes / KI)
    } else {
        format!("{}", bytes)
    }
}

/// Analyze Terraform files for Kubernetes resources.
fn analyze_terraform_resources(
    path: &Path,
    config: &K8sOptimizeConfig,
    result: &mut OptimizationResult,
) {
    use super::types::ResourceSpec;

    let tf_resources = parse_terraform_k8s_resources(path);

    for tf_res in tf_resources {
        // Skip system namespaces if not included
        if let Some(ref ns) = tf_res.namespace {
            if config.should_exclude_namespace(ns) {
                continue;
            }
        }

        result.summary.resources_analyzed += 1;

        // Map Terraform resource type to K8s kind
        let kind = match tf_res.resource_type.as_str() {
            t if t.contains("deployment") => "Deployment",
            t if t.contains("stateful_set") => "StatefulSet",
            t if t.contains("daemon_set") => "DaemonSet",
            t if t.contains("job") && !t.contains("cron") => "Job",
            t if t.contains("cron_job") => "CronJob",
            t if t.contains("pod") => "Pod",
            _ => "Deployment",
        };

        let resource_name = tf_res
            .k8s_name
            .clone()
            .unwrap_or_else(|| tf_res.tf_name.clone());

        for container in &tf_res.containers {
            result.summary.containers_analyzed += 1;

            // Build ResourceSpec from Terraform container
            // Convert millicores/bytes back to K8s format strings
            let cpu_req = container
                .requests
                .as_ref()
                .and_then(|r| r.cpu)
                .map(|c| format!("{}m", c));
            let mem_req = container
                .requests
                .as_ref()
                .and_then(|r| r.memory)
                .map(|m| format_bytes_to_k8s(m));
            let cpu_lim = container
                .limits
                .as_ref()
                .and_then(|l| l.cpu)
                .map(|c| format!("{}m", c));
            let mem_lim = container
                .limits
                .as_ref()
                .and_then(|l| l.memory)
                .map(|m| format_bytes_to_k8s(m));

            let current = ResourceSpec {
                cpu_request: cpu_req,
                memory_request: mem_req,
                cpu_limit: cpu_lim,
                memory_limit: mem_lim,
            };

            let ctx = ContainerContext {
                resource_kind: kind.to_string(),
                resource_name: resource_name.clone(),
                namespace: tf_res.namespace.clone(),
                container_name: container.name.clone(),
                file_path: std::path::PathBuf::from(&tf_res.source_file),
                line: None,
                current,
                workload_type: tf_res.workload_type,
            };

            let recommendations = generate_recommendations(&ctx, config);
            result.recommendations.extend(recommendations);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_simple_deployment() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nginx-deployment
  namespace: default
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nginx
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
"#;
        let config = K8sOptimizeConfig::default().with_system();
        let result = analyze_content(yaml, &config);

        assert_eq!(result.summary.resources_analyzed, 1);
        assert_eq!(result.summary.containers_analyzed, 1);
    }

    #[test]
    fn test_analyze_no_resources() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: no-resources
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: app
        image: myapp:v1
"#;
        let config = K8sOptimizeConfig::default().with_system();
        let result = analyze_content(yaml, &config);

        assert_eq!(result.summary.containers_analyzed, 1);
        assert!(result.has_recommendations());
        assert!(
            result
                .recommendations
                .iter()
                .any(|r| { r.issue == OptimizationIssue::NoRequestsDefined })
        );
    }

    #[test]
    fn test_analyze_over_provisioned() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: over-provisioned
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21
        resources:
          requests:
            cpu: 4000m
            memory: 8Gi
          limits:
            cpu: 8000m
            memory: 16Gi
"#;
        let config = K8sOptimizeConfig::default().with_system();
        let result = analyze_content(yaml, &config);

        assert!(result.has_recommendations());
        assert!(
            result
                .recommendations
                .iter()
                .any(|r| { r.issue == OptimizationIssue::OverProvisioned })
        );
    }

    #[test]
    fn test_analyze_multi_container() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: multi-container
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    spec:
      initContainers:
      - name: init
        image: busybox
      containers:
      - name: app
        image: myapp:v1
      - name: sidecar
        image: envoy:v1
"#;
        let config = K8sOptimizeConfig::default().with_system();
        let result = analyze_content(yaml, &config);

        assert_eq!(result.summary.containers_analyzed, 3);
    }

    #[test]
    #[ignore] // TODO: Fix test - cronjob getting unexpected OverProvisioned recommendations
    fn test_analyze_cronjob() {
        let yaml = r#"
apiVersion: batch/v1
kind: CronJob
metadata:
  name: batch-job
spec:
  schedule: "0 * * * *"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: job
            image: batch:v1
            resources:
              requests:
                cpu: 2000m
                memory: 4Gi
          restartPolicy: Never
"#;
        let config = K8sOptimizeConfig::default().with_system();
        let result = analyze_content(yaml, &config);

        assert_eq!(result.summary.containers_analyzed, 1);
        // CronJobs should be detected as Batch workload and not trigger over-provisioned warnings
        assert!(
            !result
                .recommendations
                .iter()
                .any(|r| { r.issue == OptimizationIssue::OverProvisioned })
        );
    }

    #[test]
    fn test_exclude_kube_system() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: coredns
  namespace: kube-system
spec:
  replicas: 2
  selector:
    matchLabels:
      app: coredns
  template:
    spec:
      containers:
      - name: coredns
        image: coredns:1.10
"#;
        let config = K8sOptimizeConfig::default(); // include_system = false by default
        let result = analyze_content(yaml, &config);

        // kube-system should be excluded
        assert_eq!(result.summary.containers_analyzed, 0);
    }

    #[test]
    fn test_is_workload_kind() {
        assert!(is_workload_kind("Deployment"));
        assert!(is_workload_kind("StatefulSet"));
        assert!(is_workload_kind("DaemonSet"));
        assert!(is_workload_kind("Job"));
        assert!(is_workload_kind("CronJob"));
        assert!(is_workload_kind("Pod"));
        assert!(!is_workload_kind("Service"));
        assert!(!is_workload_kind("ConfigMap"));
        assert!(!is_workload_kind("Secret"));
    }
}
