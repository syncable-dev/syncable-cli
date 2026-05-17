//! CD-04 — Azure OIDC Authentication Step
//!
//! Generates the GitHub Actions YAML snippet for Azure login using
//! OpenID Connect (OIDC) / Workload Identity Federation. This is the
//! recommended zero-secret-rotation approach:
//!
//! ```yaml
//! - name: Azure login (OIDC)
//!   uses: azure/login@v2
//!   with:
//!     client-id: ${{ secrets.AZURE_CLIENT_ID }}
//!     tenant-id: ${{ secrets.AZURE_TENANT_ID }}
//!     subscription-id: ${{ secrets.AZURE_SUBSCRIPTION_ID }}
//! ```
//!
//! The workflow must have `permissions: { id-token: write }` for OIDC to work.

use super::schema::AuthStep;

// ── Public types ──────────────────────────────────────────────────────────────

/// Resolved Azure auth configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AzureAuthConfig {
    /// GitHub Actions action reference.
    pub action: String,
    /// Auth method label.
    pub method: String,
    /// Secrets the user must configure.
    pub required_secrets: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Builds the Azure OIDC auth configuration.
pub fn generate_azure_auth() -> AzureAuthConfig {
    AzureAuthConfig {
        action: "azure/login@v2".to_string(),
        method: "oidc".to_string(),
        required_secrets: vec![
            "AZURE_CLIENT_ID".to_string(),
            "AZURE_TENANT_ID".to_string(),
            "AZURE_SUBSCRIPTION_ID".to_string(),
        ],
    }
}

/// Converts an `AzureAuthConfig` into the schema `AuthStep` for pipeline assembly.
pub fn to_auth_step(config: &AzureAuthConfig) -> AuthStep {
    AuthStep {
        action: Some(config.action.clone()),
        method: config.method.clone(),
        required_secrets: config.required_secrets.clone(),
    }
}

/// Renders the Azure OIDC login step as a GitHub Actions YAML snippet.
///
/// The output includes the `permissions` block comment as a reminder and
/// the login step itself with all three OIDC secrets.
pub fn render_azure_auth_yaml(config: &AzureAuthConfig) -> String {
    format!(
        "\
      - name: Azure login (OIDC)
        uses: {action}
        with:
          client-id: ${{{{ secrets.AZURE_CLIENT_ID }}}}
          tenant-id: ${{{{ secrets.AZURE_TENANT_ID }}}}
          subscription-id: ${{{{ secrets.AZURE_SUBSCRIPTION_ID }}}}\n",
        action = config.action,
    )
}

/// Returns the `permissions` block needed at the job level for OIDC.
pub fn azure_oidc_permissions_yaml() -> &'static str {
    "\
    permissions:
      id-token: write
      contents: read\n"
}

/// Renders secrets documentation entries for Azure OIDC.
pub fn azure_auth_secrets_doc() -> String {
    "\
### `AZURE_CLIENT_ID` *(required)*

Application (client) ID of the Azure AD App Registration used for OIDC federation.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** `az ad app show --id <app-id> --query appId -o tsv`

---

### `AZURE_TENANT_ID` *(required)*

Azure Active Directory tenant ID.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** `az account show --query tenantId -o tsv`

---

### `AZURE_SUBSCRIPTION_ID` *(required)*

Azure subscription ID for the target deployment.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** `az account show --query id -o tsv`\n"
        .to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_azure_auth ───────────────────────────────────────────

    #[test]
    fn config_uses_azure_login_v2() {
        let config = generate_azure_auth();
        assert_eq!(config.action, "azure/login@v2");
    }

    #[test]
    fn config_method_is_oidc() {
        let config = generate_azure_auth();
        assert_eq!(config.method, "oidc");
    }

    #[test]
    fn config_requires_three_secrets() {
        let config = generate_azure_auth();
        assert_eq!(config.required_secrets.len(), 3);
    }

    #[test]
    fn config_requires_client_id() {
        let config = generate_azure_auth();
        assert!(config.required_secrets.contains(&"AZURE_CLIENT_ID".to_string()));
    }

    #[test]
    fn config_requires_tenant_id() {
        let config = generate_azure_auth();
        assert!(config.required_secrets.contains(&"AZURE_TENANT_ID".to_string()));
    }

    #[test]
    fn config_requires_subscription_id() {
        let config = generate_azure_auth();
        assert!(config.required_secrets.contains(&"AZURE_SUBSCRIPTION_ID".to_string()));
    }

    // ── to_auth_step ──────────────────────────────────────────────────

    #[test]
    fn to_auth_step_preserves_action() {
        let config = generate_azure_auth();
        let step = to_auth_step(&config);
        assert_eq!(step.action, Some("azure/login@v2".to_string()));
    }

    #[test]
    fn to_auth_step_preserves_method() {
        let config = generate_azure_auth();
        let step = to_auth_step(&config);
        assert_eq!(step.method, "oidc");
    }

    #[test]
    fn to_auth_step_preserves_secrets() {
        let config = generate_azure_auth();
        let step = to_auth_step(&config);
        assert_eq!(step.required_secrets.len(), 3);
    }

    // ── render_azure_auth_yaml ────────────────────────────────────────

    #[test]
    fn yaml_contains_action_reference() {
        let config = generate_azure_auth();
        let yaml = render_azure_auth_yaml(&config);
        assert!(yaml.contains("azure/login@v2"));
    }

    #[test]
    fn yaml_references_client_id_secret() {
        let config = generate_azure_auth();
        let yaml = render_azure_auth_yaml(&config);
        assert!(yaml.contains("secrets.AZURE_CLIENT_ID"));
    }

    #[test]
    fn yaml_references_tenant_id_secret() {
        let config = generate_azure_auth();
        let yaml = render_azure_auth_yaml(&config);
        assert!(yaml.contains("secrets.AZURE_TENANT_ID"));
    }

    #[test]
    fn yaml_references_subscription_id_secret() {
        let config = generate_azure_auth();
        let yaml = render_azure_auth_yaml(&config);
        assert!(yaml.contains("secrets.AZURE_SUBSCRIPTION_ID"));
    }

    #[test]
    fn yaml_contains_step_name() {
        let config = generate_azure_auth();
        let yaml = render_azure_auth_yaml(&config);
        assert!(yaml.contains("- name: Azure login"));
    }

    #[test]
    fn yaml_no_hardcoded_secret_values() {
        let config = generate_azure_auth();
        let yaml = render_azure_auth_yaml(&config);
        // Should reference secrets, never embed UUIDs or real values
        assert!(!yaml.contains("00000000-"));
        assert!(yaml.contains("${{"));
    }

    // ── azure_oidc_permissions_yaml ───────────────────────────────────

    #[test]
    fn permissions_contains_id_token_write() {
        let perms = azure_oidc_permissions_yaml();
        assert!(perms.contains("id-token: write"));
    }

    #[test]
    fn permissions_contains_contents_read() {
        let perms = azure_oidc_permissions_yaml();
        assert!(perms.contains("contents: read"));
    }

    // ── azure_auth_secrets_doc ────────────────────────────────────────

    #[test]
    fn secrets_doc_mentions_all_three_secrets() {
        let doc = azure_auth_secrets_doc();
        assert!(doc.contains("AZURE_CLIENT_ID"));
        assert!(doc.contains("AZURE_TENANT_ID"));
        assert!(doc.contains("AZURE_SUBSCRIPTION_ID"));
    }

    #[test]
    fn secrets_doc_includes_az_cli_commands() {
        let doc = azure_auth_secrets_doc();
        assert!(doc.contains("az ad app show"));
        assert!(doc.contains("az account show"));
    }

    #[test]
    fn secrets_doc_marks_all_as_required() {
        let doc = azure_auth_secrets_doc();
        assert_eq!(doc.matches("*(required)*").count(), 3);
    }
}
