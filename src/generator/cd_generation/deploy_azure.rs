//! CD-07 — Azure Deploy Step Generator
//!
//! Generates GitHub Actions YAML snippets for Azure deployment targets:
//!
//! | Target          | Action                               | Key params                    |
//! |-----------------|--------------------------------------|-------------------------------|
//! | App Service     | `azure/webapps-deploy@v3`            | `app-name`, `images`          |
//! | AKS             | `azure/k8s-deploy@v5`                | `namespace`, `manifests`      |
//! | Container Apps  | `azure/container-apps-deploy@v2`     | `containerAppName`, `image`   |
//!
//! Each function returns a `DeployStep` for the schema and a YAML snippet
//! string for direct embedding. Rollback hints are also provided per target.

use super::context::DeployTarget;
use super::schema::{DeployStep, RollbackInfo};

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates the deploy step for the given Azure target.
pub fn generate_azure_deploy(target: &DeployTarget, image_tag: &str) -> DeployStep {
    match target {
        DeployTarget::AppService => DeployStep {
            strategy: "rolling".to_string(),
            command: "azure/webapps-deploy@v3".to_string(),
            args: vec![
                "app-name={{APP_NAME}}".to_string(),
                format!("images={image_tag}"),
            ],
            target: target.clone(),
        },
        DeployTarget::Aks => DeployStep {
            strategy: "rolling".to_string(),
            command: "azure/k8s-deploy@v5".to_string(),
            args: vec![
                "namespace={{K8S_NAMESPACE}}".to_string(),
                "manifests={{K8S_MANIFEST_DIR}}".to_string(),
                format!("images={image_tag}"),
            ],
            target: target.clone(),
        },
        DeployTarget::ContainerApps => DeployStep {
            strategy: "rolling".to_string(),
            command: "azure/container-apps-deploy@v2".to_string(),
            args: vec![
                "containerAppName={{APP_NAME}}".to_string(),
                "resourceGroup={{RESOURCE_GROUP}}".to_string(),
                format!("imageToDeploy={image_tag}"),
            ],
            target: target.clone(),
        },
        // Non-Azure targets should not reach here; return a sensible fallback.
        other => DeployStep {
            strategy: "rolling".to_string(),
            command: format!("echo 'Unsupported Azure target: {other}'"),
            args: vec![],
            target: other.clone(),
        },
    }
}

/// Generates rollback info for the given Azure target.
pub fn azure_rollback_info(target: &DeployTarget) -> RollbackInfo {
    match target {
        DeployTarget::AppService => RollbackInfo {
            strategy: "redeploy-previous".to_string(),
            command_hint: "az webapp deployment slot swap --resource-group {{RESOURCE_GROUP}} --name {{APP_NAME}} --slot staging --target-slot production".to_string(),
        },
        DeployTarget::Aks => RollbackInfo {
            strategy: "rollout-undo".to_string(),
            command_hint: "kubectl rollout undo deployment/{{DEPLOYMENT_NAME}} -n {{K8S_NAMESPACE}}".to_string(),
        },
        DeployTarget::ContainerApps => RollbackInfo {
            strategy: "redeploy-previous".to_string(),
            command_hint: "az containerapp revision activate --name {{APP_NAME}} --resource-group {{RESOURCE_GROUP}} --revision <previous-revision>".to_string(),
        },
        _ => RollbackInfo {
            strategy: "manual".to_string(),
            command_hint: "Manually redeploy the previous version".to_string(),
        },
    }
}

/// Renders the App Service deploy step as a GitHub Actions YAML snippet.
pub fn render_app_service_deploy_yaml(image_tag: &str) -> String {
    format!(
        "\
      - name: Deploy to Azure App Service
        uses: azure/webapps-deploy@v3
        with:
          app-name: ${{{{ secrets.APP_NAME }}}}
          images: {image_tag}\n"
    )
}

/// Renders the AKS deploy step as a GitHub Actions YAML snippet.
pub fn render_aks_deploy_yaml(image_tag: &str) -> String {
    format!(
        "\
      - name: Set AKS context
        uses: azure/aks-set-context@v4
        with:
          resource-group: ${{{{ secrets.RESOURCE_GROUP }}}}
          cluster-name: ${{{{ secrets.AKS_CLUSTER_NAME }}}}

      - name: Deploy to AKS
        uses: azure/k8s-deploy@v5
        with:
          namespace: ${{{{ secrets.K8S_NAMESPACE }}}}
          manifests: |
            ${{{{ secrets.K8S_MANIFEST_DIR }}}}/deployment.yaml
            ${{{{ secrets.K8S_MANIFEST_DIR }}}}/service.yaml
          images: {image_tag}\n"
    )
}

/// Renders the Container Apps deploy step as a GitHub Actions YAML snippet.
pub fn render_container_apps_deploy_yaml(image_tag: &str) -> String {
    format!(
        "\
      - name: Deploy to Azure Container Apps
        uses: azure/container-apps-deploy@v2
        with:
          containerAppName: ${{{{ secrets.APP_NAME }}}}
          resourceGroup: ${{{{ secrets.RESOURCE_GROUP }}}}
          imageToDeploy: {image_tag}\n"
    )
}

/// Renders the deploy YAML snippet for any Azure target.
pub fn render_azure_deploy_yaml(target: &DeployTarget, image_tag: &str) -> String {
    match target {
        DeployTarget::AppService => render_app_service_deploy_yaml(image_tag),
        DeployTarget::Aks => render_aks_deploy_yaml(image_tag),
        DeployTarget::ContainerApps => render_container_apps_deploy_yaml(image_tag),
        _ => format!("      - name: Deploy\n        run: echo 'Unsupported Azure target'\n"),
    }
}

/// Returns secrets required for the Azure deploy target.
pub fn azure_deploy_required_secrets(target: &DeployTarget) -> Vec<String> {
    match target {
        DeployTarget::AppService => vec![
            "APP_NAME".to_string(),
        ],
        DeployTarget::Aks => vec![
            "RESOURCE_GROUP".to_string(),
            "AKS_CLUSTER_NAME".to_string(),
            "K8S_NAMESPACE".to_string(),
            "K8S_MANIFEST_DIR".to_string(),
        ],
        DeployTarget::ContainerApps => vec![
            "APP_NAME".to_string(),
            "RESOURCE_GROUP".to_string(),
        ],
        _ => vec![],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: &str = "myacr.azurecr.io/app:sha123";

    // ── generate_azure_deploy ─────────────────────────────────────────

    #[test]
    fn app_service_deploy_step_uses_correct_action() {
        let step = generate_azure_deploy(&DeployTarget::AppService, IMAGE);
        assert_eq!(step.command, "azure/webapps-deploy@v3");
    }

    #[test]
    fn app_service_deploy_step_strategy_is_rolling() {
        let step = generate_azure_deploy(&DeployTarget::AppService, IMAGE);
        assert_eq!(step.strategy, "rolling");
    }

    #[test]
    fn app_service_deploy_step_contains_app_name_placeholder() {
        let step = generate_azure_deploy(&DeployTarget::AppService, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{APP_NAME}}")));
    }

    #[test]
    fn app_service_deploy_step_contains_image_tag() {
        let step = generate_azure_deploy(&DeployTarget::AppService, IMAGE);
        assert!(step.args.iter().any(|a| a.contains(IMAGE)));
    }

    #[test]
    fn aks_deploy_step_uses_correct_action() {
        let step = generate_azure_deploy(&DeployTarget::Aks, IMAGE);
        assert_eq!(step.command, "azure/k8s-deploy@v5");
    }

    #[test]
    fn aks_deploy_step_contains_namespace_placeholder() {
        let step = generate_azure_deploy(&DeployTarget::Aks, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{K8S_NAMESPACE}}")));
    }

    #[test]
    fn aks_deploy_step_contains_manifest_dir_placeholder() {
        let step = generate_azure_deploy(&DeployTarget::Aks, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{K8S_MANIFEST_DIR}}")));
    }

    #[test]
    fn container_apps_deploy_step_uses_correct_action() {
        let step = generate_azure_deploy(&DeployTarget::ContainerApps, IMAGE);
        assert_eq!(step.command, "azure/container-apps-deploy@v2");
    }

    #[test]
    fn container_apps_deploy_step_contains_resource_group() {
        let step = generate_azure_deploy(&DeployTarget::ContainerApps, IMAGE);
        assert!(step.args.iter().any(|a| a.contains("{{RESOURCE_GROUP}}")));
    }

    #[test]
    fn container_apps_deploy_step_target_preserved() {
        let step = generate_azure_deploy(&DeployTarget::ContainerApps, IMAGE);
        assert_eq!(step.target, DeployTarget::ContainerApps);
    }

    // ── azure_rollback_info ───────────────────────────────────────────

    #[test]
    fn app_service_rollback_strategy() {
        let info = azure_rollback_info(&DeployTarget::AppService);
        assert_eq!(info.strategy, "redeploy-previous");
    }

    #[test]
    fn app_service_rollback_uses_slot_swap() {
        let info = azure_rollback_info(&DeployTarget::AppService);
        assert!(info.command_hint.contains("slot swap"));
    }

    #[test]
    fn aks_rollback_uses_rollout_undo() {
        let info = azure_rollback_info(&DeployTarget::Aks);
        assert_eq!(info.strategy, "rollout-undo");
        assert!(info.command_hint.contains("rollout undo"));
    }

    #[test]
    fn container_apps_rollback_activates_previous_revision() {
        let info = azure_rollback_info(&DeployTarget::ContainerApps);
        assert!(info.command_hint.contains("revision activate"));
    }

    // ── render_azure_deploy_yaml ──────────────────────────────────────

    #[test]
    fn app_service_yaml_contains_action() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::AppService, IMAGE);
        assert!(yaml.contains("azure/webapps-deploy@v3"));
    }

    #[test]
    fn app_service_yaml_contains_image() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::AppService, IMAGE);
        assert!(yaml.contains(IMAGE));
    }

    #[test]
    fn app_service_yaml_references_app_name_secret() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::AppService, IMAGE);
        assert!(yaml.contains("secrets.APP_NAME"));
    }

    #[test]
    fn aks_yaml_contains_k8s_deploy_action() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::Aks, IMAGE);
        assert!(yaml.contains("azure/k8s-deploy@v5"));
    }

    #[test]
    fn aks_yaml_contains_set_context() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::Aks, IMAGE);
        assert!(yaml.contains("azure/aks-set-context@v4"));
    }

    #[test]
    fn aks_yaml_references_cluster_name() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::Aks, IMAGE);
        assert!(yaml.contains("secrets.AKS_CLUSTER_NAME"));
    }

    #[test]
    fn container_apps_yaml_contains_action() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::ContainerApps, IMAGE);
        assert!(yaml.contains("azure/container-apps-deploy@v2"));
    }

    #[test]
    fn container_apps_yaml_references_resource_group() {
        let yaml = render_azure_deploy_yaml(&DeployTarget::ContainerApps, IMAGE);
        assert!(yaml.contains("secrets.RESOURCE_GROUP"));
    }

    // ── azure_deploy_required_secrets ─────────────────────────────────

    #[test]
    fn app_service_requires_app_name() {
        let secrets = azure_deploy_required_secrets(&DeployTarget::AppService);
        assert!(secrets.contains(&"APP_NAME".to_string()));
    }

    #[test]
    fn aks_requires_four_secrets() {
        let secrets = azure_deploy_required_secrets(&DeployTarget::Aks);
        assert_eq!(secrets.len(), 4);
    }

    #[test]
    fn container_apps_requires_app_name_and_resource_group() {
        let secrets = azure_deploy_required_secrets(&DeployTarget::ContainerApps);
        assert!(secrets.contains(&"APP_NAME".to_string()));
        assert!(secrets.contains(&"RESOURCE_GROUP".to_string()));
    }
}
