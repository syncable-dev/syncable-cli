//! CI-17 — Multi-Version Test Matrix Generator
//!
//! Maps a project's declared runtime version range to a concrete list of
//! LTS/stable versions that should be tested, then renders a GitHub Actions
//! `strategy.matrix` YAML fragment.
//!
//! ## Supported languages and version sources
//!
//! | Language | Version source in `CiContext.runtime_versions` |
//! |----------|------------------------------------------------|
//! | Node.js  | `engines.node` from package.json (semver range) |
//! | Python   | `python_requires` from pyproject.toml / setup.cfg |
//! | Go       | `go` directive in go.mod (exact or `~1.x`)       |
//! | Rust     | `rust-toolchain.toml` channel (`stable`, `1.x`)  |
//! | Java     | `source_compatibility` / `java.version` in pom.xml |
//!
//! When a version constraint does not match any known LTS, the module falls
//! back to the detected version string as a single-element list, ensuring
//! the matrix is never empty.

use crate::generator::ci_generation::context::CiContext;

// ── Public types ──────────────────────────────────────────────────────────────

/// A resolved version matrix for a specific language, ready to embed in a
/// GitHub Actions workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMatrix {
    /// Language label (e.g. `"node"`, `"python"`, `"go"`).
    pub language: String,
    /// Concrete versions to test (e.g. `["18", "20", "22"]`).
    pub versions: Vec<String>,
    /// Rendered `strategy:` YAML block.
    pub rendered_yaml: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns a `VersionMatrix` for the project's primary language when at least
/// two distinct LTS versions are identified.  Returns `None` for languages
/// with only a single relevant version or no version information.
pub fn generate_version_matrix(ctx: &CiContext) -> Option<VersionMatrix> {
    let lang = ctx.primary_language.to_lowercase();
    let key = runtime_key(&lang);
    let constraint = ctx.runtime_versions.get(key).map(|s| s.as_str()).unwrap_or("");

    let versions = expand_versions(&lang, constraint);
    if versions.len() < 2 {
        return None;
    }

    let rendered_yaml = render_matrix_yaml(&lang, &versions);
    Some(VersionMatrix {
        language: lang,
        versions,
        rendered_yaml,
    })
}

/// Expands a version constraint string to a list of concrete LTS / stable
/// version strings.  Exposed for testing.
pub fn expand_versions(language: &str, constraint: &str) -> Vec<String> {
    match language {
        "node" | "node.js" | "javascript" | "typescript" => expand_node(constraint),
        "python" => expand_python(constraint),
        "go" => expand_go(constraint),
        "rust" => expand_rust(constraint),
        "java" | "kotlin" => expand_java(constraint),
        _ => {
            if constraint.is_empty() {
                vec![]
            } else {
                vec![constraint.to_string()]
            }
        }
    }
}

// ── Language-specific expanders ───────────────────────────────────────────────

/// Node.js LTS versions (even majors ≥ 18 are Active/Maintenance LTS).
static NODE_LTS: &[&str] = &["18", "20", "22"];

fn expand_node(constraint: &str) -> Vec<String> {
    if constraint.is_empty() {
        // No constraint → test all current LTS
        return NODE_LTS.iter().map(|s| s.to_string()).collect();
    }
    let min = parse_semver_lower_bound(constraint).unwrap_or(0);
    let upper = parse_semver_upper_bound(constraint);
    NODE_LTS
        .iter()
        .filter(|v| {
            let n: u32 = v.parse().unwrap_or(0);
            n >= min && upper.map_or(true, |(m, inclusive)| if inclusive { n <= m } else { n < m })
        })
        .map(|s| s.to_string())
        .collect()
}

/// Python CPython versions currently receiving security / active support.
static PYTHON_LTS: &[&str] = &["3.10", "3.11", "3.12", "3.13"];

fn expand_python(constraint: &str) -> Vec<String> {
    if constraint.is_empty() {
        return PYTHON_LTS.iter().map(|s| s.to_string()).collect();
    }
    // `python_requires` is a PEP 440 specifier like `>=3.10,<4`
    // Extract the minor version from the lower bound (e.g. `>=3.10` → 10).
    let min_minor = parse_python_lower_minor(constraint).unwrap_or(0);
    PYTHON_LTS
        .iter()
        .filter(|v| {
            let minor = v.split('.').nth(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            minor >= min_minor
        })
        .map(|s| s.to_string())
        .collect()
}

/// Go versions — stable series for the last two minor releases.
static GO_STABLE: &[&str] = &["1.22", "1.23"];

fn expand_go(constraint: &str) -> Vec<String> {
    if constraint.is_empty() {
        return GO_STABLE.iter().map(|s| s.to_string()).collect();
    }
    // go.mod `go 1.21` means minimum — test that and latest stable
    let declared = constraint.trim_start_matches("~").trim().to_string();
    let mut versions: Vec<String> = GO_STABLE
        .iter()
        .filter(|&&v| v >= declared.as_str())
        .map(|s| s.to_string())
        .collect();
    // Always include the declared version if it isn't already present
    if !versions.contains(&declared) && !declared.is_empty() {
        versions.insert(0, declared);
    }
    versions
}

/// Rust — channels.  Meaningful matrix is `stable` + `beta`; `nightly` is
/// opt-in by convention.
fn expand_rust(constraint: &str) -> Vec<String> {
    match constraint.trim() {
        "stable" | "" => vec!["stable".to_string(), "beta".to_string()],
        "nightly" => vec!["nightly".to_string()],
        channel => vec![channel.to_string(), "stable".to_string()],
    }
}

/// Java LTS releases currently supported by Adoptium/Temurin.
static JAVA_LTS: &[&str] = &["17", "21"];

fn expand_java(constraint: &str) -> Vec<String> {
    if constraint.is_empty() {
        return JAVA_LTS.iter().map(|s| s.to_string()).collect();
    }
    let min = parse_semver_lower_bound(constraint).unwrap_or(0);
    JAVA_LTS
        .iter()
        .filter(|v| v.parse::<u32>().unwrap_or(0) >= min)
        .map(|s| s.to_string())
        .collect()
}

// ── Version constraint parser ─────────────────────────────────────────────────

/// Extracts the lower-bound major (or minor for Python/Go) version from a
/// semver constraint like `>=18`, `>=18.0.0`, `^18`, `~1.21`.
fn parse_semver_lower_bound(constraint: &str) -> Option<u32> {
    // Strip operators and pull the first numeric segment
    let stripped = constraint
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?;
    // For major-only languages take the first segment; for minor-based (Python,
    // Go) take the second if the caller normalises to that.
    stripped.split('.').next()?.parse().ok()
}

/// Extracts an explicit upper bound from a range like `>=18 <23` or `<23`.
/// Returns `(bound, inclusive)` where `inclusive = true` for `<=`.
fn parse_semver_upper_bound(constraint: &str) -> Option<(u32, bool)> {
    let lt_pos = constraint.find('<')?;
    let after_lt = &constraint[lt_pos + 1..];
    let inclusive = after_lt.starts_with('=');
    let digits = after_lt
        .trim_start_matches('=')
        .trim()
        .split(|c: char| !c.is_ascii_digit())
        .next()?;
    digits.parse().ok().map(|n| (n, inclusive))
}

/// Extracts the minor version from a Python constraint like `>=3.10` → `10`.
fn parse_python_lower_minor(constraint: &str) -> Option<u32> {
    // Find first `>=3.X` or `>3.X` pattern and extract the minor
    let stripped = constraint
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split(|c: char| c == ',' || c == ' ')
        .next()?;
    stripped.split('.').nth(1)?.parse().ok()
}

/// Maps primary language label to the key used in `CiContext.runtime_versions`.
fn runtime_key(language: &str) -> &str {
    match language {
        "javascript" | "typescript" => "node",
        other => other,
    }
}

// ── Markdown renderer ─────────────────────────────────────────────────────────

fn render_matrix_yaml(language: &str, versions: &[String]) -> String {
    let matrix_key = match language {
        "node" | "javascript" | "typescript" => "node-version",
        "python" => "python-version",
        "go" => "go-version",
        "rust" => "toolchain",
        "java" | "kotlin" => "java-version",
        _ => "version",
    };

    let version_list = versions
        .iter()
        .map(|v| format!("\"{}\"", v))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "    strategy:\n      matrix:\n        {}: [{}]\n      fail-fast: false\n",
        matrix_key, version_list
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::generator::ci_generation::test_helpers::make_base_ctx;

    // ── expand_versions ────────────────────────────────────────────────────

    #[test]
    fn test_node_no_constraint_returns_all_lts() {
        let v = expand_versions("node", "");
        assert_eq!(v, vec!["18", "20", "22"]);
    }

    #[test]
    fn test_node_lower_bound_filters() {
        // >=20 should exclude 18
        let v = expand_versions("node", ">=20");
        assert!(!v.contains(&"18".to_string()));
        assert!(v.contains(&"20".to_string()));
        assert!(v.contains(&"22".to_string()));
    }

    #[test]
    fn test_node_upper_bound_filters() {
        // >=18 <22 should exclude 22
        let v = expand_versions("node", ">=18 <22");
        assert!(v.contains(&"18".to_string()));
        assert!(v.contains(&"20".to_string()));
        assert!(!v.contains(&"22".to_string()));
    }

    #[test]
    fn test_python_no_constraint_returns_supported() {
        let v = expand_versions("python", "");
        assert!(v.contains(&"3.11".to_string()));
        assert!(v.contains(&"3.12".to_string()));
    }

    #[test]
    fn test_go_no_constraint_returns_stable_pair() {
        let v = expand_versions("go", "");
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn test_rust_stable_returns_stable_and_beta() {
        let v = expand_versions("rust", "stable");
        assert_eq!(v, vec!["stable", "beta"]);
    }

    #[test]
    fn test_rust_nightly_returns_nightly_only() {
        let v = expand_versions("rust", "nightly");
        assert_eq!(v, vec!["nightly"]);
    }

    #[test]
    fn test_java_no_constraint_returns_lts() {
        let v = expand_versions("java", "");
        assert_eq!(v, vec!["17", "21"]);
    }

    #[test]
    fn test_unknown_language_passthrough() {
        let v = expand_versions("cobol", "6.5");
        assert_eq!(v, vec!["6.5"]);
    }

    #[test]
    fn test_unknown_language_empty_constraint_returns_empty() {
        let v = expand_versions("cobol", "");
        assert!(v.is_empty());
    }

    // ── generate_version_matrix ────────────────────────────────────────────

    #[test]
    fn test_returns_none_when_single_version() {
        let mut ctx = make_base_ctx(Path::new("/tmp/test"), "Python");
        ctx.runtime_versions.insert("python".to_string(), ">=3.13".to_string());
        // Only 3.13 matches >=3.13 in our LTS table → single entry → None
        let m = generate_version_matrix(&ctx);
        assert!(m.is_none());
    }

    #[test]
    fn test_node_matrix_from_context() {
        let mut ctx = make_base_ctx(Path::new("/tmp/test"), "JavaScript");
        ctx.runtime_versions.insert("node".to_string(), ">=18".to_string());
        let m = generate_version_matrix(&ctx).unwrap();
        assert_eq!(m.language, "javascript");
        assert!(m.versions.len() >= 2);
    }

    #[test]
    fn test_rendered_yaml_contains_matrix_key() {
        let mut ctx = make_base_ctx(Path::new("/tmp/test"), "JavaScript");
        ctx.runtime_versions.insert("node".to_string(), ">=18".to_string());
        let m = generate_version_matrix(&ctx).unwrap();
        assert!(m.rendered_yaml.contains("node-version"));
        assert!(m.rendered_yaml.contains("fail-fast: false"));
    }

    #[test]
    fn test_rust_matrix_from_context() {
        let mut ctx = make_base_ctx(Path::new("/tmp/test"), "Rust");
        ctx.runtime_versions.insert("rust".to_string(), "stable".to_string());
        let m = generate_version_matrix(&ctx).unwrap();
        assert!(m.versions.contains(&"stable".to_string()));
        assert!(m.versions.contains(&"beta".to_string()));
        assert!(m.rendered_yaml.contains("toolchain"));
    }
}
