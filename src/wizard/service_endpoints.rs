//! Service endpoint discovery and env var matching for inter-service linking
//!
//! When deploying service A that calls service B, this module discovers
//! already-deployed services, shows their public URLs, and offers to inject
//! them as environment variables.

use crate::platform::api::types::{CloudRunnerNetwork, DeployedService, DeploymentSecretInput};
use crate::wizard::render::wizard_render_config;
use colored::Colorize;
use inquire::{Confirm, InquireError, MultiSelect, Text};

/// A deployed service with a reachable URL (public or private network).
#[derive(Debug, Clone)]
pub struct AvailableServiceEndpoint {
    pub service_name: String,
    /// The URL to use for connecting — either public URL or private IP.
    pub url: String,
    /// Whether this endpoint is a private network address (no public URL).
    pub is_private: bool,
    /// Cloud provider this service runs on (e.g. "hetzner", "gcp", "azure").
    pub cloud_provider: Option<String>,
    pub status: String,
}

/// Confidence level for an env-var-to-service match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchConfidence {
    Low,
    Medium,
    High,
}

/// A suggested mapping: env var -> deployed service URL.
#[derive(Debug, Clone)]
pub struct EndpointSuggestion {
    pub env_var_name: String,
    pub service: AvailableServiceEndpoint,
    pub confidence: MatchConfidence,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Suffixes that indicate a URL-like env var
// ---------------------------------------------------------------------------

const URL_SUFFIXES: &[&str] = &[
    "_URL",
    "_SERVICE_URL",
    "_ENDPOINT",
    "_HOST",
    "_BASE",
    "_BASE_URL",
    "_API_URL",
    "_URI",
];

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

/// Filter deployments down to services that have a reachable URL (public or
/// private) and are not in a known-bad state.
///
/// The `list_deployments` API may return multiple records per service (one per
/// deploy attempt). We deduplicate by `service_name`, keeping the most recent
/// record (the API returns most-recent-first).
///
/// A service is included if it has a `public_url` OR a `private_ip` (for
/// internal services deployed on a private network without public access).
pub fn get_available_endpoints(deployments: &[DeployedService]) -> Vec<AvailableServiceEndpoint> {
    const EXCLUDED_STATUSES: &[&str] = &[
        "failed",
        "cancelled",
        "canceled",
        "pending",
        "processing",
        "building",
        "deploying",
        "generating",
        "deleting",
        "deleted",
    ];

    let mut seen_services = std::collections::HashSet::new();

    deployments
        .iter()
        .filter_map(|d| {
            // Deduplicate: keep only the first (most recent) record per service
            if !seen_services.insert(d.service_name.clone()) {
                return None;
            }

            let status_lower = d.status.to_lowercase();
            if EXCLUDED_STATUSES.iter().any(|&s| status_lower == s) {
                log::debug!(
                    "Skipping service '{}' (status: {}): excluded status",
                    d.service_name,
                    d.status
                );
                return None;
            }

            // Prefer public URL; fall back to private IP
            let public_url = d.public_url.as_deref().unwrap_or("").trim();
            let private_ip = d.private_ip.as_deref().unwrap_or("").trim();

            if !public_url.is_empty() {
                log::debug!(
                    "Available endpoint: '{}' -> {} (public, status: {})",
                    d.service_name,
                    public_url,
                    d.status
                );
                Some(AvailableServiceEndpoint {
                    service_name: d.service_name.clone(),
                    url: public_url.to_string(),
                    is_private: false,
                    cloud_provider: d.cloud_provider.clone(),
                    status: d.status.clone(),
                })
            } else if !private_ip.is_empty() {
                // Build a usable URL from the private IP.
                // Services on Hetzner private networks are reachable by IP from
                // other services on the same network.
                let url = format!("http://{}", private_ip);
                log::debug!(
                    "Available endpoint: '{}' -> {} (private, status: {})",
                    d.service_name,
                    url,
                    d.status
                );
                Some(AvailableServiceEndpoint {
                    service_name: d.service_name.clone(),
                    url,
                    is_private: true,
                    cloud_provider: d.cloud_provider.clone(),
                    status: d.status.clone(),
                })
            } else {
                log::debug!(
                    "Skipping service '{}' (status: {}): no public_url or private_ip",
                    d.service_name,
                    d.status
                );
                None
            }
        })
        .collect()
}

/// Filter endpoints so that private-network endpoints only appear when they
/// share the same cloud provider as the service being deployed.
///
/// Public endpoints are always kept regardless of provider — they're reachable
/// from anywhere. Private endpoints are only reachable within the same provider
/// network.
pub fn filter_endpoints_for_provider(
    endpoints: Vec<AvailableServiceEndpoint>,
    target_provider: &str,
) -> Vec<AvailableServiceEndpoint> {
    let target = target_provider.to_lowercase();
    endpoints
        .into_iter()
        .filter(|ep| {
            if !ep.is_private {
                // Public URLs are reachable from any provider
                return true;
            }
            // Private IPs are only useful on the same provider network
            ep.cloud_provider
                .as_ref()
                .map(|p| p.to_lowercase() == target)
                .unwrap_or(false)
        })
        .collect()
}

/// Check whether an env var name looks like it holds a URL.
pub fn is_url_env_var(name: &str) -> bool {
    let upper = name.to_uppercase();
    URL_SUFFIXES.iter().any(|suffix| upper.ends_with(suffix))
}

/// Strip the URL-like suffix from an env var name to extract a service hint.
///
/// `SENTIMENT_SERVICE_URL` -> `"sentiment"`
/// `API_BASE` -> `"api"`
/// `NODE_ENV` -> `None`
pub fn extract_service_hint(env_var_name: &str) -> Option<String> {
    let upper = env_var_name.to_uppercase();

    // Try suffixes longest-first so _SERVICE_URL is tried before _URL
    let mut suffixes: Vec<&&str> = URL_SUFFIXES.iter().collect();
    suffixes.sort_by(|a, b| b.len().cmp(&a.len()));

    for suffix in suffixes {
        if upper.ends_with(suffix) {
            let prefix = &upper[..upper.len() - suffix.len()];
            if prefix.is_empty() {
                return None;
            }
            return Some(prefix.to_lowercase());
        }
    }
    None
}

/// Normalize a name for matching: lowercase, strip `-` and `_`.
fn normalize(s: &str) -> String {
    s.to_lowercase().replace(['-', '_'], "")
}

/// Split a name into tokens on `_` and `-`.
fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| c == '_' || c == '-')
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

/// Match a service hint against a service name.
///
/// Returns `None` if there is no meaningful overlap.
pub fn match_hint_to_service(hint: &str, service_name: &str) -> Option<MatchConfidence> {
    let nh = normalize(hint);
    let ns = normalize(service_name);

    if nh.is_empty() || ns.is_empty() {
        return None;
    }

    // Exact match or hint is prefix of service (normalized)
    if nh == ns || ns.starts_with(&nh) {
        return Some(MatchConfidence::High);
    }

    // One contains the other (normalized, no separators)
    if ns.contains(&nh) || nh.contains(&ns) {
        return Some(MatchConfidence::Medium);
    }

    // Check if either normalized form is a prefix of the other
    // (catches "contacts" ~ "contactintelligence" via shared stem)
    if nh.starts_with(&ns) || ns.starts_with(&nh) {
        return Some(MatchConfidence::Medium);
    }

    // Token overlap: exact or prefix match between tokens
    let hint_tokens = tokenize(hint);
    let svc_tokens = tokenize(service_name);
    let overlap = hint_tokens
        .iter()
        .filter(|ht| {
            svc_tokens.iter().any(|st| {
                st == *ht || st.starts_with(ht.as_str()) || ht.starts_with(st.as_str())
            })
        })
        .count();

    if overlap == 0 {
        return None;
    }

    let max_tokens = hint_tokens.len().max(svc_tokens.len());
    if overlap * 2 >= max_tokens {
        Some(MatchConfidence::Medium)
    } else {
        Some(MatchConfidence::Low)
    }
}

/// For each URL-like env var, find the best matching deployed service.
///
/// Returns suggestions sorted by confidence (highest first).
pub fn match_env_vars_to_services(
    env_var_names: &[String],
    endpoints: &[AvailableServiceEndpoint],
) -> Vec<EndpointSuggestion> {
    let mut suggestions = Vec::new();

    for var_name in env_var_names {
        if !is_url_env_var(var_name) {
            continue;
        }
        let hint = match extract_service_hint(var_name) {
            Some(h) => h,
            None => continue,
        };

        // Find best match
        let mut best: Option<(MatchConfidence, &AvailableServiceEndpoint)> = None;
        for ep in endpoints {
            if let Some(conf) = match_hint_to_service(&hint, &ep.service_name) {
                if best.as_ref().map_or(true, |(bc, _)| conf > *bc) {
                    best = Some((conf, ep));
                }
            }
        }

        if let Some((confidence, ep)) = best {
            suggestions.push(EndpointSuggestion {
                env_var_name: var_name.clone(),
                service: ep.clone(),
                confidence,
                reason: format!(
                    "Env var '{}' (hint '{}') matches service '{}' ({:?})",
                    var_name, hint, ep.service_name, confidence
                ),
            });
        }
    }

    suggestions.sort_by(|a, b| b.confidence.cmp(&a.confidence));
    suggestions
}

/// Generate a default env var name for a service.
///
/// `"sentiment-analysis"` -> `"SENTIMENT_ANALYSIS_URL"`
pub fn suggest_env_var_name(service_name: &str) -> String {
    let base = service_name
        .to_uppercase()
        .replace('-', "_");
    format!("{}_URL", base)
}

// ---------------------------------------------------------------------------
// Wizard UI
// ---------------------------------------------------------------------------

/// Interactive prompt to link deployed service URLs as env vars.
///
/// Shows available endpoints, lets the user select which to link, and
/// prompts for each env var name. Returns `DeploymentSecretInput` entries
/// with `is_secret: false` (URLs are not secrets).
pub fn collect_service_endpoint_env_vars(
    endpoints: &[AvailableServiceEndpoint],
) -> Vec<DeploymentSecretInput> {
    if endpoints.is_empty() {
        return Vec::new();
    }

    println!();
    println!(
        "{}",
        "─── Deployed Service Endpoints ────────────────────".dimmed()
    );
    println!(
        "  Found {} running service(s) with reachable URLs:",
        endpoints.len().to_string().cyan()
    );
    for ep in endpoints {
        let access_label = if ep.is_private { " (private network)" } else { "" };
        println!(
            "    {} {:<30} {}{}",
            "●".green(),
            ep.service_name.cyan(),
            ep.url.dimmed(),
            access_label.yellow()
        );
    }
    println!();

    // Ask if user wants to link any
    let wants_link = match Confirm::new("Link any deployed service URLs as env vars?")
        .with_default(true)
        .with_help_message("Inject deployed service URLs as environment variables")
        .prompt()
    {
        Ok(v) => v,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    };

    if !wants_link {
        return Vec::new();
    }

    // Build labels for multi-select
    let labels: Vec<String> = endpoints
        .iter()
        .map(|ep| {
            let suffix = if ep.is_private { " [private]" } else { "" };
            format!("{} ({}){}", ep.service_name, ep.url, suffix)
        })
        .collect();

    let selected = match MultiSelect::new("Select services to link:", labels.clone())
        .with_render_config(wizard_render_config())
        .with_help_message("Space to toggle, Enter to confirm")
        .prompt()
    {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => return Vec::new(),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    };

    // Map selected labels back to endpoints
    let mut result = Vec::new();
    for sel_label in &selected {
        let idx = match labels.iter().position(|l| l == sel_label) {
            Some(i) => i,
            None => continue,
        };
        let ep = &endpoints[idx];
        let default_name = suggest_env_var_name(&ep.service_name);

        let var_name = match Text::new(&format!("Env var name for '{}':", ep.service_name))
            .with_default(&default_name)
            .with_help_message("Environment variable name to hold this service URL")
            .prompt()
        {
            Ok(name) => name.trim().to_uppercase(),
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                break;
            }
            Err(_) => break,
        };

        if var_name.is_empty() {
            continue;
        }

        let private_note = if ep.is_private { " (private network)" } else { "" };
        println!(
            "  {} {} = {}{}",
            "✓".green(),
            var_name.cyan(),
            ep.url.dimmed(),
            private_note.yellow()
        );

        result.push(DeploymentSecretInput {
            key: var_name,
            value: ep.url.clone(),
            is_secret: false,
        });
    }

    result
}

// ---------------------------------------------------------------------------
// Network endpoint discovery
// ---------------------------------------------------------------------------

/// A network resource with its connection-relevant details.
///
/// Extracted from `CloudRunnerNetwork` records, filtered for the target
/// provider and environment. Contains key-value pairs of useful connection
/// info (VPC_ID, DEFAULT_DOMAIN, etc.) that can be injected as env vars.
#[derive(Debug, Clone)]
pub struct NetworkEndpointInfo {
    pub network_id: String,
    pub cloud_provider: String,
    pub region: String,
    pub status: String,
    pub environment_id: Option<String>,
    /// Key-value pairs of useful connection info for this network
    /// e.g., ("NETWORK_VPC_ID", "12345"), ("NETWORK_DEFAULT_DOMAIN", "my-app.azurecontainerapps.io")
    pub connection_details: Vec<(String, String)>,
}

/// Extract useful connection details from cloud runner networks.
///
/// Returns only networks that are "ready" and on the target provider.
/// Optionally filters by environment ID (shared/default networks with no
/// environment_id are always included).
pub fn extract_network_endpoints(
    networks: &[CloudRunnerNetwork],
    target_provider: &str,
    target_environment_id: Option<&str>,
) -> Vec<NetworkEndpointInfo> {
    networks
        .iter()
        .filter(|n| {
            n.status == "ready"
                && n.cloud_provider.eq_ignore_ascii_case(target_provider)
                && (target_environment_id.is_none()
                    || n.environment_id.as_deref() == target_environment_id
                    || n.environment_id.is_none()) // shared/default networks
        })
        .map(|n| {
            let mut details = Vec::new();

            // Provider-generic connection details
            if let Some(ref vpc_id) = n.vpc_id {
                details.push(("NETWORK_VPC_ID".to_string(), vpc_id.clone()));
            }
            if let Some(ref vpc_name) = n.vpc_name {
                details.push(("NETWORK_VPC_NAME".to_string(), vpc_name.clone()));
            }
            if let Some(ref subnet_id) = n.subnet_id {
                details.push(("NETWORK_SUBNET_ID".to_string(), subnet_id.clone()));
            }
            // Azure-specific
            if let Some(ref cae_name) = n.container_app_environment_name {
                details.push((
                    "AZURE_CONTAINER_APP_ENV_NAME".to_string(),
                    cae_name.clone(),
                ));
            }
            if let Some(ref domain) = n.default_domain {
                details.push(("NETWORK_DEFAULT_DOMAIN".to_string(), domain.clone()));
            }
            if let Some(ref rg) = n.resource_group_name {
                details.push(("AZURE_RESOURCE_GROUP".to_string(), rg.clone()));
            }
            // GCP-specific
            if let Some(ref connector_name) = n.vpc_connector_name {
                details.push(("GCP_VPC_CONNECTOR".to_string(), connector_name.clone()));
            }

            NetworkEndpointInfo {
                network_id: n.id.clone(),
                cloud_provider: n.cloud_provider.clone(),
                region: n.region.clone(),
                status: n.status.clone(),
                environment_id: n.environment_id.clone(),
                connection_details: details,
            }
        })
        .collect()
}

/// Interactive prompt to offer network connection details as env vars.
///
/// Shows discovered network info and lets the user select which to inject.
/// Returns `DeploymentSecretInput` entries with `is_secret: false` (network
/// identifiers are infrastructure metadata, not secrets).
pub fn collect_network_endpoint_env_vars(
    network_endpoints: &[NetworkEndpointInfo],
) -> Vec<DeploymentSecretInput> {
    if network_endpoints.is_empty() {
        return Vec::new();
    }

    // Flatten all connection details across networks
    let all_details: Vec<(&NetworkEndpointInfo, &str, &str)> = network_endpoints
        .iter()
        .flat_map(|ne| {
            ne.connection_details
                .iter()
                .map(move |(k, v)| (ne, k.as_str(), v.as_str()))
        })
        .collect();

    if all_details.is_empty() {
        return Vec::new();
    }

    println!();
    println!(
        "{}",
        "─── Private Network Resources ────────────────────".dimmed()
    );
    for ne in network_endpoints {
        println!(
            "  {} {} network in {} ({})",
            "●".green(),
            ne.cloud_provider.cyan(),
            ne.region,
            ne.status,
        );
        for (k, v) in &ne.connection_details {
            println!("    {} = {}", k.dimmed(), v);
        }
    }
    println!();

    let wants_inject = match Confirm::new("Inject any network details as env vars?")
        .with_default(false)
        .with_help_message("Add network identifiers like VPC_ID, DEFAULT_DOMAIN as env vars")
        .prompt()
    {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    if !wants_inject {
        return Vec::new();
    }

    let labels: Vec<String> = all_details
        .iter()
        .map(|(ne, k, v)| format!("{} = {} [{}]", k, v, ne.cloud_provider))
        .collect();

    let selected = match MultiSelect::new("Select network details to inject:", labels.clone())
        .with_render_config(wizard_render_config())
        .with_help_message("Space to toggle, Enter to confirm")
        .prompt()
    {
        Ok(s) if !s.is_empty() => s,
        _ => return Vec::new(),
    };

    selected
        .iter()
        .filter_map(|label| {
            let idx = labels.iter().position(|l| l == label)?;
            let (_, key, value) = &all_details[idx];
            Some(DeploymentSecretInput {
                key: key.to_string(),
                value: value.to_string(),
                is_secret: false,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_service_hint() {
        assert_eq!(
            extract_service_hint("SENTIMENT_SERVICE_URL"),
            Some("sentiment".to_string())
        );
        assert_eq!(
            extract_service_hint("API_BASE"),
            Some("api".to_string())
        );
        assert_eq!(extract_service_hint("NODE_ENV"), None);
        assert_eq!(
            extract_service_hint("CONTACTS_API_URL"),
            Some("contacts".to_string())
        );
        assert_eq!(
            extract_service_hint("BACKEND_ENDPOINT"),
            Some("backend".to_string())
        );
    }

    #[test]
    fn test_match_hint_exact() {
        assert_eq!(
            match_hint_to_service("sentiment", "sentiment"),
            Some(MatchConfidence::High)
        );
    }

    #[test]
    fn test_match_hint_prefix() {
        assert_eq!(
            match_hint_to_service("sentiment", "sentiment-analysis"),
            Some(MatchConfidence::High)
        );
    }

    #[test]
    fn test_match_hint_containment() {
        assert_eq!(
            match_hint_to_service("contacts", "contact-intelligence"),
            Some(MatchConfidence::Medium)
        );
    }

    #[test]
    fn test_no_match() {
        assert_eq!(
            match_hint_to_service("database", "sentiment-analysis"),
            None
        );
    }

    #[test]
    fn test_is_url_env_var() {
        assert!(is_url_env_var("DATABASE_URL"));
        assert!(is_url_env_var("BACKEND_SERVICE_URL"));
        assert!(is_url_env_var("API_ENDPOINT"));
        assert!(is_url_env_var("SERVICE_HOST"));
        assert!(is_url_env_var("API_BASE"));
        assert!(is_url_env_var("APP_BASE_URL"));
        assert!(is_url_env_var("BACKEND_API_URL"));
        assert!(is_url_env_var("SERVICE_URI"));
        assert!(!is_url_env_var("NODE_ENV"));
        assert!(!is_url_env_var("PORT"));
        assert!(!is_url_env_var("DEBUG"));
    }

    #[test]
    fn test_suggest_env_var_name() {
        assert_eq!(
            suggest_env_var_name("sentiment-analysis"),
            "SENTIMENT_ANALYSIS_URL"
        );
        assert_eq!(suggest_env_var_name("backend"), "BACKEND_URL");
        assert_eq!(
            suggest_env_var_name("contact-intelligence"),
            "CONTACT_INTELLIGENCE_URL"
        );
    }

    #[test]
    fn test_match_env_vars_to_services() {
        let endpoints = vec![
            AvailableServiceEndpoint {
                service_name: "sentiment-analysis".to_string(),
                url: "https://sentiment-abc.syncable.dev".to_string(),
                is_private: false,
                cloud_provider: Some("hetzner".to_string()),
                status: "running".to_string(),
            },
            AvailableServiceEndpoint {
                service_name: "contact-intelligence".to_string(),
                url: "https://contact-def.syncable.dev".to_string(),
                is_private: false,
                cloud_provider: Some("hetzner".to_string()),
                status: "running".to_string(),
            },
        ];

        let env_vars = vec![
            "SENTIMENT_SERVICE_URL".to_string(),
            "CONTACTS_API_URL".to_string(),
            "NODE_ENV".to_string(), // not a URL var
            "DATABASE_URL".to_string(), // no matching service
        ];

        let suggestions = match_env_vars_to_services(&env_vars, &endpoints);

        // SENTIMENT_SERVICE_URL should match sentiment-analysis
        let sent = suggestions
            .iter()
            .find(|s| s.env_var_name == "SENTIMENT_SERVICE_URL");
        assert!(sent.is_some());
        assert_eq!(sent.unwrap().service.service_name, "sentiment-analysis");
        assert_eq!(sent.unwrap().confidence, MatchConfidence::High);

        // CONTACTS_API_URL should match contact-intelligence
        let cont = suggestions
            .iter()
            .find(|s| s.env_var_name == "CONTACTS_API_URL");
        assert!(cont.is_some());
        assert_eq!(cont.unwrap().service.service_name, "contact-intelligence");

        // NODE_ENV should not be in suggestions (not a URL var)
        assert!(suggestions
            .iter()
            .all(|s| s.env_var_name != "NODE_ENV"));
    }

    #[test]
    fn test_get_available_endpoints() {
        use crate::platform::api::types::DeployedService;
        use chrono::Utc;

        let deployments = vec![
            DeployedService {
                id: "1".to_string(),
                project_id: "p1".to_string(),
                service_name: "running-svc".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "running".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: Some("https://running.example.com".to_string()),
                private_ip: None,
                cloud_provider: None,
                created_at: Utc::now(),
            },
            DeployedService {
                id: "2".to_string(),
                project_id: "p1".to_string(),
                service_name: "no-url-svc".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "running".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: None,
                private_ip: None,
                cloud_provider: None,
                created_at: Utc::now(),
            },
            DeployedService {
                id: "3".to_string(),
                project_id: "p1".to_string(),
                service_name: "failed-svc".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "failed".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: Some("https://failed.example.com".to_string()),
                private_ip: None,
                cloud_provider: None,
                created_at: Utc::now(),
            },
            DeployedService {
                id: "4".to_string(),
                project_id: "p1".to_string(),
                service_name: "healthy-svc".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "healthy".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: Some("https://healthy.example.com".to_string()),
                private_ip: None,
                cloud_provider: None,
                created_at: Utc::now(),
            },
        ];

        let endpoints = get_available_endpoints(&deployments);
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].service_name, "running-svc");
        assert_eq!(endpoints[1].service_name, "healthy-svc");
    }

    #[test]
    fn test_get_available_endpoints_includes_private_ip() {
        use crate::platform::api::types::DeployedService;
        use chrono::Utc;

        let deployments = vec![
            DeployedService {
                id: "1".to_string(),
                project_id: "p1".to_string(),
                service_name: "public-svc".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "healthy".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: Some("https://public.example.com".to_string()),
                private_ip: Some("10.0.0.2".to_string()),
                cloud_provider: Some("hetzner".to_string()),
                created_at: Utc::now(),
            },
            DeployedService {
                id: "2".to_string(),
                project_id: "p1".to_string(),
                service_name: "internal-svc".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "healthy".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: None,
                private_ip: Some("10.0.0.3".to_string()),
                cloud_provider: Some("hetzner".to_string()),
                created_at: Utc::now(),
            },
            DeployedService {
                id: "3".to_string(),
                project_id: "p1".to_string(),
                service_name: "ghost-svc".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "healthy".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: None,
                private_ip: None,
                cloud_provider: None,
                created_at: Utc::now(),
            },
        ];

        let endpoints = get_available_endpoints(&deployments);
        assert_eq!(endpoints.len(), 2);

        // Public service uses public URL, not private IP
        assert_eq!(endpoints[0].service_name, "public-svc");
        assert_eq!(endpoints[0].url, "https://public.example.com");
        assert!(!endpoints[0].is_private);

        // Internal service uses private IP
        assert_eq!(endpoints[1].service_name, "internal-svc");
        assert_eq!(endpoints[1].url, "http://10.0.0.3");
        assert!(endpoints[1].is_private);
    }

    #[test]
    fn test_get_available_endpoints_deduplicates() {
        use crate::platform::api::types::DeployedService;
        use chrono::Utc;

        // Simulate API returning two records for same service (most recent first)
        let deployments = vec![
            DeployedService {
                id: "2".to_string(),
                project_id: "p1".to_string(),
                service_name: "backend".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "running".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: Some("https://backend.example.com".to_string()),
                private_ip: None,
                cloud_provider: None,
                created_at: Utc::now(),
            },
            DeployedService {
                id: "1".to_string(),
                project_id: "p1".to_string(),
                service_name: "backend".to_string(),
                repository_full_name: "org/repo".to_string(),
                status: "failed".to_string(),
                backstage_task_id: None,
                commit_sha: None,
                public_url: Some("https://backend-old.example.com".to_string()),
                private_ip: None,
                cloud_provider: None,
                created_at: Utc::now(),
            },
        ];

        let endpoints = get_available_endpoints(&deployments);
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].url, "https://backend.example.com");
    }

    #[test]
    fn test_get_available_endpoints_accepts_unknown_statuses() {
        use crate::platform::api::types::DeployedService;
        use chrono::Utc;

        // A service with an unexpected status but a public URL should be included
        let deployments = vec![DeployedService {
            id: "1".to_string(),
            project_id: "p1".to_string(),
            service_name: "api-svc".to_string(),
            repository_full_name: "org/repo".to_string(),
            status: "succeeded".to_string(),
            backstage_task_id: None,
            commit_sha: None,
            public_url: Some("https://api.example.com".to_string()),
            private_ip: None,
            cloud_provider: None,
            created_at: Utc::now(),
        }];

        let endpoints = get_available_endpoints(&deployments);
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].service_name, "api-svc");
    }

    #[test]
    fn test_filter_endpoints_for_provider() {
        let endpoints = vec![
            // Public endpoint on Azure — should always be kept
            AvailableServiceEndpoint {
                service_name: "azure-api".to_string(),
                url: "https://azure-api.example.com".to_string(),
                is_private: false,
                cloud_provider: Some("azure".to_string()),
                status: "healthy".to_string(),
            },
            // Private endpoint on Hetzner — should be kept when deploying to Hetzner
            AvailableServiceEndpoint {
                service_name: "hetzner-worker".to_string(),
                url: "http://10.0.0.5".to_string(),
                is_private: true,
                cloud_provider: Some("hetzner".to_string()),
                status: "healthy".to_string(),
            },
            // Private endpoint on Azure — should NOT be kept when deploying to Hetzner
            AvailableServiceEndpoint {
                service_name: "azure-internal".to_string(),
                url: "http://10.1.0.5".to_string(),
                is_private: true,
                cloud_provider: Some("azure".to_string()),
                status: "healthy".to_string(),
            },
        ];

        // Deploying to Hetzner: keep public endpoints + Hetzner private only
        let filtered = filter_endpoints_for_provider(endpoints.clone(), "hetzner");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].service_name, "azure-api"); // public, always kept
        assert_eq!(filtered[1].service_name, "hetzner-worker"); // same provider

        // Deploying to Azure: keep public endpoints + Azure private only
        let filtered = filter_endpoints_for_provider(endpoints, "azure");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].service_name, "azure-api"); // public
        assert_eq!(filtered[1].service_name, "azure-internal"); // same provider
    }

    // =========================================================================
    // Network endpoint tests
    // =========================================================================

    fn make_network(
        id: &str,
        provider: &str,
        region: &str,
        status: &str,
        env_id: Option<&str>,
    ) -> CloudRunnerNetwork {
        CloudRunnerNetwork {
            id: id.to_string(),
            project_id: "proj-1".to_string(),
            organization_id: "org-1".to_string(),
            environment_id: env_id.map(String::from),
            cloud_provider: provider.to_string(),
            region: region.to_string(),
            vpc_id: None,
            vpc_name: None,
            subnet_id: None,
            vpc_connector_id: None,
            vpc_connector_name: None,
            resource_group_name: None,
            container_app_environment_id: None,
            container_app_environment_name: None,
            default_domain: None,
            status: status.to_string(),
            error_message: None,
        }
    }

    #[test]
    fn test_extract_network_endpoints_filters_by_provider_and_status() {
        let networks = vec![
            {
                let mut n = make_network("n1", "hetzner", "nbg1", "ready", Some("env-1"));
                n.vpc_id = Some("vpc-123".to_string());
                n.subnet_id = Some("subnet-456".to_string());
                n
            },
            // Different provider — should be excluded
            {
                let mut n = make_network("n2", "gcp", "us-central1", "ready", Some("env-1"));
                n.vpc_connector_name = Some("my-connector".to_string());
                n
            },
            // Same provider but not ready — should be excluded
            {
                let mut n = make_network("n3", "hetzner", "fsn1", "provisioning", Some("env-1"));
                n.vpc_id = Some("vpc-789".to_string());
                n
            },
        ];

        let endpoints = extract_network_endpoints(&networks, "hetzner", Some("env-1"));
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].network_id, "n1");
        assert_eq!(endpoints[0].cloud_provider, "hetzner");
        assert_eq!(endpoints[0].connection_details.len(), 2);
        assert!(endpoints[0]
            .connection_details
            .iter()
            .any(|(k, v)| k == "NETWORK_VPC_ID" && v == "vpc-123"));
        assert!(endpoints[0]
            .connection_details
            .iter()
            .any(|(k, v)| k == "NETWORK_SUBNET_ID" && v == "subnet-456"));
    }

    #[test]
    fn test_extract_network_endpoints_azure() {
        let networks = vec![{
            let mut n = make_network("n1", "azure", "eastus", "ready", Some("env-1"));
            n.container_app_environment_name = Some("my-cae".to_string());
            n.default_domain = Some("my-app.azurecontainerapps.io".to_string());
            n.resource_group_name = Some("rg-prod".to_string());
            n
        }];

        let endpoints = extract_network_endpoints(&networks, "azure", Some("env-1"));
        assert_eq!(endpoints.len(), 1);
        assert!(endpoints[0]
            .connection_details
            .iter()
            .any(|(k, v)| k == "AZURE_CONTAINER_APP_ENV_NAME" && v == "my-cae"));
        assert!(endpoints[0]
            .connection_details
            .iter()
            .any(|(k, v)| k == "NETWORK_DEFAULT_DOMAIN"
                && v == "my-app.azurecontainerapps.io"));
        assert!(endpoints[0]
            .connection_details
            .iter()
            .any(|(k, v)| k == "AZURE_RESOURCE_GROUP" && v == "rg-prod"));
    }

    #[test]
    fn test_extract_network_endpoints_hetzner() {
        let networks = vec![{
            let mut n = make_network("n1", "hetzner", "nbg1", "ready", None);
            n.vpc_id = Some("hetz-vpc-1".to_string());
            n.subnet_id = Some("hetz-sub-1".to_string());
            n
        }];

        let endpoints = extract_network_endpoints(&networks, "hetzner", Some("env-1"));
        // Shared network (no environment_id) should be included
        assert_eq!(endpoints.len(), 1);
        assert!(endpoints[0]
            .connection_details
            .iter()
            .any(|(k, v)| k == "NETWORK_VPC_ID" && v == "hetz-vpc-1"));
        assert!(endpoints[0]
            .connection_details
            .iter()
            .any(|(k, v)| k == "NETWORK_SUBNET_ID" && v == "hetz-sub-1"));
    }

    #[test]
    fn test_extract_network_endpoints_gcp() {
        let networks = vec![{
            let mut n = make_network("n1", "gcp", "us-central1", "ready", Some("env-1"));
            n.vpc_connector_name = Some("projects/my-proj/locations/us-central1/connectors/vpc-conn".to_string());
            n
        }];

        let endpoints = extract_network_endpoints(&networks, "gcp", Some("env-1"));
        assert_eq!(endpoints.len(), 1);
        assert!(endpoints[0].connection_details.iter().any(|(k, v)| k
            == "GCP_VPC_CONNECTOR"
            && v == "projects/my-proj/locations/us-central1/connectors/vpc-conn"));
    }

    #[test]
    fn test_extract_network_endpoints_filters_non_ready() {
        let networks = vec![
            {
                let mut n = make_network("n1", "hetzner", "nbg1", "error", Some("env-1"));
                n.vpc_id = Some("vpc-err".to_string());
                n
            },
            {
                let mut n = make_network("n2", "hetzner", "nbg1", "provisioning", Some("env-1"));
                n.vpc_id = Some("vpc-prov".to_string());
                n
            },
        ];

        let endpoints = extract_network_endpoints(&networks, "hetzner", Some("env-1"));
        assert!(endpoints.is_empty());
    }
}
