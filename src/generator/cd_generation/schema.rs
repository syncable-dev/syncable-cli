//! CD Pipeline Schema — CD-17
//!
//! Defines the canonical, platform-agnostic `CdPipeline` intermediate
//! representation. Template builders (CD-18, CD-19, CD-20) render YAML
//! from this struct, not directly from `CdContext`. This mirrors the CI
//! schema pattern: context collection → schema → template rendering.

use serde::Serialize;

use super::context::{CdPlatform, DeployTarget, MigrationTool, Registry};

// ── Unresolved token ──────────────────────────────────────────────────────────

/// A placeholder that could not be filled deterministically from project files.
///
/// Serialised into `cd-manifest.toml [unresolved]` so the agent fill phase
/// and interactive prompts know exactly what still needs a human decision.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UnresolvedToken {
    /// Token name as it appears in the YAML output, e.g. `"REGISTRY_URL"`.
    pub name: String,
    /// The `{{TOKEN_NAME}}` string injected into the generated YAML.
    pub placeholder: String,
    /// Human-readable hint for what value to supply.
    pub hint: String,
    /// Type annotation used in the manifest file (e.g. `"string"`, `"url"`).
    pub token_type: String,
}

impl UnresolvedToken {
    pub fn new(name: &str, hint: &str, token_type: &str) -> Self {
        Self {
            name: name.to_string(),
            placeholder: format!("{{{{{}}}}}", name),
            hint: hint.to_string(),
            token_type: token_type.to_string(),
        }
    }
}

// ── Step structs ──────────────────────────────────────────────────────────────

/// Cloud provider authentication step.
///
/// Azure uses OIDC federation, GCP uses Workload Identity Federation,
/// Hetzner uses SSH keys or API tokens.
#[derive(Debug, Clone, Serialize)]
pub struct AuthStep {
    /// GitHub Actions action, e.g. `"azure/login@v2"` or `"google-github-actions/auth@v2"`.
    pub action: Option<String>,
    /// Method description: `"oidc"`, `"workload-identity"`, `"ssh"`, `"api-token"`.
    pub method: String,
    /// Secrets that must be configured in the repo (e.g. `"AZURE_CLIENT_ID"`).
    pub required_secrets: Vec<String>,
}

/// Container registry login step.
#[derive(Debug, Clone, Serialize)]
pub struct RegistryStep {
    /// Registry type from context.
    pub registry: Registry,
    /// Login action, e.g. `"docker/login-action@v3"` or a shell command.
    pub login_action: Option<String>,
    /// Full registry URL or placeholder, e.g. `"ghcr.io"` or `"{{REGISTRY_URL}}"`.
    pub registry_url: String,
}

/// Docker build and push step.
#[derive(Debug, Clone, Serialize)]
pub struct DockerBuildPushStep {
    /// Full image reference including registry and tag placeholder.
    /// e.g. `"ghcr.io/org/app:${{ github.sha }}"`.
    pub image_tag: String,
    /// Build context path relative to repo root.
    pub context: String,
    /// Dockerfile path relative to repo root.
    pub dockerfile: String,
    /// Whether to push the image (always `true` for CD).
    pub push: bool,
    /// Enable multi-platform via `docker/setup-buildx-action`.
    pub buildx: bool,
    /// Build arguments to pass, e.g. `["BUILD_ENV=production"]`.
    pub build_args: Vec<String>,
}

/// Database migration step — omitted when no migration tool detected.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationStep {
    /// Tool name for logging and comments.
    pub tool: MigrationTool,
    /// Shell command to run migrations.
    /// e.g. `"npx prisma migrate deploy"`, `"alembic upgrade head"`.
    pub command: String,
    /// Whether migration runs via SSH (Hetzner VPS pattern).
    pub via_ssh: bool,
}

/// Terraform plan + apply step — omitted when no terraform directory found.
#[derive(Debug, Clone, Serialize)]
pub struct TerraformStep {
    /// Working directory for `terraform` commands.
    pub working_directory: String,
    /// Version of Terraform to set up, or `{{TERRAFORM_VERSION}}`.
    pub version: String,
    /// Backend configuration arguments (e.g. `["-backend-config=env/prod.hcl"]`).
    pub backend_config: Vec<String>,
    /// Whether to auto-approve `terraform apply` (typically only in non-prod).
    pub auto_approve: bool,
}

/// Platform-specific deployment step.
#[derive(Debug, Clone, Serialize)]
pub struct DeployStep {
    /// Human-readable strategy label: `"rolling"`, `"blue-green"`, `"canary"`, `"recreate"`.
    pub strategy: String,
    /// Primary deploy command or action.
    pub command: String,
    /// Additional arguments for the deploy command.
    pub args: Vec<String>,
    /// The deployment target for reference.
    pub target: DeployTarget,
}

/// Post-deployment health check step.
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckStep {
    /// URL to probe, e.g. `"https://{{APP_URL}}/health"`.
    pub url: String,
    /// Maximum number of retry attempts.
    pub retries: u32,
    /// Delay between retries in seconds.
    pub interval_secs: u32,
    /// Expected HTTP status code (typically `200`).
    pub expected_status: u16,
}

/// Rollback metadata — not an executable step, but information baked into
/// the generated YAML comments and manifest.
#[derive(Debug, Clone, Serialize)]
pub struct RollbackInfo {
    /// Rollback strategy description: `"redeploy-previous"`, `"helm-rollback"`, `"manual"`.
    pub strategy: String,
    /// Shell command suggestion for manual rollback.
    pub command_hint: String,
}

/// Slack (or other) deployment notification step.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationStep {
    /// Channel or webhook approach: `"slack-webhook"`, `"teams-webhook"`.
    pub channel_type: String,
    /// Secret name for the webhook URL, e.g. `"SLACK_WEBHOOK_URL"`.
    pub webhook_secret: String,
    /// Whether to send on success.
    pub on_success: bool,
    /// Whether to send on failure.
    pub on_failure: bool,
}

/// Per-environment configuration used when rendering per-env deploy jobs.
#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentConfig {
    /// Environment name: `"dev"`, `"staging"`, `"production"`.
    pub name: String,
    /// Branch or tag filter for this environment.
    pub branch_filter: Option<String>,
    /// Whether this environment requires a GitHub environment protection rule
    /// (manual approval).
    pub requires_approval: bool,
    /// URL of the running application in this environment, or placeholder.
    pub app_url: Option<String>,
    /// Optional Kubernetes namespace override.
    pub namespace: Option<String>,
    /// Optional replica count override for this environment.
    pub replicas: Option<u32>,
}

// ── Top-level pipeline ────────────────────────────────────────────────────────

/// Platform-agnostic intermediate representation of a complete CD pipeline.
///
/// Template builders (CD-18, CD-19, CD-20) render YAML from this struct.
/// The agent fill phase patches individual fields without re-running full
/// context collection.
#[derive(Debug, Clone, Serialize)]
pub struct CdPipeline {
    /// Human-readable project name.
    pub project_name: String,
    /// Target cloud platform.
    pub platform: CdPlatform,
    /// Concrete deployment target.
    pub deploy_target: DeployTarget,
    /// Ordered list of environment configs (dev → staging → production).
    pub environments: Vec<EnvironmentConfig>,
    /// Cloud provider authentication step.
    pub auth: AuthStep,
    /// Container registry login step.
    pub registry: RegistryStep,
    /// Docker build and push step.
    pub docker_build_push: DockerBuildPushStep,
    /// Database migration step (omitted if not detected).
    pub migration: Option<MigrationStep>,
    /// Terraform step (omitted if not detected).
    pub terraform: Option<TerraformStep>,
    /// Deployment step.
    pub deploy: DeployStep,
    /// Post-deployment health check.
    pub health_check: HealthCheckStep,
    /// Rollback info baked into manifest and YAML comments.
    pub rollback_info: RollbackInfo,
    /// Optional deployment notification step.
    pub notifications: Option<NotificationStep>,
    /// Tokens that could not be resolved deterministically.
    pub unresolved_tokens: Vec<UnresolvedToken>,
    /// Default git branch.
    pub default_branch: String,
    /// Docker image name (without registry or tag).
    pub image_name: String,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::cd_generation::context::{
        CdPlatform, DeployTarget, MigrationTool, Registry,
    };

    #[test]
    fn unresolved_token_new_formats_placeholder() {
        let token = UnresolvedToken::new("REGISTRY_URL", "Your ACR login server", "url");
        assert_eq!(token.name, "REGISTRY_URL");
        assert_eq!(token.placeholder, "{{REGISTRY_URL}}");
        assert_eq!(token.hint, "Your ACR login server");
        assert_eq!(token.token_type, "url");
    }

    #[test]
    fn auth_step_azure_oidc() {
        let step = AuthStep {
            action: Some("azure/login@v2".to_string()),
            method: "oidc".to_string(),
            required_secrets: vec![
                "AZURE_CLIENT_ID".to_string(),
                "AZURE_TENANT_ID".to_string(),
                "AZURE_SUBSCRIPTION_ID".to_string(),
            ],
        };
        assert_eq!(step.method, "oidc");
        assert_eq!(step.required_secrets.len(), 3);
    }

    #[test]
    fn auth_step_gcp_workload_identity() {
        let step = AuthStep {
            action: Some("google-github-actions/auth@v2".to_string()),
            method: "workload-identity".to_string(),
            required_secrets: vec![
                "GCP_WORKLOAD_IDENTITY_PROVIDER".to_string(),
                "GCP_SERVICE_ACCOUNT".to_string(),
            ],
        };
        assert_eq!(step.method, "workload-identity");
        assert_eq!(step.required_secrets.len(), 2);
    }

    #[test]
    fn auth_step_hetzner_ssh() {
        let step = AuthStep {
            action: None,
            method: "ssh".to_string(),
            required_secrets: vec![
                "SSH_PRIVATE_KEY".to_string(),
                "SSH_HOST".to_string(),
            ],
        };
        assert!(step.action.is_none());
        assert_eq!(step.method, "ssh");
    }

    #[test]
    fn registry_step_ghcr() {
        let step = RegistryStep {
            registry: Registry::Ghcr,
            login_action: Some("docker/login-action@v3".to_string()),
            registry_url: "ghcr.io".to_string(),
        };
        assert_eq!(step.registry_url, "ghcr.io");
    }

    #[test]
    fn registry_step_acr_with_placeholder() {
        let step = RegistryStep {
            registry: Registry::Acr,
            login_action: Some("azure/docker-login@v2".to_string()),
            registry_url: "{{ACR_LOGIN_SERVER}}".to_string(),
        };
        assert!(step.registry_url.contains("{{"));
    }

    #[test]
    fn docker_build_push_step() {
        let step = DockerBuildPushStep {
            image_tag: "ghcr.io/org/app:abc123".to_string(),
            context: ".".to_string(),
            dockerfile: "Dockerfile".to_string(),
            push: true,
            buildx: false,
            build_args: vec!["BUILD_ENV=production".to_string()],
        };
        assert!(step.push);
        assert!(!step.buildx);
        assert_eq!(step.build_args.len(), 1);
    }

    #[test]
    fn migration_step_prisma() {
        let step = MigrationStep {
            tool: MigrationTool::Prisma,
            command: "npx prisma migrate deploy".to_string(),
            via_ssh: false,
        };
        assert_eq!(step.tool, MigrationTool::Prisma);
        assert!(!step.via_ssh);
    }

    #[test]
    fn migration_step_via_ssh() {
        let step = MigrationStep {
            tool: MigrationTool::Alembic,
            command: "ssh deploy@host 'cd /app && alembic upgrade head'".to_string(),
            via_ssh: true,
        };
        assert!(step.via_ssh);
    }

    #[test]
    fn terraform_step_defaults() {
        let step = TerraformStep {
            working_directory: "terraform/".to_string(),
            version: "{{TERRAFORM_VERSION}}".to_string(),
            backend_config: vec![],
            auto_approve: false,
        };
        assert!(!step.auto_approve);
        assert!(step.version.contains("{{"));
    }

    #[test]
    fn deploy_step_cloud_run() {
        let step = DeployStep {
            strategy: "rolling".to_string(),
            command: "gcloud run deploy".to_string(),
            args: vec![
                "--image={{IMAGE_TAG}}".to_string(),
                "--region={{GCP_REGION}}".to_string(),
            ],
            target: DeployTarget::CloudRun,
        };
        assert_eq!(step.strategy, "rolling");
        assert_eq!(step.target, DeployTarget::CloudRun);
    }

    #[test]
    fn health_check_step_defaults() {
        let step = HealthCheckStep {
            url: "https://{{APP_URL}}/health".to_string(),
            retries: 5,
            interval_secs: 10,
            expected_status: 200,
        };
        assert_eq!(step.retries, 5);
        assert_eq!(step.expected_status, 200);
    }

    #[test]
    fn rollback_info() {
        let info = RollbackInfo {
            strategy: "redeploy-previous".to_string(),
            command_hint: "az webapp deployment slot swap --slot staging".to_string(),
        };
        assert_eq!(info.strategy, "redeploy-previous");
    }

    #[test]
    fn notification_step_slack() {
        let step = NotificationStep {
            channel_type: "slack-webhook".to_string(),
            webhook_secret: "SLACK_WEBHOOK_URL".to_string(),
            on_success: true,
            on_failure: true,
        };
        assert!(step.on_success);
        assert!(step.on_failure);
    }

    #[test]
    fn environment_config_production_with_approval() {
        let env = EnvironmentConfig {
            name: "production".to_string(),
            branch_filter: Some("main".to_string()),
            requires_approval: true,
            app_url: Some("https://myapp.com".to_string()),
            namespace: Some("prod".to_string()),
            replicas: Some(3),
        };
        assert!(env.requires_approval);
        assert_eq!(env.replicas, Some(3));
    }

    #[test]
    fn environment_config_dev_no_approval() {
        let env = EnvironmentConfig {
            name: "dev".to_string(),
            branch_filter: Some("develop".to_string()),
            requires_approval: false,
            app_url: None,
            namespace: None,
            replicas: None,
        };
        assert!(!env.requires_approval);
        assert!(env.app_url.is_none());
    }

    #[test]
    fn cd_pipeline_full_assembly() {
        let pipeline = CdPipeline {
            project_name: "my-app".to_string(),
            platform: CdPlatform::Azure,
            deploy_target: DeployTarget::ContainerApps,
            environments: vec![
                EnvironmentConfig {
                    name: "dev".to_string(),
                    branch_filter: Some("develop".to_string()),
                    requires_approval: false,
                    app_url: None,
                    namespace: None,
                    replicas: None,
                },
                EnvironmentConfig {
                    name: "production".to_string(),
                    branch_filter: Some("main".to_string()),
                    requires_approval: true,
                    app_url: Some("https://my-app.azurewebsites.net".to_string()),
                    namespace: None,
                    replicas: Some(2),
                },
            ],
            auth: AuthStep {
                action: Some("azure/login@v2".to_string()),
                method: "oidc".to_string(),
                required_secrets: vec![
                    "AZURE_CLIENT_ID".to_string(),
                    "AZURE_TENANT_ID".to_string(),
                    "AZURE_SUBSCRIPTION_ID".to_string(),
                ],
            },
            registry: RegistryStep {
                registry: Registry::Acr,
                login_action: Some("azure/docker-login@v2".to_string()),
                registry_url: "{{ACR_LOGIN_SERVER}}".to_string(),
            },
            docker_build_push: DockerBuildPushStep {
                image_tag: "{{ACR_LOGIN_SERVER}}/my-app:abc123".to_string(),
                context: ".".to_string(),
                dockerfile: "Dockerfile".to_string(),
                push: true,
                buildx: false,
                build_args: vec![],
            },
            migration: Some(MigrationStep {
                tool: MigrationTool::Prisma,
                command: "npx prisma migrate deploy".to_string(),
                via_ssh: false,
            }),
            terraform: None,
            deploy: DeployStep {
                strategy: "rolling".to_string(),
                command: "az containerapp update".to_string(),
                args: vec![
                    "--name={{APP_NAME}}".to_string(),
                    "--resource-group={{RESOURCE_GROUP}}".to_string(),
                ],
                target: DeployTarget::ContainerApps,
            },
            health_check: HealthCheckStep {
                url: "https://{{APP_URL}}/health".to_string(),
                retries: 5,
                interval_secs: 10,
                expected_status: 200,
            },
            rollback_info: RollbackInfo {
                strategy: "redeploy-previous".to_string(),
                command_hint: "az containerapp revision activate --revision <prev>".to_string(),
            },
            notifications: Some(NotificationStep {
                channel_type: "slack-webhook".to_string(),
                webhook_secret: "SLACK_WEBHOOK_URL".to_string(),
                on_success: true,
                on_failure: true,
            }),
            unresolved_tokens: vec![
                UnresolvedToken::new("ACR_LOGIN_SERVER", "Your Azure Container Registry login server URL", "url"),
                UnresolvedToken::new("APP_URL", "Public URL of your application", "url"),
                UnresolvedToken::new("APP_NAME", "Azure Container App name", "string"),
                UnresolvedToken::new("RESOURCE_GROUP", "Azure resource group name", "string"),
            ],
            default_branch: "main".to_string(),
            image_name: "my-app".to_string(),
        };

        assert_eq!(pipeline.project_name, "my-app");
        assert_eq!(pipeline.environments.len(), 2);
        assert!(pipeline.migration.is_some());
        assert!(pipeline.terraform.is_none());
        assert!(pipeline.notifications.is_some());
        assert_eq!(pipeline.unresolved_tokens.len(), 4);
        assert_eq!(pipeline.default_branch, "main");
    }

    #[test]
    fn cd_pipeline_minimal_hetzner_vps() {
        let pipeline = CdPipeline {
            project_name: "simple-api".to_string(),
            platform: CdPlatform::Hetzner,
            deploy_target: DeployTarget::Vps,
            environments: vec![EnvironmentConfig {
                name: "production".to_string(),
                branch_filter: Some("main".to_string()),
                requires_approval: false,
                app_url: None,
                namespace: None,
                replicas: None,
            }],
            auth: AuthStep {
                action: None,
                method: "ssh".to_string(),
                required_secrets: vec![
                    "SSH_PRIVATE_KEY".to_string(),
                    "SSH_HOST".to_string(),
                ],
            },
            registry: RegistryStep {
                registry: Registry::Ghcr,
                login_action: Some("docker/login-action@v3".to_string()),
                registry_url: "ghcr.io".to_string(),
            },
            docker_build_push: DockerBuildPushStep {
                image_tag: "ghcr.io/user/simple-api:latest".to_string(),
                context: ".".to_string(),
                dockerfile: "Dockerfile".to_string(),
                push: true,
                buildx: false,
                build_args: vec![],
            },
            migration: None,
            terraform: None,
            deploy: DeployStep {
                strategy: "recreate".to_string(),
                command: "ssh deploy@host 'docker compose pull && docker compose up -d'".to_string(),
                args: vec![],
                target: DeployTarget::Vps,
            },
            health_check: HealthCheckStep {
                url: "http://{{SSH_HOST}}:8080/health".to_string(),
                retries: 3,
                interval_secs: 5,
                expected_status: 200,
            },
            rollback_info: RollbackInfo {
                strategy: "manual".to_string(),
                command_hint: "ssh deploy@host 'docker compose down && docker compose up -d'".to_string(),
            },
            notifications: None,
            unresolved_tokens: vec![
                UnresolvedToken::new("SSH_HOST", "IP or hostname of your Hetzner VPS", "string"),
            ],
            default_branch: "main".to_string(),
            image_name: "simple-api".to_string(),
        };

        assert_eq!(pipeline.platform, CdPlatform::Hetzner);
        assert_eq!(pipeline.deploy_target, DeployTarget::Vps);
        assert!(pipeline.migration.is_none());
        assert!(pipeline.notifications.is_none());
        assert_eq!(pipeline.unresolved_tokens.len(), 1);
    }
}
