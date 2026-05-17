//! CD-06 — Hetzner SSH Authentication Step
//!
//! Generates the GitHub Actions YAML snippets for Hetzner deployments.
//! Hetzner has no managed OIDC integration, so we use:
//!
//! - **VPS / Docker Compose targets:** SSH key via `webfactory/ssh-agent@v0.9.0`
//! - **K8s targets:** `kubectl` kubeconfig written from a secret
//!
//! ## VPS pattern
//!
//! ```yaml
//! - name: Set up SSH agent
//!   uses: webfactory/ssh-agent@v0.9.0
//!   with:
//!     ssh-private-key: ${{ secrets.SSH_PRIVATE_KEY }}
//!
//! - name: Add host to known_hosts
//!   run: ssh-keyscan -H ${{ secrets.SSH_HOST }} >> ~/.ssh/known_hosts
//! ```
//!
//! ## K8s pattern
//!
//! ```yaml
//! - name: Set up kubeconfig
//!   run: |
//!     mkdir -p ~/.kube
//!     echo "${{ secrets.KUBECONFIG }}" > ~/.kube/config
//!     chmod 600 ~/.kube/config
//! ```

use super::context::DeployTarget;
use super::schema::AuthStep;

// ── Public types ──────────────────────────────────────────────────────────────

/// Resolved Hetzner auth configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HetznerAuthConfig {
    /// Auth method label: `"ssh"` or `"kubeconfig"`.
    pub method: String,
    /// Secrets the user must configure.
    pub required_secrets: Vec<String>,
    /// Deploy target determines which auth pattern to use.
    pub target: DeployTarget,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Builds the Hetzner auth configuration for the given deploy target.
///
/// - VPS / Coolify → SSH-based auth
/// - HetznerK8s → Kubeconfig-based auth
pub fn generate_hetzner_auth(target: &DeployTarget) -> HetznerAuthConfig {
    match target {
        DeployTarget::HetznerK8s => HetznerAuthConfig {
            method: "kubeconfig".to_string(),
            required_secrets: vec!["KUBECONFIG".to_string()],
            target: target.clone(),
        },
        // VPS, Coolify, and any other Hetzner target use SSH
        _ => HetznerAuthConfig {
            method: "ssh".to_string(),
            required_secrets: vec![
                "SSH_PRIVATE_KEY".to_string(),
                "SSH_HOST".to_string(),
                "SSH_USER".to_string(),
            ],
            target: target.clone(),
        },
    }
}

/// Converts a `HetznerAuthConfig` into the schema `AuthStep` for pipeline assembly.
pub fn to_auth_step(config: &HetznerAuthConfig) -> AuthStep {
    AuthStep {
        action: match config.method.as_str() {
            "ssh" => Some("webfactory/ssh-agent@v0.9.0".to_string()),
            _ => None,
        },
        method: config.method.clone(),
        required_secrets: config.required_secrets.clone(),
    }
}

/// Renders the Hetzner auth steps as a GitHub Actions YAML snippet.
///
/// For SSH targets, emits the ssh-agent setup + known_hosts step.
/// For K8s targets, emits the kubeconfig write step.
pub fn render_hetzner_auth_yaml(config: &HetznerAuthConfig) -> String {
    match config.method.as_str() {
        "kubeconfig" => render_kubeconfig_auth(),
        _ => render_ssh_auth(),
    }
}

/// Renders the SSH-based auth snippet (VPS / Coolify).
fn render_ssh_auth() -> String {
    format!(
        "\
      - name: Set up SSH agent
        uses: webfactory/ssh-agent@v0.9.0
        with:
          ssh-private-key: ${{{{ secrets.SSH_PRIVATE_KEY }}}}

      - name: Add host to known_hosts
        run: ssh-keyscan -H ${{{{ secrets.SSH_HOST }}}} >> ~/.ssh/known_hosts\n"
    )
}

/// Renders the kubeconfig-based auth snippet (HetznerK8s).
fn render_kubeconfig_auth() -> String {
    format!(
        "\
      - name: Set up kubeconfig
        run: |
          mkdir -p ~/.kube
          echo \"${{{{ secrets.KUBECONFIG }}}}\" > ~/.kube/config
          chmod 600 ~/.kube/config\n"
    )
}

/// Renders secrets documentation entries for Hetzner auth.
pub fn hetzner_auth_secrets_doc(config: &HetznerAuthConfig) -> String {
    match config.method.as_str() {
        "kubeconfig" => "\
### `KUBECONFIG` *(required)*

Base64-encoded or raw kubeconfig for the Hetzner Kubernetes cluster.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:**
```bash
hcloud kubernetes cluster kubeconfig --name <cluster>
# or
kubectl config view --raw --minify
```

**Important:** Ensure the kubeconfig uses a service account token, not a user certificate that expires.\n"
            .to_string(),
        _ => "\
### `SSH_PRIVATE_KEY` *(required)*

Ed25519 or RSA private key for SSH access to the Hetzner VPS.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:**
```bash
ssh-keygen -t ed25519 -C \"github-actions-deploy\" -f deploy_key -N \"\"
# Add deploy_key.pub to the server's ~/.ssh/authorized_keys
# Paste the contents of deploy_key into the secret
```

---

### `SSH_HOST` *(required)*

IP address or hostname of the Hetzner VPS.

**Where to set:** Repository → Settings → Secrets and variables → Actions

---

### `SSH_USER` *(required)*

Username for SSH login, e.g. `deploy` or `root`.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**Best practice:** Create a dedicated `deploy` user with limited sudo privileges rather than using `root`.\n"
            .to_string(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::context::DeployTarget;

    // ── generate_hetzner_auth — VPS ───────────────────────────────────

    #[test]
    fn vps_config_method_is_ssh() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        assert_eq!(config.method, "ssh");
    }

    #[test]
    fn vps_config_requires_three_secrets() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        assert_eq!(config.required_secrets.len(), 3);
    }

    #[test]
    fn vps_config_requires_ssh_private_key() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        assert!(config.required_secrets.contains(&"SSH_PRIVATE_KEY".to_string()));
    }

    #[test]
    fn vps_config_requires_ssh_host() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        assert!(config.required_secrets.contains(&"SSH_HOST".to_string()));
    }

    #[test]
    fn vps_config_requires_ssh_user() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        assert!(config.required_secrets.contains(&"SSH_USER".to_string()));
    }

    // ── generate_hetzner_auth — Coolify ───────────────────────────────

    #[test]
    fn coolify_also_uses_ssh() {
        let config = generate_hetzner_auth(&DeployTarget::Coolify);
        assert_eq!(config.method, "ssh");
    }

    // ── generate_hetzner_auth — K8s ───────────────────────────────────

    #[test]
    fn k8s_config_method_is_kubeconfig() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        assert_eq!(config.method, "kubeconfig");
    }

    #[test]
    fn k8s_config_requires_one_secret() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        assert_eq!(config.required_secrets.len(), 1);
    }

    #[test]
    fn k8s_config_requires_kubeconfig() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        assert!(config.required_secrets.contains(&"KUBECONFIG".to_string()));
    }

    // ── to_auth_step ──────────────────────────────────────────────────

    #[test]
    fn ssh_auth_step_has_action() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let step = to_auth_step(&config);
        assert_eq!(step.action, Some("webfactory/ssh-agent@v0.9.0".to_string()));
    }

    #[test]
    fn k8s_auth_step_has_no_action() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        let step = to_auth_step(&config);
        assert!(step.action.is_none());
    }

    #[test]
    fn to_auth_step_preserves_method() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let step = to_auth_step(&config);
        assert_eq!(step.method, "ssh");
    }

    #[test]
    fn to_auth_step_preserves_secrets() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let step = to_auth_step(&config);
        assert_eq!(step.required_secrets.len(), 3);
    }

    // ── render_hetzner_auth_yaml — SSH ────────────────────────────────

    #[test]
    fn ssh_yaml_contains_ssh_agent_action() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(yaml.contains("webfactory/ssh-agent@v0.9.0"));
    }

    #[test]
    fn ssh_yaml_references_private_key_secret() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(yaml.contains("secrets.SSH_PRIVATE_KEY"));
    }

    #[test]
    fn ssh_yaml_contains_known_hosts_step() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(yaml.contains("ssh-keyscan"));
        assert!(yaml.contains("known_hosts"));
    }

    #[test]
    fn ssh_yaml_references_host_secret() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(yaml.contains("secrets.SSH_HOST"));
    }

    #[test]
    fn ssh_yaml_contains_two_steps() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let yaml = render_hetzner_auth_yaml(&config);
        let step_count = yaml.matches("- name:").count();
        assert_eq!(step_count, 2);
    }

    // ── render_hetzner_auth_yaml — Kubeconfig ─────────────────────────

    #[test]
    fn k8s_yaml_creates_kube_directory() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(yaml.contains("mkdir -p ~/.kube"));
    }

    #[test]
    fn k8s_yaml_writes_kubeconfig() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(yaml.contains("secrets.KUBECONFIG"));
        assert!(yaml.contains("~/.kube/config"));
    }

    #[test]
    fn k8s_yaml_sets_secure_permissions() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(yaml.contains("chmod 600"));
    }

    #[test]
    fn k8s_yaml_does_not_contain_ssh_agent() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        let yaml = render_hetzner_auth_yaml(&config);
        assert!(!yaml.contains("ssh-agent"));
    }

    // ── hetzner_auth_secrets_doc ──────────────────────────────────────

    #[test]
    fn ssh_secrets_doc_mentions_all_secrets() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let doc = hetzner_auth_secrets_doc(&config);
        assert!(doc.contains("SSH_PRIVATE_KEY"));
        assert!(doc.contains("SSH_HOST"));
        assert!(doc.contains("SSH_USER"));
    }

    #[test]
    fn ssh_secrets_doc_includes_keygen_instructions() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let doc = hetzner_auth_secrets_doc(&config);
        assert!(doc.contains("ssh-keygen"));
    }

    #[test]
    fn k8s_secrets_doc_mentions_kubeconfig() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        let doc = hetzner_auth_secrets_doc(&config);
        assert!(doc.contains("KUBECONFIG"));
    }

    #[test]
    fn k8s_secrets_doc_includes_hcloud_command() {
        let config = generate_hetzner_auth(&DeployTarget::HetznerK8s);
        let doc = hetzner_auth_secrets_doc(&config);
        assert!(doc.contains("hcloud"));
    }

    #[test]
    fn ssh_secrets_doc_recommends_deploy_user() {
        let config = generate_hetzner_auth(&DeployTarget::Vps);
        let doc = hetzner_auth_secrets_doc(&config);
        assert!(doc.contains("deploy"));
    }
}
