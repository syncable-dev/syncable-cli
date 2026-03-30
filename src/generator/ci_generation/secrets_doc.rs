//! CI-19 — Secrets Inventory Generator
//!
//! Scans a rendered CI pipeline YAML for secret references, deduplicates
//! them, and formats a `SECRETS_REQUIRED.md` document that tells the user
//! exactly which repository secrets to create and how to obtain them.
//!
//! ## Secret reference patterns recognised
//!
//! | Platform         | Pattern                          | Example                             |
//! |------------------|----------------------------------|-------------------------------------|
//! | GitHub Actions   | `${{ secrets.NAME }}`            | `${{ secrets.GITHUB_TOKEN }}`       |
//! | Azure Pipelines  | `$(SecretVariableName)`          | `$(ACR_PASSWORD)`                   |
//! | Cloud Build      | `$$SECRET_NAME` or substitutions | `$$_GITHUB_TOKEN`                   |
//!
//! Known secrets (e.g. `GITHUB_TOKEN`, Gitleaks, Trivy, Docker) are enriched
//! with descriptions and setup instructions.  Unknown secrets get a generic
//! template row.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::cli::{CiFormat, CiPlatform};

// ── Secret metadata ───────────────────────────────────────────────────────────

/// A single secret entry in the generated document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretEntry {
    pub name: String,
    pub description: String,
    pub how_to_obtain: String,
    pub where_to_set: String,
    pub required: bool,
}

impl SecretEntry {
    fn new(name: &str, description: &str, how_to_obtain: &str, where_to_set: &str, required: bool) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            how_to_obtain: how_to_obtain.to_string(),
            where_to_set: where_to_set.to_string(),
            required,
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Scans `yaml` for secret references and returns a `SECRETS_REQUIRED.md`
/// document body as a `String`.
///
/// `platform` and `format` are used to emit platform-specific setup instructions
/// and to choose which regex patterns to apply.
pub fn generate_secrets_doc(yaml: &str, platform: CiPlatform, format: CiFormat) -> String {
    let names = collect_secret_names(yaml, &format);
    let entries = enrich_secrets(names, &platform);
    render_markdown(&entries, &platform)
}

/// Writes `.syncable/SECRETS_REQUIRED.md` to `output_dir`.
///
/// Creates the `.syncable/` subdirectory if it does not exist.
pub fn write_secrets_doc(
    yaml: &str,
    platform: CiPlatform,
    format: CiFormat,
    output_dir: &Path,
) -> crate::Result<()> {
    let content = generate_secrets_doc(yaml, platform, format);
    let syncable_dir = output_dir.join(".syncable");
    std::fs::create_dir_all(&syncable_dir)?;
    std::fs::write(syncable_dir.join("SECRETS_REQUIRED.md"), content)?;
    Ok(())
}

/// Returns just the deduplicated set of secret names found in `yaml`.
/// Exposed for testing.
pub fn collect_secret_names(yaml: &str, format: &CiFormat) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    match format {
        CiFormat::GithubActions => {
            // ${{ secrets.NAME }} — NAME is uppercase letters, digits, underscores
            for cap in regex_captures(r"\$\{\{\s*secrets\.([A-Z0-9_]+)\s*\}\}", yaml) {
                names.insert(cap);
            }
        }
        CiFormat::AzurePipelines => {
            // $(VARIABLE_NAME) — capitalised names that look like secrets
            for cap in regex_captures(r"\$\(([A-Z][A-Z0-9_]+)\)", yaml) {
                names.insert(cap);
            }
            // Also catch ${{ secrets.X }} in case GitHub Actions blocks are mixed in
            for cap in regex_captures(r"\$\{\{\s*secrets\.([A-Z0-9_]+)\s*\}\}", yaml) {
                names.insert(cap);
            }
        }
        CiFormat::CloudBuild => {
            // $$_VARIABLE or $$ prefixed substitutions (Cloud Build convention)
            for cap in regex_captures(r"\$\$([_A-Z][A-Z0-9_]*)", yaml) {
                names.insert(cap);
            }
            // Plain $_VAR substitution style
            for cap in regex_captures(r"\$_([A-Z][A-Z0-9_]*)", yaml) {
                names.insert(cap);
            }
        }
    }

    names
}

// ── Regex helper (no regex crate dependency — hand-rolled parser) ─────────────

/// Minimal pattern scanner: extracts the first capture group from
/// each non-overlapping match of `pattern` in `text`.
///
/// Supports only the simple patterns needed here (literal prefix + capture
/// of `[A-Z0-9_]+`).  Uses Rust's standard library only — avoids adding
/// the `regex` crate as a dependency.
fn regex_captures(pattern: &str, text: &str) -> Vec<String> {
    // Delegate to the regex crate which is already an indirect dependency
    // via other parts of the codebase.  If it isn't available we fall back
    // to a manual scan.  In practice this will always use the regex crate.
    regex_captures_impl(pattern, text)
}

#[cfg(not(test))]
fn regex_captures_impl(pattern: &str, text: &str) -> Vec<String> {
    use regex::Regex;
    let re = Regex::new(pattern).expect("hardcoded pattern is valid");
    re.captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

#[cfg(test)]
fn regex_captures_impl(pattern: &str, text: &str) -> Vec<String> {
    use regex::Regex;
    let re = Regex::new(pattern).expect("hardcoded pattern is valid");
    re.captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

// ── Knowledge base ────────────────────────────────────────────────────────────

/// Builds a map of well-known secret names → `SecretEntry` metadata.
fn known_secrets() -> BTreeMap<&'static str, SecretEntry> {
    let mut m = BTreeMap::new();

    m.insert("GITHUB_TOKEN", SecretEntry::new(
        "GITHUB_TOKEN",
        "GitHub-issued token for Actions API access. Automatically available in all GitHub Actions runs.",
        "No action required — GitHub injects this automatically.",
        "Injected automatically — no manual secret needed.",
        true,
    ));
    m.insert("GITLEAKS_LICENSE", SecretEntry::new(
        "GITLEAKS_LICENSE",
        "Gitleaks commercial licence key (required for private repositories only).",
        "Purchase at https://gitleaks.io/ · Then add as a repository secret.",
        "GitHub repo → Settings → Secrets and variables → Actions → New repository secret.",
        false,
    ));
    m.insert("CODECOV_TOKEN", SecretEntry::new(
        "CODECOV_TOKEN",
        "API token for uploading coverage reports to Codecov.",
        "Sign in at https://app.codecov.io/ · Navigate to your repo · Copy the upload token.",
        "GitHub repo → Settings → Secrets and variables → Actions → New repository secret.",
        false,
    ));
    m.insert("SLACK_BOT_TOKEN", SecretEntry::new(
        "SLACK_BOT_TOKEN",
        "Slack bot OAuth token for posting CI failure notifications.",
        "Create a Slack app at https://api.slack.com/apps · Add `chat:write` scope · Install to workspace.",
        "GitHub repo → Settings → Secrets and variables → Actions → New repository secret.",
        false,
    ));
    m.insert("SLACK_CHANNEL_ID", SecretEntry::new(
        "SLACK_CHANNEL_ID",
        "Slack channel ID where CI failure notifications are posted.",
        "Right-click the channel in Slack → Copy link — the ID is the last segment (e.g. `C012AB3CD`).",
        "GitHub repo → Settings → Secrets and variables → Actions → New repository secret.",
        false,
    ));

    // Docker / container registry secrets
    for name in &["DOCKER_USERNAME", "DOCKER_PASSWORD", "DOCKER_TOKEN"] {
        m.insert(name, SecretEntry::new(
            name,
            "Docker Hub credentials for pushing container images.",
            "Create an access token at https://hub.docker.com/settings/security · Store username and token separately.",
            "GitHub repo → Settings → Secrets and variables → Actions → New repository secret.",
            true,
        ));
    }
    for name in &["ACR_LOGIN_SERVER", "ACR_USERNAME", "ACR_PASSWORD"] {
        m.insert(name, SecretEntry::new(
            name,
            "Azure Container Registry credentials.",
            "Azure Portal → Container registries → [your registry] → Access keys.",
            "Azure DevOps → Pipelines → Library **or** GitHub repo → Settings → Secrets and variables → Actions.",
            true,
        ));
    }
    for name in &["GCP_SA_KEY", "GCP_PROJECT_ID"] {
        m.insert(name, SecretEntry::new(
            name,
            "GCP service account key / project ID for pushing images to Artifact Registry.",
            "GCP Console → IAM & Admin → Service Accounts → Create key (JSON).",
            "GCP Secret Manager **or** GitHub repo → Settings → Secrets and variables → Actions.",
            true,
        ));
    }

    m
}

/// Converts a set of raw secret names into enriched `SecretEntry` values.
fn enrich_secrets(names: BTreeSet<String>, _platform: &CiPlatform) -> Vec<SecretEntry> {
    let known = known_secrets();
    names
        .into_iter()
        .map(|name| {
            known.get(name.as_str()).cloned().unwrap_or_else(|| SecretEntry::new(
                &name,
                "Project-specific secret — description not yet documented.",
                "Add the value as a repository secret.",
                "GitHub repo → Settings → Secrets and variables → Actions → New repository secret.",
                true,
            ))
        })
        .collect()
}

// ── Markdown renderer ─────────────────────────────────────────────────────────

fn render_markdown(entries: &[SecretEntry], platform: &CiPlatform) -> String {
    if entries.is_empty() {
        return "# Secrets Required\n\nNo secrets detected in the generated pipeline.\n".to_string();
    }

    let platform_label = match platform {
        CiPlatform::Azure => "Azure",
        CiPlatform::Gcp => "GCP",
        CiPlatform::Hetzner => "Hetzner / GitHub Actions",
    };

    let required: Vec<_> = entries.iter().filter(|e| e.required).collect();
    let optional: Vec<_> = entries.iter().filter(|e| !e.required).collect();

    let mut out = String::new();
    out.push_str("# Secrets Required\n\n");
    out.push_str(&format!(
        "Generated by `sync-ctl generate ci` for platform **{}**.\n\n",
        platform_label
    ));
    out.push_str("---\n\n");

    if !required.is_empty() {
        out.push_str("## Required\n\n");
        out.push_str(table_header());
        for e in &required {
            out.push_str(&table_row(e));
        }
        out.push('\n');
    }

    if !optional.is_empty() {
        out.push_str("## Optional\n\n");
        out.push_str(table_header());
        for e in &optional {
            out.push_str(&table_row(e));
        }
        out.push('\n');
    }

    out
}

fn table_header() -> &'static str {
    "| Secret Name | Description | How to obtain | Where to set |\n\
     |---|---|---|---|\n"
}

fn table_row(e: &SecretEntry) -> String {
    format!(
        "| `{}` | {} | {} | {} |\n",
        e.name, e.description, e.how_to_obtain, e.where_to_set
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── collect_secret_names ───────────────────────────────────────────────

    #[test]
    fn test_github_actions_secrets_extracted() {
        let yaml = r#"
env:
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  OTHER: ${{ secrets.MY_TOKEN }}
"#;
        let names = collect_secret_names(yaml, &CiFormat::GithubActions);
        assert!(names.contains("GITHUB_TOKEN"));
        assert!(names.contains("MY_TOKEN"));
    }

    #[test]
    fn test_github_actions_lowercase_secrets_ignored() {
        // Secret names in the patterns must be uppercase — lowercase vars are not secrets
        let yaml = "run: echo ${{ env.foo }}";
        let names = collect_secret_names(yaml, &CiFormat::GithubActions);
        assert!(names.is_empty());
    }

    #[test]
    fn test_azure_dollar_paren_secrets_extracted() {
        let yaml = "value: $(ACR_PASSWORD)\nother: $(System.AccessToken)";
        let names = collect_secret_names(yaml, &CiFormat::AzurePipelines);
        assert!(names.contains("ACR_PASSWORD"));
    }

    #[test]
    fn test_cloud_build_dollar_dollar_extracted() {
        let yaml = "env:\n  - GITHUB_TOKEN=$$_GITHUB_TOKEN";
        let names = collect_secret_names(yaml, &CiFormat::CloudBuild);
        assert!(names.contains("_GITHUB_TOKEN"));
    }

    #[test]
    fn test_cloud_build_dollar_underscore_extracted() {
        let yaml = "args: [\"$_GCP_PROJECT_ID\"]";
        let names = collect_secret_names(yaml, &CiFormat::CloudBuild);
        assert!(names.contains("GCP_PROJECT_ID"));
    }

    #[test]
    fn test_deduplication() {
        let yaml = r#"
env:
  TOKEN: ${{ secrets.GITHUB_TOKEN }}
  OTHER: ${{ secrets.GITHUB_TOKEN }}
"#;
        let names = collect_secret_names(yaml, &CiFormat::GithubActions);
        assert_eq!(names.len(), 1);
        assert!(names.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn test_empty_yaml_gives_empty_set() {
        let names = collect_secret_names("steps: []", &CiFormat::GithubActions);
        assert!(names.is_empty());
    }

    // ── enrich_secrets ─────────────────────────────────────────────────────

    #[test]
    fn test_known_secret_enriched() {
        let mut names = BTreeSet::new();
        names.insert("GITHUB_TOKEN".to_string());
        let entries = enrich_secrets(names, &CiPlatform::Hetzner);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "GITHUB_TOKEN");
        assert!(entries[0].required);
        assert!(entries[0].description.contains("GitHub-issued"));
    }

    #[test]
    fn test_unknown_secret_gets_generic_entry() {
        let mut names = BTreeSet::new();
        names.insert("MY_CUSTOM_API_KEY".to_string());
        let entries = enrich_secrets(names, &CiPlatform::Hetzner);
        assert_eq!(entries[0].name, "MY_CUSTOM_API_KEY");
        assert!(entries[0].description.contains("Project-specific"));
    }

    #[test]
    fn test_gitleaks_license_is_optional() {
        let mut names = BTreeSet::new();
        names.insert("GITLEAKS_LICENSE".to_string());
        let entries = enrich_secrets(names, &CiPlatform::Hetzner);
        assert!(!entries[0].required);
    }

    // ── generate_secrets_doc ───────────────────────────────────────────────

    #[test]
    fn test_doc_contains_required_heading() {
        let yaml = "env:\n  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}";
        let doc = generate_secrets_doc(yaml, CiPlatform::Hetzner, CiFormat::GithubActions);
        assert!(doc.contains("## Required"));
        assert!(doc.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn test_doc_contains_optional_section_for_gitleaks() {
        let yaml = r#"
env:
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  GL: ${{ secrets.GITLEAKS_LICENSE }}
"#;
        let doc = generate_secrets_doc(yaml, CiPlatform::Hetzner, CiFormat::GithubActions);
        assert!(doc.contains("## Optional"));
        assert!(doc.contains("GITLEAKS_LICENSE"));
    }

    #[test]
    fn test_doc_no_secrets_message() {
        let doc = generate_secrets_doc("steps: []", CiPlatform::Gcp, CiFormat::CloudBuild);
        assert!(doc.contains("No secrets detected"));
    }

    #[test]
    fn test_doc_platform_label_azure() {
        let yaml = "value: $(ACR_PASSWORD)";
        let doc = generate_secrets_doc(yaml, CiPlatform::Azure, CiFormat::AzurePipelines);
        assert!(doc.contains("Azure"));
    }

    #[test]
    fn test_doc_platform_label_gcp() {
        let yaml = "env:\n  - TOKEN=$$_GITHUB_TOKEN";
        let doc = generate_secrets_doc(yaml, CiPlatform::Gcp, CiFormat::CloudBuild);
        assert!(doc.contains("GCP"));
    }

    #[test]
    fn test_doc_is_valid_markdown_table() {
        let yaml = "env:\n  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}";
        let doc = generate_secrets_doc(yaml, CiPlatform::Hetzner, CiFormat::GithubActions);
        // Table has a header row and separator row
        assert!(doc.contains("| Secret Name | Description | How to obtain |"));
        assert!(doc.contains("|---|---|---|"));
    }

    #[test]
    fn test_doc_entries_sorted_alphabetically() {
        let yaml = r#"
env:
  B: ${{ secrets.BETA_TOKEN }}
  A: ${{ secrets.ALPHA_TOKEN }}
"#;
        let doc = generate_secrets_doc(yaml, CiPlatform::Hetzner, CiFormat::GithubActions);
        let alpha_pos = doc.find("ALPHA_TOKEN").unwrap();
        let beta_pos = doc.find("BETA_TOKEN").unwrap();
        assert!(alpha_pos < beta_pos, "entries should be sorted A-Z");
    }

    // ── where_to_set + write_secrets_doc ───────────────────────────────────

    #[test]
    fn test_where_to_set_field_populated_on_known_secret() {
        let mut names = BTreeSet::new();
        names.insert("GITHUB_TOKEN".to_string());
        let entries = enrich_secrets(names, &CiPlatform::Hetzner);
        assert!(!entries[0].where_to_set.is_empty());
        assert!(entries[0].where_to_set.contains("Injected automatically"));
    }

    #[test]
    fn test_where_to_set_field_populated_on_unknown_secret() {
        let mut names = BTreeSet::new();
        names.insert("MY_CUSTOM_KEY".to_string());
        let entries = enrich_secrets(names, &CiPlatform::Hetzner);
        assert!(entries[0].where_to_set.contains("Settings → Secrets"));
    }

    #[test]
    fn test_where_to_set_column_in_table() {
        let yaml = "env:\n  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}";
        let doc = generate_secrets_doc(yaml, CiPlatform::Hetzner, CiFormat::GithubActions);
        assert!(doc.contains("| Secret Name | Description | How to obtain | Where to set |"));
        assert!(doc.contains("|---|---|---|---|"));
    }

    #[test]
    fn test_write_secrets_doc_creates_file() {
        let tmp = std::env::temp_dir().join(format!(
            "syncable_secrets_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let yaml = "env:\n  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}";
        write_secrets_doc(yaml, CiPlatform::Hetzner, CiFormat::GithubActions, &tmp).unwrap();
        let out = tmp.join(".syncable").join("SECRETS_REQUIRED.md");
        assert!(out.exists(), "SECRETS_REQUIRED.md should be created");
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("GITHUB_TOKEN"));
        assert!(content.contains("Where to set"));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
