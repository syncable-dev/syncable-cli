//! CD-09 — Hetzner Deploy Step Generator
//!
//! Generates GitHub Actions YAML snippets for Hetzner deployment targets:
//!
//! | Target       | Method                        | Steps                              |
//! |-------------|-------------------------------|-------------------------------------|
//! | VPS          | SSH + Docker Compose          | `ssh` → `docker pull` → `up -d`   |
//! | HetznerK8s   | kubectl via kubeconfig        | `kubectl set image`                |
//! | Coolify      | Coolify API webhook           | `curl` POST to webhook URL         |
//!
//! VPS deployments use SSH to pull the latest image and restart services
//! on the remote host via `docker compose`.

use super::context::DeployTarget;
use super::schema::{DeployStep, RollbackInfo};

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates the deploy step for the given Hetzner target.
pub fn generate_hetzner_deploy(target: &DeployTarget, image_tag: &str) -> DeployStep {
    match target {
        DeployTarget::Vps => DeployStep {
            strategy: "recreate".to_string(),
            command: "ssh".to_string(),
            args: vec![
                "${{ secrets.SSH_USER }}@${{ secrets.SSH_HOST }}".to_string(),
                format!("'docker pull {image_tag} && docker compose up -d'"),
            ],
            target: target.clone(),
        },
        DeployTarget::HetznerK8s => DeployStep {
            strategy: "rolling".to_string(),
            command: "kubectl set image".to_string(),
            args: vec![
                "deployment/{{DEPLOYMENT_NAME}}".to_string(),
                format!("app={image_tag}"),
                "--namespace={{K8S_NAMESPACE}}".to_string(),
            ],
            target: target.clone(),
        },
        DeployTarget::Coolify => DeployStep {
            strategy: "rolling".to_string(),
            command: "curl".to_string(),
            args: vec![
                "-fsSL".to_string(),
                "-X POST".to_string(),
                "${{ secrets.COOLIFY_WEBHOOK }}".to_string(),
            ],
            target: target.clone(),
        },
        other => DeployStep {
            strategy: "recreate".to_string(),
            command: format!("echo 'Unsupported Hetzner target: {other}'"),
            args: vec![],
            target: other.clone(),
        },
    }
}

/// Generates rollback info for the given Hetzner target.
pub fn hetzner_rollback_info(target: &DeployTarget) -> RollbackInfo {
    match target {
        DeployTarget::Vps => RollbackInfo {
            strategy: "manual".to_string(),
            command_hint: "ssh $SSH_USER@$SSH_HOST 'docker compose down && docker pull <previous-image> && docker compose up -d'".to_string(),
        },
        DeployTarget::HetznerK8s => RollbackInfo {
            strategy: "rollout-undo".to_string(),
            command_hint: "kubectl rollout undo deployment/{{DEPLOYMENT_NAME}} -n {{K8S_NAMESPACE}}".to_string(),
        },
        DeployTarget::Coolify => RollbackInfo {
            strategy: "manual".to_string(),
            command_hint: "Use the Coolify dashboard to rollback to a previous deployment".to_string(),
        },
        _ => RollbackInfo {
            strategy: "manual".to_string(),
            command_hint: "Manually redeploy the previous version".to_string(),
        },
    }
}

/// Renders the VPS deploy step as a GitHub Actions YAML snippet.
pub fn render_vps_deploy_yaml(image_tag: &str) -> String {
    format!(
        "\
      - name: Deploy to VPS via SSH
        run: |
          ssh ${{{{ secrets.SSH_USER }}}}@${{{{ secrets.SSH_HOST }}}} << 'DEPLOY_EOF'
            docker pull {image_tag}
            cd /opt/app && docker compose up -d
          DEPLOY_EOF\n"
    )
}

/// Renders the Hetzner K8s deploy step as a GitHub Actions YAML snippet.
pub fn render_hetzner_k8s_deploy_yaml(image_tag: &str) -> String {
    format!(
        "\
      - name: Deploy to Hetzner Kubernetes
        run: |
          kubectl set image deployment/${{{{ secrets.DEPLOYMENT_NAME }}}} \\
            app={image_tag} \\
            --namespace=${{{{ secrets.K8S_NAMESPACE }}}}
          kubectl rollout status deployment/${{{{ secrets.DEPLOYMENT_NAME }}}} \\
            --namespace=${{{{ secrets.K8S_NAMESPACE }}}} \\
            --timeout=300s\n"
    )
}

/// Renders the Coolify deploy step as a GitHub Actions YAML snippet.
pub fn render_coolify_deploy_yaml() -> String {
    "\
      - name: Trigger Coolify deployment
        run: |
          curl -fsSL -X POST \"${{ secrets.COOLIFY_WEBHOOK }}\"\n"
        .to_string()
}

/// Renders the deploy YAML snippet for any Hetzner target.
pub fn render_hetzner_deploy_yaml(target: &DeployTarget, image_tag: &str) -> String {
    match target {
        DeployTarget::Vps => render_vps_deploy_yaml(image_tag),
        DeployTarget::HetznerK8s => render_hetzner_k8s_deploy_yaml(image_tag),
        DeployTarget::Coolify => render_coolify_deploy_yaml(),
        _ => format!("      - name: Deploy\n        run: echo 'Unsupported Hetzner target'\n"),
    }
}

/// Returns secrets required for the Hetzner deploy target.
pub fn hetzner_deploy_required_secrets(target: &DeployTarget) -> Vec<String> {
    match target {
        DeployTarget::Vps => vec![
            "SSH_USER".to_string(),
            "SSH_HOST".to_string(),
        ],
        DeployTarget::HetznerK8s => vec![
            "DEPLOYMENT_NAME".to_string(),
            "K8S_NAMESPACE".to_string(),
        ],
        DeployTarget::Coolify => vec![
            "COOLIFY_WEBHOOK".to_string(),
        ],
        _ => vec![],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: &str = "ghcr.io/user/app:sha123";

    // ── generate_hetzner_deploy ───────────────────────────────────────

    #[test]
    fn vps_deploy_step_uses_ssh() {
        let step = generate_hetzner_deploy(&DeployTarget::Vps, IMAGE);
        assert_eq!(step.command, "ssh");
    }

    #[test]
    fn vps_deploy_step_strategy_is_recreate() {
        let step = generate_hetzner_deploy(&DeployTarget::Vps, IMAGE);
        assert_eq!(step.strategy, "recreate");
    }

    #[test]
    fn vps_deploy_step_contains_docker_pull() {
        let step = generate_hetzner_deploy(&DeployTarget::Vps, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("docker pull")));
    }

    #[test]
    fn vps_deploy_step_contains_compose_up() {
        let step = generate_hetzner_deploy(&DeployTarget::Vps, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("docker compose up")));
    }

    #[test]
    fn vps_deploy_step_target_preserved() {
        let step = generate_hetzner_deploy(&DeployTarget::Vps, IMAGE);
        assert_eq!(step.target, DeployTarget::Vps);
    }

    #[test]
    fn k8s_deploy_step_uses_kubectl() {
        let step = generate_hetzner_deploy(&DeployTarget::HetznerK8s, IMAGE);
        assert!(step.command.contains("kubectl"));
    }

    #[test]
    fn k8s_deploy_step_strategy_is_rolling() {
        let step = generate_hetzner_deploy(&DeployTarget::HetznerK8s, IMAGE);
        assert_eq!(step.strategy, "rolling");
    }

    #[test]
    fn k8s_deploy_step_contains_deployment_placeholder() {
        let step = generate_hetzner_deploy(&DeployTarget::HetznerK8s, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{DEPLOYMENT_NAME}}")));
    }

    #[test]
    fn coolify_deploy_step_uses_curl() {
        let step = generate_hetzner_deploy(&DeployTarget::Coolify, IMAGE);
        assert_eq!(step.command, "curl");
    }

    #[test]
    fn coolify_deploy_step_contains_webhook_ref() {
        let step = generate_hetzner_deploy(&DeployTarget::Coolify, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("COOLIFY_WEBHOOK")));
    }

    // ── hetzner_rollback_info ─────────────────────────────────────────

    #[test]
    fn vps_rollback_is_manual() {
        let info = hetzner_rollback_info(&DeployTarget::Vps);
        assert_eq!(info.strategy, "manual");
    }

    #[test]
    fn vps_rollback_mentions_docker_compose() {
        let info = hetzner_rollback_info(&DeployTarget::Vps);
        assert!(info.command_hint.contains("docker compose"));
    }

    #[test]
    fn k8s_rollback_uses_rollout_undo() {
        let info = hetzner_rollback_info(&DeployTarget::HetznerK8s);
        assert_eq!(info.strategy, "rollout-undo");
        assert!(info.command_hint.contains("rollout undo"));
    }

    #[test]
    fn coolify_rollback_references_dashboard() {
        let info = hetzner_rollback_info(&DeployTarget::Coolify);
        assert!(info.command_hint.contains("Coolify dashboard"));
    }

    // ── render_hetzner_deploy_yaml ────────────────────────────────────

    #[test]
    fn vps_yaml_contains_ssh_command() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::Vps, IMAGE);
        assert!(yaml.contains("ssh"));
    }

    #[test]
    fn vps_yaml_contains_docker_pull() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::Vps, IMAGE);
        assert!(yaml.contains("docker pull"));
    }

    #[test]
    fn vps_yaml_contains_docker_compose_up() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::Vps, IMAGE);
        assert!(yaml.contains("docker compose up -d"));
    }

    #[test]
    fn vps_yaml_references_ssh_secrets() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::Vps, IMAGE);
        assert!(yaml.contains("secrets.SSH_USER"));
        assert!(yaml.contains("secrets.SSH_HOST"));
    }

    #[test]
    fn k8s_yaml_contains_kubectl_set_image() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::HetznerK8s, IMAGE);
        assert!(yaml.contains("kubectl set image"));
    }

    #[test]
    fn k8s_yaml_contains_rollout_status() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::HetznerK8s, IMAGE);
        assert!(yaml.contains("kubectl rollout status"));
    }

    #[test]
    fn coolify_yaml_contains_curl_post() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::Coolify, IMAGE);
        assert!(yaml.contains("curl"));
        assert!(yaml.contains("-X POST"));
    }

    #[test]
    fn coolify_yaml_references_webhook_secret() {
        let yaml = render_hetzner_deploy_yaml(&DeployTarget::Coolify, IMAGE);
        assert!(yaml.contains("secrets.COOLIFY_WEBHOOK"));
    }

    // ── hetzner_deploy_required_secrets ───────────────────────────────

    #[test]
    fn vps_requires_ssh_user_and_host() {
        let secrets = hetzner_deploy_required_secrets(&DeployTarget::Vps);
        assert!(secrets.contains(&"SSH_USER".to_string()));
        assert!(secrets.contains(&"SSH_HOST".to_string()));
    }

    #[test]
    fn k8s_requires_deployment_and_namespace() {
        let secrets = hetzner_deploy_required_secrets(&DeployTarget::HetznerK8s);
        assert!(secrets.contains(&"DEPLOYMENT_NAME".to_string()));
        assert!(secrets.contains(&"K8S_NAMESPACE".to_string()));
    }

    #[test]
    fn coolify_requires_webhook() {
        let secrets = hetzner_deploy_required_secrets(&DeployTarget::Coolify);
        assert!(secrets.contains(&"COOLIFY_WEBHOOK".to_string()));
    }
}
