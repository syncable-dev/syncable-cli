//! CD-05 — GCP Workload Identity Federation Authentication Step
//!
//! Generates the GitHub Actions YAML snippet for GCP authentication using
//! Workload Identity Federation (WIF). This is the recommended keyless
//! approach — no service account JSON keys needed:
//!
//! ```yaml
//! - name: Authenticate to Google Cloud
//!   uses: google-github-actions/auth@v2
//!   with:
//!     workload_identity_provider: ${{ secrets.GCP_WORKLOAD_IDENTITY_PROVIDER }}
//!     service_account: ${{ secrets.GCP_SERVICE_ACCOUNT }}
//!
//! - name: Set up Cloud SDK
//!   uses: google-github-actions/setup-gcloud@v2
//! ```
//!
//! The workflow must have `permissions: { id-token: write }` for WIF to work.

use super::schema::AuthStep;

// ── Public types ──────────────────────────────────────────────────────────────

/// Resolved GCP auth configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcpAuthConfig {
    /// GitHub Actions action reference for auth.
    pub auth_action: String,
    /// GitHub Actions action reference for gcloud SDK setup.
    pub setup_gcloud_action: String,
    /// Auth method label.
    pub method: String,
    /// Secrets the user must configure.
    pub required_secrets: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Builds the GCP Workload Identity Federation auth configuration.
pub fn generate_gcp_auth() -> GcpAuthConfig {
    GcpAuthConfig {
        auth_action: "google-github-actions/auth@v2".to_string(),
        setup_gcloud_action: "google-github-actions/setup-gcloud@v2".to_string(),
        method: "workload-identity".to_string(),
        required_secrets: vec![
            "GCP_WORKLOAD_IDENTITY_PROVIDER".to_string(),
            "GCP_SERVICE_ACCOUNT".to_string(),
        ],
    }
}

/// Converts a `GcpAuthConfig` into the schema `AuthStep` for pipeline assembly.
pub fn to_auth_step(config: &GcpAuthConfig) -> AuthStep {
    AuthStep {
        action: Some(config.auth_action.clone()),
        method: config.method.clone(),
        required_secrets: config.required_secrets.clone(),
    }
}

/// Renders the GCP WIF auth steps as a GitHub Actions YAML snippet.
///
/// Emits two steps:
/// 1. `google-github-actions/auth@v2` — authenticates via WIF
/// 2. `google-github-actions/setup-gcloud@v2` — configures the `gcloud` CLI
pub fn render_gcp_auth_yaml(config: &GcpAuthConfig) -> String {
    format!(
        "\
      - name: Authenticate to Google Cloud
        uses: {auth_action}
        with:
          workload_identity_provider: ${{{{ secrets.GCP_WORKLOAD_IDENTITY_PROVIDER }}}}
          service_account: ${{{{ secrets.GCP_SERVICE_ACCOUNT }}}}

      - name: Set up Cloud SDK
        uses: {setup_action}\n",
        auth_action = config.auth_action,
        setup_action = config.setup_gcloud_action,
    )
}

/// Renders the GAR Docker auth configuration step.
///
/// After WIF auth, this step configures Docker to authenticate against
/// Google Artifact Registry using `gcloud auth configure-docker`.
pub fn render_gar_docker_auth_yaml(gar_location: &str) -> String {
    format!(
        "\
      - name: Configure Docker for Artifact Registry
        run: gcloud auth configure-docker {gar_location}-docker.pkg.dev --quiet\n"
    )
}

/// Returns the `permissions` block needed at the job level for WIF.
pub fn gcp_wif_permissions_yaml() -> &'static str {
    "\
    permissions:
      id-token: write
      contents: read\n"
}

/// Renders secrets documentation entries for GCP WIF.
pub fn gcp_auth_secrets_doc() -> String {
    "\
### `GCP_WORKLOAD_IDENTITY_PROVIDER` *(required)*

Full resource name of the Workload Identity Federation provider.

Format: `projects/<project-number>/locations/global/workloadIdentityPools/<pool>/providers/<provider>`

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:**
```bash
gcloud iam workload-identity-pools providers describe <provider> \\
  --project=<project-id> \\
  --location=global \\
  --workload-identity-pool=<pool> \\
  --format='value(name)'
```

---

### `GCP_SERVICE_ACCOUNT` *(required)*

Email address of the Google Cloud service account to impersonate.

Format: `<name>@<project-id>.iam.gserviceaccount.com`

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** `gcloud iam service-accounts list --project=<project-id>`\n"
        .to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_gcp_auth ─────────────────────────────────────────────

    #[test]
    fn config_uses_google_auth_v2() {
        let config = generate_gcp_auth();
        assert_eq!(config.auth_action, "google-github-actions/auth@v2");
    }

    #[test]
    fn config_uses_setup_gcloud_v2() {
        let config = generate_gcp_auth();
        assert_eq!(config.setup_gcloud_action, "google-github-actions/setup-gcloud@v2");
    }

    #[test]
    fn config_method_is_workload_identity() {
        let config = generate_gcp_auth();
        assert_eq!(config.method, "workload-identity");
    }

    #[test]
    fn config_requires_two_secrets() {
        let config = generate_gcp_auth();
        assert_eq!(config.required_secrets.len(), 2);
    }

    #[test]
    fn config_requires_wif_provider() {
        let config = generate_gcp_auth();
        assert!(config
            .required_secrets
            .contains(&"GCP_WORKLOAD_IDENTITY_PROVIDER".to_string()));
    }

    #[test]
    fn config_requires_service_account() {
        let config = generate_gcp_auth();
        assert!(config
            .required_secrets
            .contains(&"GCP_SERVICE_ACCOUNT".to_string()));
    }

    // ── to_auth_step ──────────────────────────────────────────────────

    #[test]
    fn to_auth_step_preserves_action() {
        let config = generate_gcp_auth();
        let step = to_auth_step(&config);
        assert_eq!(step.action, Some("google-github-actions/auth@v2".to_string()));
    }

    #[test]
    fn to_auth_step_preserves_method() {
        let config = generate_gcp_auth();
        let step = to_auth_step(&config);
        assert_eq!(step.method, "workload-identity");
    }

    #[test]
    fn to_auth_step_preserves_secrets() {
        let config = generate_gcp_auth();
        let step = to_auth_step(&config);
        assert_eq!(step.required_secrets.len(), 2);
    }

    // ── render_gcp_auth_yaml ──────────────────────────────────────────

    #[test]
    fn yaml_contains_auth_action() {
        let config = generate_gcp_auth();
        let yaml = render_gcp_auth_yaml(&config);
        assert!(yaml.contains("google-github-actions/auth@v2"));
    }

    #[test]
    fn yaml_contains_setup_gcloud_action() {
        let config = generate_gcp_auth();
        let yaml = render_gcp_auth_yaml(&config);
        assert!(yaml.contains("google-github-actions/setup-gcloud@v2"));
    }

    #[test]
    fn yaml_references_wif_provider_secret() {
        let config = generate_gcp_auth();
        let yaml = render_gcp_auth_yaml(&config);
        assert!(yaml.contains("secrets.GCP_WORKLOAD_IDENTITY_PROVIDER"));
    }

    #[test]
    fn yaml_references_service_account_secret() {
        let config = generate_gcp_auth();
        let yaml = render_gcp_auth_yaml(&config);
        assert!(yaml.contains("secrets.GCP_SERVICE_ACCOUNT"));
    }

    #[test]
    fn yaml_contains_two_step_names() {
        let config = generate_gcp_auth();
        let yaml = render_gcp_auth_yaml(&config);
        assert!(yaml.contains("Authenticate to Google Cloud"));
        assert!(yaml.contains("Set up Cloud SDK"));
    }

    #[test]
    fn yaml_no_hardcoded_json_keys() {
        let config = generate_gcp_auth();
        let yaml = render_gcp_auth_yaml(&config);
        assert!(!yaml.contains("\"type\": \"service_account\""));
        assert!(yaml.contains("${{"));
    }

    // ── render_gar_docker_auth_yaml ───────────────────────────────────

    #[test]
    fn gar_docker_auth_contains_configure_docker() {
        let yaml = render_gar_docker_auth_yaml("us-central1");
        assert!(yaml.contains("gcloud auth configure-docker"));
    }

    #[test]
    fn gar_docker_auth_includes_location() {
        let yaml = render_gar_docker_auth_yaml("europe-west1");
        assert!(yaml.contains("europe-west1-docker.pkg.dev"));
    }

    #[test]
    fn gar_docker_auth_uses_quiet_flag() {
        let yaml = render_gar_docker_auth_yaml("us-central1");
        assert!(yaml.contains("--quiet"));
    }

    // ── gcp_wif_permissions_yaml ──────────────────────────────────────

    #[test]
    fn permissions_contains_id_token_write() {
        let perms = gcp_wif_permissions_yaml();
        assert!(perms.contains("id-token: write"));
    }

    #[test]
    fn permissions_contains_contents_read() {
        let perms = gcp_wif_permissions_yaml();
        assert!(perms.contains("contents: read"));
    }

    // ── gcp_auth_secrets_doc ──────────────────────────────────────────

    #[test]
    fn secrets_doc_mentions_both_secrets() {
        let doc = gcp_auth_secrets_doc();
        assert!(doc.contains("GCP_WORKLOAD_IDENTITY_PROVIDER"));
        assert!(doc.contains("GCP_SERVICE_ACCOUNT"));
    }

    #[test]
    fn secrets_doc_includes_gcloud_commands() {
        let doc = gcp_auth_secrets_doc();
        assert!(doc.contains("gcloud iam"));
    }

    #[test]
    fn secrets_doc_marks_all_as_required() {
        let doc = gcp_auth_secrets_doc();
        assert_eq!(doc.matches("*(required)*").count(), 2);
    }

    #[test]
    fn secrets_doc_includes_format_example() {
        let doc = gcp_auth_secrets_doc();
        assert!(doc.contains("projects/"));
        assert!(doc.contains("iam.gserviceaccount.com"));
    }
}
