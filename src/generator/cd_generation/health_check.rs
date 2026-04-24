//! CD-11 — Post-Deploy Health Check Step Generator
//!
//! Generates a GitHub Actions YAML snippet that probes the deployed application
//! via `curl` with configurable retries. The health-check URL pattern depends
//! on the deploy target:
//!
//! | Target        | URL Pattern                                                  |
//! |---------------|--------------------------------------------------------------|
//! | AppService    | `https://{{APP_NAME}}.azurewebsites.net/{{HEALTH_PATH}}`     |
//! | ContainerApps | `https://{{CONTAINER_APP_FQDN}}/{{HEALTH_PATH}}`             |
//! | CloudRun      | Uses Cloud Run service URL from previous step output         |
//! | Aks / Gke / HetznerK8s | `kubectl rollout status` (no HTTP probe)           |
//! | Vps           | `https://{{SSH_HOST}}/{{HEALTH_PATH}}`                       |
//! | Coolify       | `https://{{COOLIFY_DOMAIN}}/{{HEALTH_PATH}}`                 |

use super::context::DeployTarget;
use super::schema::HealthCheckStep;

/// Default health path when the caller doesn't provide one.
pub const DEFAULT_HEALTH_PATH: &str = "health";

/// Default retry count.
pub const DEFAULT_RETRIES: u32 = 5;

/// Default interval between retries (seconds).
pub const DEFAULT_INTERVAL_SECS: u32 = 10;

/// Default expected HTTP status code.
pub const DEFAULT_EXPECTED_STATUS: u16 = 200;

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates a `HealthCheckStep` tailored to the given deploy target.
///
/// `health_path` is the path component (without leading `/`) of the health
/// endpoint. Defaults to `"health"` when `None`.
pub fn generate_health_check(
    target: &DeployTarget,
    health_path: Option<&str>,
) -> HealthCheckStep {
    let path = health_path.unwrap_or(DEFAULT_HEALTH_PATH);
    let url = health_check_url(target, path);

    HealthCheckStep {
        url,
        retries: DEFAULT_RETRIES,
        interval_secs: DEFAULT_INTERVAL_SECS,
        expected_status: DEFAULT_EXPECTED_STATUS,
    }
}

/// Returns the probe URL template for the given target and path.
pub fn health_check_url(target: &DeployTarget, health_path: &str) -> String {
    match target {
        DeployTarget::AppService => {
            format!("https://${{{{ secrets.AZURE_APP_NAME }}}}.azurewebsites.net/{health_path}")
        }
        DeployTarget::ContainerApps => {
            format!("https://${{{{ secrets.CONTAINER_APP_FQDN }}}}/{health_path}")
        }
        DeployTarget::CloudRun => {
            // Cloud Run URL comes from the deploy step output.
            format!("${{{{ steps.deploy.outputs.url }}}}/{health_path}")
        }
        DeployTarget::Aks | DeployTarget::Gke | DeployTarget::HetznerK8s => {
            // Kubernetes targets use kubectl rollout status — no HTTP URL needed.
            "kubectl://rollout-status".to_string()
        }
        DeployTarget::Vps => {
            format!("https://${{{{ secrets.SSH_HOST }}}}/{health_path}")
        }
        DeployTarget::Coolify => {
            format!("https://${{{{ secrets.COOLIFY_DOMAIN }}}}/{health_path}")
        }
    }
}

/// Returns `true` when the target uses `kubectl rollout status` instead of
/// an HTTP health probe.
pub fn is_kubectl_health_check(target: &DeployTarget) -> bool {
    matches!(
        target,
        DeployTarget::Aks | DeployTarget::Gke | DeployTarget::HetznerK8s
    )
}

/// Renders the health-check step as a GitHub Actions YAML snippet.
///
/// For Kubernetes targets, uses `kubectl rollout status` with a timeout.
/// For all other targets, uses `curl --fail --retry`.
pub fn render_health_check_yaml(target: &DeployTarget, step: &HealthCheckStep) -> String {
    if is_kubectl_health_check(target) {
        render_kubectl_health_check_yaml(target, step)
    } else {
        render_curl_health_check_yaml(target, step)
    }
}

/// Returns the secrets referenced by the health-check step.
pub fn health_check_required_secrets(target: &DeployTarget) -> Vec<String> {
    match target {
        DeployTarget::AppService => vec!["AZURE_APP_NAME".to_string()],
        DeployTarget::ContainerApps => vec!["CONTAINER_APP_FQDN".to_string()],
        DeployTarget::CloudRun => vec![], // URL from step output, no secret
        DeployTarget::Aks | DeployTarget::Gke | DeployTarget::HetznerK8s => vec![],
        DeployTarget::Vps => vec!["SSH_HOST".to_string()],
        DeployTarget::Coolify => vec!["COOLIFY_DOMAIN".to_string()],
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn render_curl_health_check_yaml(target: &DeployTarget, step: &HealthCheckStep) -> String {
    format!(
        "\
      - name: Health check ({target})
        run: |
          curl --fail \\
            --retry {retries} \\
            --retry-delay {interval} \\
            --retry-all-errors \\
            -o /dev/null -s -w '%{{http_code}}' \\
            {url}
        env:
          EXPECTED_STATUS: '{status}'\n",
        target = target,
        retries = step.retries,
        interval = step.interval_secs,
        url = step.url,
        status = step.expected_status,
    )
}

fn render_kubectl_health_check_yaml(target: &DeployTarget, step: &HealthCheckStep) -> String {
    let timeout = step.retries * step.interval_secs;
    format!(
        "\
      - name: Health check ({target}) — rollout status
        run: |
          kubectl rollout status deployment/${{{{ secrets.K8S_DEPLOYMENT_NAME }}}} \\
            --namespace=${{{{ secrets.K8S_NAMESPACE }}}} \\
            --timeout={timeout}s\n",
        target = target,
        timeout = timeout,
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_health_check ─────────────────────────────────────────

    #[test]
    fn default_health_path_used_when_none() {
        let step = generate_health_check(&DeployTarget::AppService, None);
        assert!(step.url.contains("health"));
    }

    #[test]
    fn custom_health_path() {
        let step = generate_health_check(&DeployTarget::AppService, Some("readyz"));
        assert!(step.url.contains("readyz"));
    }

    #[test]
    fn default_retries() {
        let step = generate_health_check(&DeployTarget::CloudRun, None);
        assert_eq!(step.retries, DEFAULT_RETRIES);
    }

    #[test]
    fn default_interval() {
        let step = generate_health_check(&DeployTarget::CloudRun, None);
        assert_eq!(step.interval_secs, DEFAULT_INTERVAL_SECS);
    }

    #[test]
    fn default_expected_status() {
        let step = generate_health_check(&DeployTarget::CloudRun, None);
        assert_eq!(step.expected_status, DEFAULT_EXPECTED_STATUS);
    }

    // ── health_check_url ──────────────────────────────────────────────

    #[test]
    fn app_service_url_pattern() {
        let url = health_check_url(&DeployTarget::AppService, "health");
        assert!(url.contains("azurewebsites.net/health"));
        assert!(url.contains("AZURE_APP_NAME"));
    }

    #[test]
    fn container_apps_url_pattern() {
        let url = health_check_url(&DeployTarget::ContainerApps, "health");
        assert!(url.contains("CONTAINER_APP_FQDN"));
    }

    #[test]
    fn cloud_run_url_uses_step_output() {
        let url = health_check_url(&DeployTarget::CloudRun, "health");
        assert!(url.contains("steps.deploy.outputs.url"));
    }

    #[test]
    fn kubernetes_targets_return_kubectl_sentinel() {
        for target in &[DeployTarget::Aks, DeployTarget::Gke, DeployTarget::HetznerK8s] {
            let url = health_check_url(target, "health");
            assert_eq!(url, "kubectl://rollout-status");
        }
    }

    #[test]
    fn vps_url_pattern() {
        let url = health_check_url(&DeployTarget::Vps, "status");
        assert!(url.contains("SSH_HOST"));
        assert!(url.contains("status"));
    }

    #[test]
    fn coolify_url_pattern() {
        let url = health_check_url(&DeployTarget::Coolify, "health");
        assert!(url.contains("COOLIFY_DOMAIN"));
    }

    // ── is_kubectl_health_check ───────────────────────────────────────

    #[test]
    fn aks_is_kubectl() {
        assert!(is_kubectl_health_check(&DeployTarget::Aks));
    }

    #[test]
    fn gke_is_kubectl() {
        assert!(is_kubectl_health_check(&DeployTarget::Gke));
    }

    #[test]
    fn hetzner_k8s_is_kubectl() {
        assert!(is_kubectl_health_check(&DeployTarget::HetznerK8s));
    }

    #[test]
    fn app_service_is_not_kubectl() {
        assert!(!is_kubectl_health_check(&DeployTarget::AppService));
    }

    #[test]
    fn vps_is_not_kubectl() {
        assert!(!is_kubectl_health_check(&DeployTarget::Vps));
    }

    #[test]
    fn coolify_is_not_kubectl() {
        assert!(!is_kubectl_health_check(&DeployTarget::Coolify));
    }

    #[test]
    fn cloud_run_is_not_kubectl() {
        assert!(!is_kubectl_health_check(&DeployTarget::CloudRun));
    }

    // ── render_health_check_yaml ──────────────────────────────────────

    #[test]
    fn curl_yaml_for_app_service() {
        let step = generate_health_check(&DeployTarget::AppService, None);
        let yaml = render_health_check_yaml(&DeployTarget::AppService, &step);
        assert!(yaml.contains("curl --fail"));
        assert!(yaml.contains("--retry 5"));
    }

    #[test]
    fn curl_yaml_for_cloud_run() {
        let step = generate_health_check(&DeployTarget::CloudRun, None);
        let yaml = render_health_check_yaml(&DeployTarget::CloudRun, &step);
        assert!(yaml.contains("curl --fail"));
        assert!(yaml.contains("steps.deploy.outputs.url"));
    }

    #[test]
    fn kubectl_yaml_for_aks() {
        let step = generate_health_check(&DeployTarget::Aks, None);
        let yaml = render_health_check_yaml(&DeployTarget::Aks, &step);
        assert!(yaml.contains("kubectl rollout status"));
        assert!(yaml.contains("K8S_DEPLOYMENT_NAME"));
    }

    #[test]
    fn kubectl_yaml_timeout_calculated_from_retries() {
        let step = generate_health_check(&DeployTarget::Gke, None);
        let yaml = render_health_check_yaml(&DeployTarget::Gke, &step);
        let expected_timeout = DEFAULT_RETRIES * DEFAULT_INTERVAL_SECS;
        assert!(yaml.contains(&format!("--timeout={}s", expected_timeout)));
    }

    #[test]
    fn kubectl_yaml_references_namespace() {
        let step = generate_health_check(&DeployTarget::HetznerK8s, None);
        let yaml = render_health_check_yaml(&DeployTarget::HetznerK8s, &step);
        assert!(yaml.contains("K8S_NAMESPACE"));
    }

    #[test]
    fn vps_curl_yaml() {
        let step = generate_health_check(&DeployTarget::Vps, Some("ping"));
        let yaml = render_health_check_yaml(&DeployTarget::Vps, &step);
        assert!(yaml.contains("curl --fail"));
        assert!(yaml.contains("SSH_HOST"));
        assert!(yaml.contains("ping"));
    }

    #[test]
    fn coolify_curl_yaml() {
        let step = generate_health_check(&DeployTarget::Coolify, None);
        let yaml = render_health_check_yaml(&DeployTarget::Coolify, &step);
        assert!(yaml.contains("curl --fail"));
        assert!(yaml.contains("COOLIFY_DOMAIN"));
    }

    #[test]
    fn yaml_contains_step_name() {
        let step = generate_health_check(&DeployTarget::AppService, None);
        let yaml = render_health_check_yaml(&DeployTarget::AppService, &step);
        assert!(yaml.contains("Health check"));
    }

    #[test]
    fn curl_yaml_includes_retry_delay() {
        let step = generate_health_check(&DeployTarget::ContainerApps, None);
        let yaml = render_health_check_yaml(&DeployTarget::ContainerApps, &step);
        assert!(yaml.contains(&format!("--retry-delay {}", DEFAULT_INTERVAL_SECS)));
    }

    // ── health_check_required_secrets ─────────────────────────────────

    #[test]
    fn app_service_requires_app_name() {
        let secrets = health_check_required_secrets(&DeployTarget::AppService);
        assert!(secrets.contains(&"AZURE_APP_NAME".to_string()));
    }

    #[test]
    fn container_apps_requires_fqdn() {
        let secrets = health_check_required_secrets(&DeployTarget::ContainerApps);
        assert!(secrets.contains(&"CONTAINER_APP_FQDN".to_string()));
    }

    #[test]
    fn cloud_run_requires_no_secrets() {
        let secrets = health_check_required_secrets(&DeployTarget::CloudRun);
        assert!(secrets.is_empty());
    }

    #[test]
    fn k8s_targets_require_no_secrets() {
        for target in &[DeployTarget::Aks, DeployTarget::Gke, DeployTarget::HetznerK8s] {
            let secrets = health_check_required_secrets(target);
            assert!(secrets.is_empty(), "Unexpected secrets for {target}");
        }
    }

    #[test]
    fn vps_requires_ssh_host() {
        let secrets = health_check_required_secrets(&DeployTarget::Vps);
        assert!(secrets.contains(&"SSH_HOST".to_string()));
    }

    #[test]
    fn coolify_requires_domain() {
        let secrets = health_check_required_secrets(&DeployTarget::Coolify);
        assert!(secrets.contains(&"COOLIFY_DOMAIN".to_string()));
    }
}
