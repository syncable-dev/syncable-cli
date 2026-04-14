//! CD-03 — Registry Config Module
//!
//! Generates GitHub Actions YAML snippets for container registry login and
//! image tag construction. Supports ACR, GAR, GHCR, and custom registries.
//!
//! Each function returns a ready-to-embed YAML step snippet string. Template
//! builders (Session 4) will compose these snippets into full workflow files.
//!
//! ## Image tag strategy
//!
//! All CD images are tagged with the git SHA for immutability:
//!   `<registry_url>/<image_name>:${{ github.sha }}`
//!
//! The registry URL is either deterministic (e.g. `ghcr.io`) or a
//! `{{PLACEHOLDER}}` token resolved by the token engine.

use super::context::Registry;
use super::schema::RegistryStep;

// ── Public types ──────────────────────────────────────────────────────────────

/// Resolved registry configuration ready for YAML rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryConfig {
    /// Registry type.
    pub registry: Registry,
    /// Login action (GitHub Actions `uses:` reference) or `None` for shell-based login.
    pub login_action: Option<String>,
    /// Full registry URL or placeholder.
    pub registry_url: String,
    /// Secrets required for login.
    pub required_secrets: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Builds a `RegistryConfig` for the given registry type.
pub fn generate_registry_config(registry: &Registry) -> RegistryConfig {
    match registry {
        Registry::Ghcr => RegistryConfig {
            registry: Registry::Ghcr,
            login_action: Some("docker/login-action@v3".to_string()),
            registry_url: "ghcr.io".to_string(),
            required_secrets: vec![],
        },
        Registry::Acr => RegistryConfig {
            registry: Registry::Acr,
            login_action: Some("azure/docker-login@v2".to_string()),
            registry_url: "{{ACR_LOGIN_SERVER}}".to_string(),
            required_secrets: vec![
                "ACR_LOGIN_SERVER".to_string(),
            ],
        },
        Registry::Gar => RegistryConfig {
            registry: Registry::Gar,
            login_action: Some("docker/login-action@v3".to_string()),
            registry_url: "{{GAR_LOCATION}}-docker.pkg.dev".to_string(),
            required_secrets: vec![
                "GAR_LOCATION".to_string(),
                "GCP_PROJECT_ID".to_string(),
            ],
        },
        Registry::Custom(url) => RegistryConfig {
            registry: Registry::Custom(url.clone()),
            login_action: Some("docker/login-action@v3".to_string()),
            registry_url: url.clone(),
            required_secrets: vec![
                "REGISTRY_USERNAME".to_string(),
                "REGISTRY_PASSWORD".to_string(),
            ],
        },
    }
}

/// Converts a `RegistryConfig` into the schema `RegistryStep` for pipeline assembly.
pub fn to_registry_step(config: &RegistryConfig) -> RegistryStep {
    RegistryStep {
        registry: config.registry.clone(),
        login_action: config.login_action.clone(),
        registry_url: config.registry_url.clone(),
    }
}

/// Renders the registry login step as a GitHub Actions YAML snippet.
pub fn render_registry_login_yaml(config: &RegistryConfig) -> String {
    match &config.registry {
        Registry::Ghcr => format!(
            "\
      - name: Log in to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{{{ github.actor }}}}
          password: ${{{{ secrets.GITHUB_TOKEN }}}}\n"
        ),
        Registry::Acr => format!(
            "\
      - name: Log in to Azure Container Registry
        uses: azure/docker-login@v2
        with:
          login-server: ${{{{ secrets.ACR_LOGIN_SERVER }}}}\n"
        ),
        Registry::Gar => format!(
            "\
      - name: Log in to Google Artifact Registry
        uses: docker/login-action@v3
        with:
          registry: ${{{{ secrets.GAR_LOCATION }}}}-docker.pkg.dev\n"
        ),
        Registry::Custom(url) => format!(
            "\
      - name: Log in to container registry
        uses: docker/login-action@v3
        with:
          registry: {url}
          username: ${{{{ secrets.REGISTRY_USERNAME }}}}
          password: ${{{{ secrets.REGISTRY_PASSWORD }}}}\n"
        ),
    }
}

/// Builds the full image tag string for CD pipelines.
///
/// Format: `<registry_url>/<image_name>:${{ github.sha }}`
pub fn build_image_tag(config: &RegistryConfig, image_name: &str) -> String {
    format!(
        "{}/{}:${{{{ github.sha }}}}",
        config.registry_url, image_name
    )
}

/// Builds the image tag for GAR which includes the project ID.
///
/// Format: `<location>-docker.pkg.dev/<project_id>/<repo>/<image>:${{ github.sha }}`
pub fn build_gar_image_tag(image_name: &str) -> String {
    format!(
        "{{{{GAR_LOCATION}}}}-docker.pkg.dev/{{{{GCP_PROJECT_ID}}}}/{image_name}/{image_name}:${{{{ github.sha }}}}"
    )
}

/// Renders the Docker build and push steps as a GitHub Actions YAML snippet.
pub fn render_docker_build_push_yaml(image_tag: &str, dockerfile: &str, context: &str) -> String {
    format!(
        "\
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build and push Docker image
        uses: docker/build-push-action@v6
        with:
          context: {context}
          file: {dockerfile}
          push: true
          tags: {image_tag}
          cache-from: type=gha
          cache-to: type=gha,mode=max\n"
    )
}

/// Returns secrets documentation entries for the registry.
pub fn registry_secrets_doc_entries(config: &RegistryConfig) -> String {
    match &config.registry {
        Registry::Ghcr => "\
### `GITHUB_TOKEN` *(automatic)*

Used to authenticate with GitHub Container Registry. Automatically provided by GitHub Actions.\n"
            .to_string(),
        Registry::Acr => "\
### `ACR_LOGIN_SERVER` *(required)*

Your Azure Container Registry login server URL, e.g. `myapp.azurecr.io`.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** `az acr show --name <registry> --query loginServer -o tsv`\n"
            .to_string(),
        Registry::Gar => "\
### `GAR_LOCATION` *(required)*

Google Artifact Registry location, e.g. `us-central1`.

**Where to set:** Repository → Settings → Secrets and variables → Actions

---

### `GCP_PROJECT_ID` *(required)*

Your Google Cloud project ID.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** `gcloud config get-value project`\n"
            .to_string(),
        Registry::Custom(url) => format!(
            "\
### `REGISTRY_USERNAME` *(required)*

Username for authenticating with `{url}`.

**Where to set:** Repository → Settings → Secrets and variables → Actions

---

### `REGISTRY_PASSWORD` *(required)*

Password or access token for authenticating with `{url}`.

**Where to set:** Repository → Settings → Secrets and variables → Actions\n"
        ),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_registry_config ──────────────────────────────────────

    #[test]
    fn ghcr_config_has_deterministic_url() {
        let config = generate_registry_config(&Registry::Ghcr);
        assert_eq!(config.registry_url, "ghcr.io");
    }

    #[test]
    fn ghcr_config_uses_docker_login_action() {
        let config = generate_registry_config(&Registry::Ghcr);
        assert_eq!(config.login_action.as_deref(), Some("docker/login-action@v3"));
    }

    #[test]
    fn ghcr_config_requires_no_extra_secrets() {
        let config = generate_registry_config(&Registry::Ghcr);
        assert!(config.required_secrets.is_empty());
    }

    #[test]
    fn acr_config_has_placeholder_url() {
        let config = generate_registry_config(&Registry::Acr);
        assert!(config.registry_url.contains("{{ACR_LOGIN_SERVER}}"));
    }

    #[test]
    fn acr_config_uses_azure_docker_login() {
        let config = generate_registry_config(&Registry::Acr);
        assert_eq!(config.login_action.as_deref(), Some("azure/docker-login@v2"));
    }

    #[test]
    fn acr_config_requires_login_server_secret() {
        let config = generate_registry_config(&Registry::Acr);
        assert!(config.required_secrets.contains(&"ACR_LOGIN_SERVER".to_string()));
    }

    #[test]
    fn gar_config_has_placeholder_url() {
        let config = generate_registry_config(&Registry::Gar);
        assert!(config.registry_url.contains("{{GAR_LOCATION}}"));
    }

    #[test]
    fn gar_config_requires_location_and_project() {
        let config = generate_registry_config(&Registry::Gar);
        assert!(config.required_secrets.contains(&"GAR_LOCATION".to_string()));
        assert!(config.required_secrets.contains(&"GCP_PROJECT_ID".to_string()));
    }

    #[test]
    fn custom_config_uses_provided_url() {
        let config = generate_registry_config(&Registry::Custom("my.registry.io".to_string()));
        assert_eq!(config.registry_url, "my.registry.io");
    }

    #[test]
    fn custom_config_requires_username_and_password() {
        let config = generate_registry_config(&Registry::Custom("my.registry.io".to_string()));
        assert!(config.required_secrets.contains(&"REGISTRY_USERNAME".to_string()));
        assert!(config.required_secrets.contains(&"REGISTRY_PASSWORD".to_string()));
    }

    // ── to_registry_step ──────────────────────────────────────────────

    #[test]
    fn to_registry_step_preserves_url() {
        let config = generate_registry_config(&Registry::Ghcr);
        let step = to_registry_step(&config);
        assert_eq!(step.registry_url, "ghcr.io");
        assert_eq!(step.registry, Registry::Ghcr);
    }

    // ── render_registry_login_yaml ────────────────────────────────────

    #[test]
    fn ghcr_yaml_references_github_token() {
        let config = generate_registry_config(&Registry::Ghcr);
        let yaml = render_registry_login_yaml(&config);
        assert!(yaml.contains("secrets.GITHUB_TOKEN"));
    }

    #[test]
    fn ghcr_yaml_references_github_actor() {
        let config = generate_registry_config(&Registry::Ghcr);
        let yaml = render_registry_login_yaml(&config);
        assert!(yaml.contains("github.actor"));
    }

    #[test]
    fn acr_yaml_references_login_server_secret() {
        let config = generate_registry_config(&Registry::Acr);
        let yaml = render_registry_login_yaml(&config);
        assert!(yaml.contains("secrets.ACR_LOGIN_SERVER"));
    }

    #[test]
    fn gar_yaml_references_gar_location() {
        let config = generate_registry_config(&Registry::Gar);
        let yaml = render_registry_login_yaml(&config);
        assert!(yaml.contains("secrets.GAR_LOCATION"));
    }

    #[test]
    fn custom_yaml_references_username_and_password() {
        let config = generate_registry_config(&Registry::Custom("reg.io".to_string()));
        let yaml = render_registry_login_yaml(&config);
        assert!(yaml.contains("secrets.REGISTRY_USERNAME"));
        assert!(yaml.contains("secrets.REGISTRY_PASSWORD"));
    }

    #[test]
    fn custom_yaml_contains_custom_registry_url() {
        let config = generate_registry_config(&Registry::Custom("reg.io".to_string()));
        let yaml = render_registry_login_yaml(&config);
        assert!(yaml.contains("reg.io"));
    }

    #[test]
    fn all_login_yamls_contain_step_name() {
        for reg in &[
            Registry::Ghcr,
            Registry::Acr,
            Registry::Gar,
            Registry::Custom("x.io".to_string()),
        ] {
            let config = generate_registry_config(reg);
            let yaml = render_registry_login_yaml(&config);
            assert!(yaml.contains("- name:"), "Missing step name for {reg}");
        }
    }

    // ── build_image_tag ───────────────────────────────────────────────

    #[test]
    fn image_tag_contains_registry_and_name() {
        let config = generate_registry_config(&Registry::Ghcr);
        let tag = build_image_tag(&config, "my-app");
        assert!(tag.starts_with("ghcr.io/my-app:"));
    }

    #[test]
    fn image_tag_contains_github_sha() {
        let config = generate_registry_config(&Registry::Ghcr);
        let tag = build_image_tag(&config, "my-app");
        assert!(tag.contains("github.sha"));
    }

    #[test]
    fn acr_image_tag_contains_placeholder() {
        let config = generate_registry_config(&Registry::Acr);
        let tag = build_image_tag(&config, "api");
        assert!(tag.contains("{{ACR_LOGIN_SERVER}}"));
    }

    #[test]
    fn gar_image_tag_contains_project_placeholders() {
        let tag = build_gar_image_tag("api");
        assert!(tag.contains("{{GAR_LOCATION}}"));
        assert!(tag.contains("{{GCP_PROJECT_ID}}"));
        assert!(tag.contains("api"));
    }

    // ── render_docker_build_push_yaml ─────────────────────────────────

    #[test]
    fn docker_build_push_yaml_contains_buildx() {
        let yaml = render_docker_build_push_yaml("ghcr.io/app:sha", "Dockerfile", ".");
        assert!(yaml.contains("docker/setup-buildx-action@v3"));
    }

    #[test]
    fn docker_build_push_yaml_contains_build_push_action() {
        let yaml = render_docker_build_push_yaml("ghcr.io/app:sha", "Dockerfile", ".");
        assert!(yaml.contains("docker/build-push-action@v6"));
    }

    #[test]
    fn docker_build_push_yaml_sets_push_true() {
        let yaml = render_docker_build_push_yaml("ghcr.io/app:sha", "Dockerfile", ".");
        assert!(yaml.contains("push: true"));
    }

    #[test]
    fn docker_build_push_yaml_uses_gha_cache() {
        let yaml = render_docker_build_push_yaml("ghcr.io/app:sha", "Dockerfile", ".");
        assert!(yaml.contains("cache-from: type=gha"));
        assert!(yaml.contains("cache-to: type=gha,mode=max"));
    }

    #[test]
    fn docker_build_push_yaml_includes_image_tag() {
        let yaml = render_docker_build_push_yaml("ghcr.io/my-app:abc", "Dockerfile", ".");
        assert!(yaml.contains("ghcr.io/my-app:abc"));
    }

    // ── registry_secrets_doc_entries ───────────────────────────────────

    #[test]
    fn ghcr_secrets_doc_mentions_automatic() {
        let config = generate_registry_config(&Registry::Ghcr);
        let doc = registry_secrets_doc_entries(&config);
        assert!(doc.contains("automatic"));
    }

    #[test]
    fn acr_secrets_doc_mentions_login_server() {
        let config = generate_registry_config(&Registry::Acr);
        let doc = registry_secrets_doc_entries(&config);
        assert!(doc.contains("ACR_LOGIN_SERVER"));
    }

    #[test]
    fn gar_secrets_doc_mentions_location() {
        let config = generate_registry_config(&Registry::Gar);
        let doc = registry_secrets_doc_entries(&config);
        assert!(doc.contains("GAR_LOCATION"));
        assert!(doc.contains("GCP_PROJECT_ID"));
    }

    #[test]
    fn custom_secrets_doc_mentions_custom_url() {
        let config = generate_registry_config(&Registry::Custom("reg.io".to_string()));
        let doc = registry_secrets_doc_entries(&config);
        assert!(doc.contains("reg.io"));
    }
}
