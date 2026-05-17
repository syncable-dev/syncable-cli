//! CI-16 — Monorepo CI Strategy Generator
//!
//! When `CiContext.monorepo = true` this module generates two GitHub Actions
//! job fragments that together implement a path-filtered matrix build:
//!
//! 1. `detect-changes` — uses `dorny/paths-filter` to produce a JSON list of
//!    packages whose files changed in the current push/PR.
//! 2. `ci` (matrix) — depends on `detect-changes`, fans out one runner per
//!    changed package, scoping all steps to that package subdirectory.
//!
//! The fragments are returned as YAML strings so the template builders
//! (CI-11/12/13) can splice them in without knowing the internals of this
//! module.  For non-monorepo projects the public functions return `None`,
//! which callers treat as "use single-project job structure".

use crate::generator::ci_generation::context::CiContext;

// ── Public types ──────────────────────────────────────────────────────────────

/// Rendered monorepo strategy ready for insertion into a GitHub Actions workflow.
#[derive(Debug, Clone)]
pub struct MonorepoStrategy {
    /// Packages detected in the repository (relative paths from root).
    pub packages: Vec<String>,
    /// YAML fragment for the `detect-changes` job.
    pub detect_job_yaml: String,
    /// YAML fragment for the matrix `ci` job (references `detect-changes`).
    pub matrix_job_yaml: String,
    /// `dorny/paths-filter` filter block — one entry per package.
    pub filter_config: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns a `MonorepoStrategy` when `ctx.monorepo` is `true` and at least
/// two packages are present.  Returns `None` for single-project repositories
/// so callers can unconditionally call this and branch on `Option`.
pub fn generate_monorepo_strategy(ctx: &CiContext) -> Option<MonorepoStrategy> {
    if !ctx.monorepo || ctx.monorepo_packages.len() < 2 {
        return None;
    }
    let packages = ctx.monorepo_packages.clone();
    let filter_config = build_filter_config(&packages);
    let detect_job_yaml = build_detect_job(&filter_config);
    let matrix_job_yaml = build_matrix_job(ctx, &packages);
    Some(MonorepoStrategy {
        packages,
        detect_job_yaml,
        matrix_job_yaml,
        filter_config,
    })
}

// ── Internal builders ─────────────────────────────────────────────────────────

/// Builds the `dorny/paths-filter` `filters` block.
///
/// Each package gets a filter named after its directory slug — any file
/// change under that directory triggers the corresponding matrix entry.
fn build_filter_config(packages: &[String]) -> String {
    let mut out = String::new();
    for pkg in packages {
        let slug = package_slug(pkg);
        out.push_str(&format!("      {}:\n        - '{}/**'\n", slug, pkg));
    }
    out
}

/// Builds the `detect-changes` job YAML fragment.
fn build_detect_job(filter_config: &str) -> String {
    format!(
        r#"  detect-changes:
    runs-on: ubuntu-latest
    outputs:
      packages: ${{{{ steps.filter.outputs.changes }}}}
    steps:
      - uses: actions/checkout@v4
      - uses: dorny/paths-filter@v3
        id: filter
        with:
          filters: |
{}
"#,
        filter_config
    )
}

/// Builds the matrix `ci` job YAML fragment.
///
/// Each matrix value is the package slug; the actual path is reconstructed
/// inside the job via the `PACKAGE_PATH` env variable derived from the matrix
/// entry name.  This keeps the YAML readable while preserving round-trip
/// correctness.
fn build_matrix_job(ctx: &CiContext, packages: &[String]) -> String {
    let slugs: Vec<String> = packages.iter().map(|p| package_slug(p)).collect();
    let matrix_list = slugs
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(", ");

    let test_cmd = ctx
        .config_test_command
        .as_deref()
        .unwrap_or("{{TEST_COMMAND}}")
        .to_string();
    let build_cmd = ctx
        .build_command
        .as_deref()
        .unwrap_or("{{BUILD_COMMAND}}")
        .to_string();

    format!(
        r#"  ci:
    needs: detect-changes
    if: ${{{{ needs.detect-changes.outputs.packages != '[]' }}}}
    runs-on: ubuntu-latest
    strategy:
      matrix:
        package: ${{{{ fromJson(needs.detect-changes.outputs.packages) }}}}
      fail-fast: false
    defaults:
      run:
        working-directory: ${{{{ matrix.package }}}}
    steps:
      - uses: actions/checkout@v4

      # CI-03: runtime + cache scoped to package directory
      - uses: actions/cache@v4
        with:
          path: "{{{{CACHE_PATH}}}}"
          key: "${{{{ runner.os }}}}-${{{{ matrix.package }}}}-${{{{ hashFiles(format('{{{{LOCK_FILE}}}}') ) }}}}"

      - name: Install dependencies
        run: "{{{{INSTALL_COMMAND}}}}"

      - name: Test
        run: {test_cmd}

      - name: Build
        run: {build_cmd}
    # Available packages: [{matrix_list}]
"#
    )
}

/// Converts a package path like `packages/api` into a slug `api`, or
/// `services/auth-service` into `auth-service`.  Uses the last path component.
fn package_slug(path: &str) -> String {
    path.trim_end_matches('/')
        .split('/')
        .last()
        .unwrap_or(path)
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::generator::ci_generation::test_helpers::make_base_ctx;

    fn monorepo_ctx(packages: &[&str]) -> CiContext {
        let mut ctx = make_base_ctx(Path::new("/tmp/test"), "Rust");
        ctx.monorepo = true;
        ctx.monorepo_packages = packages.iter().map(|s| s.to_string()).collect();
        ctx
    }

    #[test]
    fn test_returns_none_for_single_project() {
        let ctx = make_base_ctx(Path::new("/tmp/test"), "Rust");
        assert!(generate_monorepo_strategy(&ctx).is_none());
    }

    #[test]
    fn test_returns_none_when_monorepo_flag_false() {
        let mut ctx = make_base_ctx(Path::new("/tmp/test"), "Rust");
        ctx.monorepo = true;
        ctx.monorepo_packages = vec!["packages/api".to_string()]; // only one
        assert!(generate_monorepo_strategy(&ctx).is_none());
    }

    #[test]
    fn test_returns_strategy_for_two_packages() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web"]);
        let strategy = generate_monorepo_strategy(&ctx).unwrap();
        assert_eq!(strategy.packages.len(), 2);
    }

    #[test]
    fn test_detect_job_contains_dorny_filter() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web"]);
        let s = generate_monorepo_strategy(&ctx).unwrap();
        assert!(s.detect_job_yaml.contains("dorny/paths-filter"));
    }

    #[test]
    fn test_detect_job_outputs_packages() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web"]);
        let s = generate_monorepo_strategy(&ctx).unwrap();
        assert!(s.detect_job_yaml.contains("packages:"));
        assert!(s.detect_job_yaml.contains("outputs.changes"));
    }

    #[test]
    fn test_filter_config_covers_each_package() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web"]);
        let s = generate_monorepo_strategy(&ctx).unwrap();
        assert!(s.filter_config.contains("api:"));
        assert!(s.filter_config.contains("web:"));
        assert!(s.filter_config.contains("packages/api/**"));
        assert!(s.filter_config.contains("packages/web/**"));
    }

    #[test]
    fn test_matrix_job_needs_detect_changes() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web"]);
        let s = generate_monorepo_strategy(&ctx).unwrap();
        assert!(s.matrix_job_yaml.contains("needs: detect-changes"));
    }

    #[test]
    fn test_matrix_job_uses_fail_fast_false() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web"]);
        let s = generate_monorepo_strategy(&ctx).unwrap();
        assert!(s.matrix_job_yaml.contains("fail-fast: false"));
    }

    #[test]
    fn test_matrix_job_working_directory() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web"]);
        let s = generate_monorepo_strategy(&ctx).unwrap();
        assert!(s.matrix_job_yaml.contains("working-directory:"));
        assert!(s.matrix_job_yaml.contains("matrix.package"));
    }

    #[test]
    fn test_package_slug_last_component() {
        assert_eq!(package_slug("packages/api"), "api");
        assert_eq!(package_slug("services/auth-service"), "auth-service");
        assert_eq!(package_slug("web"), "web");
    }

    #[test]
    fn test_package_slug_strips_trailing_slash() {
        assert_eq!(package_slug("packages/api/"), "api");
    }

    #[test]
    fn test_three_packages_all_appear_in_matrix() {
        let ctx = monorepo_ctx(&["packages/api", "packages/web", "packages/worker"]);
        let s = generate_monorepo_strategy(&ctx).unwrap();
        assert!(s.matrix_job_yaml.contains("\"api\""));
        assert!(s.matrix_job_yaml.contains("\"web\""));
        assert!(s.matrix_job_yaml.contains("\"worker\""));
    }
}
