//! CD-27/28 — CD Secrets Inventory & Hetzner Prerequisites
//!
//! Scans a rendered CD pipeline YAML for `secrets.*` references, deduplicates
//! them, and formats the CD section of `SECRETS_REQUIRED.md`.
//!
//! For Hetzner targets, appends a firewall & network prerequisites checklist
//! (CD-28) so the user knows about SSH keys, firewall rules, and Docker setup.

use std::collections::BTreeSet;

use crate::generator::cd_generation::context::CdPlatform;

// ── Public API ────────────────────────────────────────────────────────────────

/// Scans `yaml` for secret references and returns the CD portion of the
/// secrets document.
pub fn generate_cd_secrets_doc(yaml: &str, platform: &CdPlatform) -> String {
    let names = collect_cd_secret_names(yaml);
    let mut doc = render_cd_secrets_table(&names, platform);

    // CD-28: append Hetzner prerequisites when applicable
    if *platform == CdPlatform::Hetzner {
        doc.push_str(&hetzner_prerequisites_checklist());
    }

    doc
}

/// Collects all secret names referenced in a CD pipeline YAML.
///
/// Matches patterns like:
/// - `${{ secrets.FOO }}`
/// - `secrets.FOO`
pub fn collect_cd_secret_names(yaml: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    // Pattern: secrets.NAME — skip the first segment (text before the first match)
    let segments: Vec<&str> = yaml.split("secrets.").collect();
    for segment in segments.iter().skip(1) {
        if let Some(name) = extract_secret_name(segment) {
            names.insert(name);
        }
    }

    names
}

// ── Render ────────────────────────────────────────────────────────────────────

fn render_cd_secrets_table(names: &BTreeSet<String>, platform: &CdPlatform) -> String {
    if names.is_empty() {
        return "No CD secrets detected in the generated pipeline.\n".to_string();
    }

    let mut md = String::new();
    md.push_str("| Secret | Description | How to Obtain |\n");
    md.push_str("|--------|-------------|---------------|\n");

    for name in names {
        let (desc, how) = secret_metadata(name, platform);
        md.push_str(&format!("| `{name}` | {desc} | {how} |\n"));
    }

    md
}

/// Returns (description, how_to_obtain) for well-known CD secrets.
fn secret_metadata(name: &str, platform: &CdPlatform) -> (&'static str, &'static str) {
    match name {
        // Azure
        "AZURE_CLIENT_ID" => (
            "Azure AD App Registration client ID",
            "`az ad app create --display-name <name>` → appId",
        ),
        "AZURE_TENANT_ID" => (
            "Azure AD tenant ID",
            "`az account show` → tenantId",
        ),
        "AZURE_SUBSCRIPTION_ID" => (
            "Azure subscription ID",
            "`az account show` → id",
        ),
        "ACR_LOGIN_SERVER" => (
            "Azure Container Registry login server",
            "`az acr show --name <acr> --query loginServer`",
        ),
        // GCP
        "GCP_PROJECT_ID" => (
            "GCP project ID",
            "`gcloud config get-value project`",
        ),
        "GCP_WORKLOAD_IDENTITY_PROVIDER" => (
            "Workload Identity Federation provider",
            "IAM → Workload Identity Pools → Provider",
        ),
        "GCP_SERVICE_ACCOUNT" => (
            "GCP service account email",
            "`gcloud iam service-accounts list`",
        ),
        "GAR_LOCATION" => (
            "Google Artifact Registry location",
            "e.g. `us-central1`, `europe-west1`",
        ),
        // Hetzner
        "SSH_PRIVATE_KEY" => (
            "SSH private key for VPS access",
            "`ssh-keygen -t ed25519` → add public key to Hetzner project",
        ),
        "DEPLOY_HOST" => (
            "VPS hostname or IP address",
            "Hetzner Cloud Console → Server → IP",
        ),
        "DEPLOY_USER" => (
            "SSH user on the target server",
            "Typically `root` or a deploy user",
        ),
        "KUBECONFIG_DATA" => (
            "Base64-encoded kubeconfig for k8s cluster",
            "`cat kubeconfig | base64`",
        ),
        // Notifications
        "SLACK_WEBHOOK_URL" => (
            "Slack incoming webhook URL",
            "Slack API → Incoming Webhooks → Create",
        ),
        // Registry (generic)
        "GHCR_TOKEN" | "CR_PAT" => (
            "GitHub Container Registry personal access token",
            "GitHub Settings → Developer → PAT → `write:packages`",
        ),
        // Fallback
        _ => match platform {
            CdPlatform::Azure => (
                "Azure-specific secret",
                "Azure Portal → App Registrations / Key Vault",
            ),
            CdPlatform::Gcp => (
                "GCP-specific secret",
                "GCP Console → Secret Manager",
            ),
            CdPlatform::Hetzner => (
                "Hetzner/deployment secret",
                "Hetzner Cloud Console or SSH key management",
            ),
        },
    }
}

/// CD-28 — Hetzner firewall & network prerequisites checklist.
fn hetzner_prerequisites_checklist() -> String {
    "\n### Hetzner Prerequisites Checklist\n\n\
     Before deploying to Hetzner, ensure the following are configured:\n\n\
     - [ ] **SSH key** added to your Hetzner project (Cloud Console → SSH Keys)\n\
     - [ ] **Firewall rules** configured:\n\
       - Port 22 (SSH) — for deployment access\n\
       - Port 80 (HTTP) — for web traffic\n\
       - Port 443 (HTTPS) — for secure web traffic\n\
       - Port 6443 (K8s API) — if using Kubernetes\n\
     - [ ] **Docker installed** on the target VPS (`curl -fsSL https://get.docker.com | sh`)\n\
     - [ ] **Docker Compose** installed (or use Docker Swarm mode)\n\
     - [ ] **Deploy user** created with Docker group membership (`usermod -aG docker deploy`)\n\
     - [ ] **DNS** configured to point to the server IP\n"
        .to_string()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extracts a secret name from text immediately following `secrets.`.
fn extract_secret_name(after_dot: &str) -> Option<String> {
    let name: String = after_dot
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_secrets_from_yaml() {
        let yaml = r#"
        env:
          TOKEN: ${{ secrets.AZURE_CLIENT_ID }}
          OTHER: ${{ secrets.AZURE_TENANT_ID }}
        "#;
        let names = collect_cd_secret_names(yaml);
        assert!(names.contains("AZURE_CLIENT_ID"));
        assert!(names.contains("AZURE_TENANT_ID"));
    }

    #[test]
    fn collect_deduplicates() {
        let yaml = "secrets.FOO and secrets.FOO again";
        let names = collect_cd_secret_names(yaml);
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn collect_empty_yaml() {
        let names = collect_cd_secret_names("no secrets here");
        assert!(names.is_empty());
    }

    #[test]
    fn generate_doc_azure() {
        let yaml = "${{ secrets.AZURE_CLIENT_ID }}";
        let doc = generate_cd_secrets_doc(yaml, &CdPlatform::Azure);
        assert!(doc.contains("AZURE_CLIENT_ID"));
        assert!(doc.contains("Azure AD"));
        assert!(!doc.contains("Hetzner Prerequisites"));
    }

    #[test]
    fn generate_doc_gcp() {
        let yaml = "${{ secrets.GCP_PROJECT_ID }}";
        let doc = generate_cd_secrets_doc(yaml, &CdPlatform::Gcp);
        assert!(doc.contains("GCP_PROJECT_ID"));
    }

    #[test]
    fn generate_doc_hetzner_includes_checklist() {
        let yaml = "${{ secrets.SSH_PRIVATE_KEY }}";
        let doc = generate_cd_secrets_doc(yaml, &CdPlatform::Hetzner);
        assert!(doc.contains("SSH_PRIVATE_KEY"));
        assert!(doc.contains("Hetzner Prerequisites Checklist"));
        assert!(doc.contains("Port 22"));
        assert!(doc.contains("Docker installed"));
    }

    #[test]
    fn generate_doc_no_secrets() {
        let doc = generate_cd_secrets_doc("just yaml", &CdPlatform::Azure);
        assert!(doc.contains("No CD secrets detected"));
    }

    #[test]
    fn extract_secret_name_valid() {
        assert_eq!(
            extract_secret_name("FOO_BAR }}"),
            Some("FOO_BAR".to_string())
        );
    }

    #[test]
    fn extract_secret_name_empty() {
        assert_eq!(extract_secret_name(" not a name"), None);
    }

    #[test]
    fn hetzner_checklist_content() {
        let checklist = hetzner_prerequisites_checklist();
        assert!(checklist.contains("SSH key"));
        assert!(checklist.contains("Port 6443"));
        assert!(checklist.contains("Docker Compose"));
        assert!(checklist.contains("DNS"));
    }

    #[test]
    fn metadata_azure_known_secret() {
        let (desc, _) = secret_metadata("AZURE_CLIENT_ID", &CdPlatform::Azure);
        assert!(desc.contains("Azure AD"));
    }

    #[test]
    fn metadata_gcp_known_secret() {
        let (desc, _) = secret_metadata("GCP_PROJECT_ID", &CdPlatform::Gcp);
        assert!(desc.contains("GCP project"));
    }

    #[test]
    fn metadata_hetzner_known_secret() {
        let (desc, _) = secret_metadata("SSH_PRIVATE_KEY", &CdPlatform::Hetzner);
        assert!(desc.contains("SSH private key"));
    }

    #[test]
    fn metadata_slack_secret() {
        let (desc, _) = secret_metadata("SLACK_WEBHOOK_URL", &CdPlatform::Azure);
        assert!(desc.contains("Slack"));
    }

    #[test]
    fn metadata_unknown_secret_azure() {
        let (desc, _) = secret_metadata("CUSTOM_SECRET", &CdPlatform::Azure);
        assert!(desc.contains("Azure"));
    }

    #[test]
    fn metadata_unknown_secret_gcp() {
        let (desc, _) = secret_metadata("CUSTOM_SECRET", &CdPlatform::Gcp);
        assert!(desc.contains("GCP"));
    }

    #[test]
    fn metadata_unknown_secret_hetzner() {
        let (desc, _) = secret_metadata("CUSTOM_SECRET", &CdPlatform::Hetzner);
        assert!(desc.contains("Hetzner"));
    }

    #[test]
    fn table_format_has_header() {
        let mut names = BTreeSet::new();
        names.insert("FOO".to_string());
        let table = render_cd_secrets_table(&names, &CdPlatform::Azure);
        assert!(table.contains("| Secret |"));
        assert!(table.contains("| `FOO` |"));
    }

    #[test]
    fn collect_multiple_distinct_secrets() {
        let yaml = "secrets.A and secrets.B and secrets.C";
        let names = collect_cd_secret_names(yaml);
        assert_eq!(names.len(), 3);
    }
}
