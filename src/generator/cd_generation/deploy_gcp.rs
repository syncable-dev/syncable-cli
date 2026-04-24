//! CD-08 — GCP Deploy Step Generator
//!
//! Generates GitHub Actions YAML snippets for GCP deployment targets:
//!
//! | Target     | Action / Command                                  | Key params              |
//! |------------|--------------------------------------------------|-------------------------|
//! | Cloud Run  | `google-github-actions/deploy-cloudrun@v2`        | `service`, `image`      |
//! | GKE        | `google-github-actions/get-gke-credentials@v2`    | `cluster_name`, kubectl |
//!
//! Each function returns a `DeployStep` for the schema and a YAML snippet
//! string for direct embedding. Rollback hints are also provided per target.

use super::context::DeployTarget;
use super::schema::{DeployStep, RollbackInfo};

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates the deploy step for the given GCP target.
pub fn generate_gcp_deploy(target: &DeployTarget, image_tag: &str) -> DeployStep {
    match target {
        DeployTarget::CloudRun => DeployStep {
            strategy: "rolling".to_string(),
            command: "google-github-actions/deploy-cloudrun@v2".to_string(),
            args: vec![
                "service={{CLOUD_RUN_SERVICE}}".to_string(),
                format!("image={image_tag}"),
                "region={{GCP_REGION}}".to_string(),
            ],
            target: target.clone(),
        },
        DeployTarget::Gke => DeployStep {
            strategy: "rolling".to_string(),
            command: "kubectl set image".to_string(),
            args: vec![
                "deployment/{{DEPLOYMENT_NAME}}".to_string(),
                format!("app={image_tag}"),
                "--namespace={{K8S_NAMESPACE}}".to_string(),
            ],
            target: target.clone(),
        },
        other => DeployStep {
            strategy: "rolling".to_string(),
            command: format!("echo 'Unsupported GCP target: {other}'"),
            args: vec![],
            target: other.clone(),
        },
    }
}

/// Generates rollback info for the given GCP target.
pub fn gcp_rollback_info(target: &DeployTarget) -> RollbackInfo {
    match target {
        DeployTarget::CloudRun => RollbackInfo {
            strategy: "traffic-shift".to_string(),
            command_hint: "gcloud run services update-traffic {{CLOUD_RUN_SERVICE}} --region={{GCP_REGION}} --to-revisions=LATEST=0,<previous>=100".to_string(),
        },
        DeployTarget::Gke => RollbackInfo {
            strategy: "rollout-undo".to_string(),
            command_hint: "kubectl rollout undo deployment/{{DEPLOYMENT_NAME}} -n {{K8S_NAMESPACE}}".to_string(),
        },
        _ => RollbackInfo {
            strategy: "manual".to_string(),
            command_hint: "Manually redeploy the previous version".to_string(),
        },
    }
}

/// Renders the Cloud Run deploy step as a GitHub Actions YAML snippet.
pub fn render_cloud_run_deploy_yaml(image_tag: &str) -> String {
    format!(
        "\
      - name: Deploy to Cloud Run
        uses: google-github-actions/deploy-cloudrun@v2
        with:
          service: ${{{{ secrets.CLOUD_RUN_SERVICE }}}}
          image: {image_tag}
          region: ${{{{ secrets.GCP_REGION }}}}\n"
    )
}

/// Renders the GKE deploy steps as a GitHub Actions YAML snippet.
///
/// Emits two steps: get GKE credentials, then kubectl set image.
pub fn render_gke_deploy_yaml(image_tag: &str) -> String {
    format!(
        "\
      - name: Get GKE credentials
        uses: google-github-actions/get-gke-credentials@v2
        with:
          cluster_name: ${{{{ secrets.GKE_CLUSTER_NAME }}}}
          location: ${{{{ secrets.GCP_REGION }}}}

      - name: Deploy to GKE
        run: |
          kubectl set image deployment/${{{{ secrets.DEPLOYMENT_NAME }}}} \\
            app={image_tag} \\
            --namespace=${{{{ secrets.K8S_NAMESPACE }}}}
          kubectl rollout status deployment/${{{{ secrets.DEPLOYMENT_NAME }}}} \\
            --namespace=${{{{ secrets.K8S_NAMESPACE }}}} \\
            --timeout=300s\n"
    )
}

/// Renders the deploy YAML snippet for any GCP target.
pub fn render_gcp_deploy_yaml(target: &DeployTarget, image_tag: &str) -> String {
    match target {
        DeployTarget::CloudRun => render_cloud_run_deploy_yaml(image_tag),
        DeployTarget::Gke => render_gke_deploy_yaml(image_tag),
        _ => format!("      - name: Deploy\n        run: echo 'Unsupported GCP target'\n"),
    }
}

/// Returns secrets required for the GCP deploy target.
pub fn gcp_deploy_required_secrets(target: &DeployTarget) -> Vec<String> {
    match target {
        DeployTarget::CloudRun => vec![
            "CLOUD_RUN_SERVICE".to_string(),
            "GCP_REGION".to_string(),
        ],
        DeployTarget::Gke => vec![
            "GKE_CLUSTER_NAME".to_string(),
            "GCP_REGION".to_string(),
            "DEPLOYMENT_NAME".to_string(),
            "K8S_NAMESPACE".to_string(),
        ],
        _ => vec![],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: &str = "us-central1-docker.pkg.dev/proj/repo/app:sha123";

    // ── generate_gcp_deploy ───────────────────────────────────────────

    #[test]
    fn cloud_run_deploy_step_uses_correct_action() {
        let step = generate_gcp_deploy(&DeployTarget::CloudRun, IMAGE);
        assert_eq!(step.command, "google-github-actions/deploy-cloudrun@v2");
    }

    #[test]
    fn cloud_run_deploy_step_strategy_is_rolling() {
        let step = generate_gcp_deploy(&DeployTarget::CloudRun, IMAGE);
        assert_eq!(step.strategy, "rolling");
    }

    #[test]
    fn cloud_run_deploy_step_contains_service_placeholder() {
        let step = generate_gcp_deploy(&DeployTarget::CloudRun, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{CLOUD_RUN_SERVICE}}")));
    }

    #[test]
    fn cloud_run_deploy_step_contains_region_placeholder() {
        let step = generate_gcp_deploy(&DeployTarget::CloudRun, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{GCP_REGION}}")));
    }

    #[test]
    fn cloud_run_deploy_step_contains_image() {
        let step = generate_gcp_deploy(&DeployTarget::CloudRun, IMAGE);
        assert!(step.args.iter().any(|a| a.contains(IMAGE)));
    }

    #[test]
    fn gke_deploy_step_uses_kubectl() {
        let step = generate_gcp_deploy(&DeployTarget::Gke, IMAGE);
        assert!(step.command.contains("kubectl"));
    }

    #[test]
    fn gke_deploy_step_contains_namespace_placeholder() {
        let step = generate_gcp_deploy(&DeployTarget::Gke, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{K8S_NAMESPACE}}")));
    }

    #[test]
    fn gke_deploy_step_contains_deployment_name_placeholder() {
        let step = generate_gcp_deploy(&DeployTarget::Gke, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{DEPLOYMENT_NAME}}")));
    }

    #[test]
    fn gke_deploy_step_target_preserved() {
        let step = generate_gcp_deploy(&DeployTarget::Gke, IMAGE);
        assert_eq!(step.target, DeployTarget::Gke);
    }

    // ── gcp_rollback_info ─────────────────────────────────────────────

    #[test]
    fn cloud_run_rollback_uses_traffic_shift() {
        let info = gcp_rollback_info(&DeployTarget::CloudRun);
        assert_eq!(info.strategy, "traffic-shift");
    }

    #[test]
    fn cloud_run_rollback_mentions_update_traffic() {
        let info = gcp_rollback_info(&DeployTarget::CloudRun);
        assert!(info.command_hint.contains("update-traffic"));
    }

    #[test]
    fn gke_rollback_uses_rollout_undo() {
        let info = gcp_rollback_info(&DeployTarget::Gke);
        assert_eq!(info.strategy, "rollout-undo");
        assert!(info.command_hint.contains("rollout undo"));
    }

    // ── render_gcp_deploy_yaml ────────────────────────────────────────

    #[test]
    fn cloud_run_yaml_contains_action() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::CloudRun, IMAGE);
        assert!(yaml.contains("google-github-actions/deploy-cloudrun@v2"));
    }

    #[test]
    fn cloud_run_yaml_contains_image() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::CloudRun, IMAGE);
        assert!(yaml.contains(IMAGE));
    }

    #[test]
    fn cloud_run_yaml_references_service_secret() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::CloudRun, IMAGE);
        assert!(yaml.contains("secrets.CLOUD_RUN_SERVICE"));
    }

    #[test]
    fn cloud_run_yaml_references_region_secret() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::CloudRun, IMAGE);
        assert!(yaml.contains("secrets.GCP_REGION"));
    }

    #[test]
    fn gke_yaml_contains_get_credentials_action() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::Gke, IMAGE);
        assert!(yaml.contains("google-github-actions/get-gke-credentials@v2"));
    }

    #[test]
    fn gke_yaml_contains_kubectl_set_image() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::Gke, IMAGE);
        assert!(yaml.contains("kubectl set image"));
    }

    #[test]
    fn gke_yaml_contains_rollout_status_wait() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::Gke, IMAGE);
        assert!(yaml.contains("kubectl rollout status"));
        assert!(yaml.contains("timeout=300s"));
    }

    #[test]
    fn gke_yaml_references_cluster_name() {
        let yaml = render_gcp_deploy_yaml(&DeployTarget::Gke, IMAGE);
        assert!(yaml.contains("secrets.GKE_CLUSTER_NAME"));
    }

    // ── gcp_deploy_required_secrets ───────────────────────────────────

    #[test]
    fn cloud_run_requires_service_and_region() {
        let secrets = gcp_deploy_required_secrets(&DeployTarget::CloudRun);
        assert!(secrets.contains(&"CLOUD_RUN_SERVICE".to_string()));
        assert!(secrets.contains(&"GCP_REGION".to_string()));
    }

    #[test]
    fn gke_requires_four_secrets() {
        let secrets = gcp_deploy_required_secrets(&DeployTarget::Gke);
        assert_eq!(secrets.len(), 4);
        assert!(secrets.contains(&"GKE_CLUSTER_NAME".to_string()));
    }
}
