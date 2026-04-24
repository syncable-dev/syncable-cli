//! CI Trigger Configuration — CI-18
//!
//! Resolves `TriggerConfig` from the project's default branch and an optional
//! semver tag pattern detected in the repository's git history.

use std::path::Path;
use std::process::Command;

use crate::generator::ci_generation::{context::CiContext, schema::TriggerConfig};

/// Resolves the trigger configuration for a CI pipeline.
///
/// Both push and PR triggers default to the project's detected default branch.
/// If the repository contains any tags matching the glob `v*`, the tag trigger
/// is enabled so release workflows fire automatically on versioned tags.
pub fn resolve_triggers(ctx: &CiContext) -> TriggerConfig {
    let root = &ctx.analysis.project_root;
    let branch = ctx.default_branch.clone();

    TriggerConfig {
        push_branches: vec![branch.clone()],
        pr_branches: vec![branch],
        tag_pattern: detect_semver_tag_pattern(root),
        scheduled: Some("{{CRON_SCHEDULE}}".to_string()),
    }
}

/// Returns `Some("v*")` when the repo at `path` has at least one `v*` tag.
/// Returns `None` on any git error or if no such tags exist.
fn detect_semver_tag_pattern(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["tag", "--list", "v*"])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty() {
        Some("v*".to_string())
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::{context::CiContext, test_helpers::make_base_ctx};
    use tempfile::TempDir;

    fn ctx_on(root: &std::path::Path, branch: &str) -> CiContext {
        CiContext { default_branch: branch.to_string(), ..make_base_ctx(root, "rust") }
    }

    #[test]
    fn test_push_and_pr_branches_match_default_branch() {
        let dir = TempDir::new().unwrap();
        let triggers = resolve_triggers(&ctx_on(dir.path(), "develop"));

        assert_eq!(triggers.push_branches, vec!["develop"]);
        assert_eq!(triggers.pr_branches, vec!["develop"]);
    }

    #[test]
    fn test_scheduled_emits_cron_placeholder() {
        let dir = TempDir::new().unwrap();
        let triggers = resolve_triggers(&ctx_on(dir.path(), "main"));
        assert_eq!(triggers.scheduled, Some("{{CRON_SCHEDULE}}".to_string()));
    }

    #[test]
    fn test_no_git_repo_yields_no_tag_pattern() {
        let dir = TempDir::new().unwrap();
        // Plain temp dir — git command fails → tag_pattern is None.
        let triggers = resolve_triggers(&ctx_on(dir.path(), "main"));
        assert!(triggers.tag_pattern.is_none());
    }

    #[test]
    fn test_semver_tag_detected() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Bootstrap a minimal git repo with a semver tag.
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(root)
                .env("GIT_AUTHOR_NAME", "ci-test")
                .env("GIT_AUTHOR_EMAIL", "ci@test.local")
                .env("GIT_COMMITTER_NAME", "ci-test")
                .env("GIT_COMMITTER_EMAIL", "ci@test.local")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .expect("git command failed")
        };

        git(&["init"]);
        git(&["commit", "--allow-empty", "-m", "init"]);
        git(&["tag", "v1.0.0"]);

        let triggers = resolve_triggers(&ctx_on(root, "main"));
        assert_eq!(triggers.tag_pattern, Some("v*".to_string()));
    }

    #[test]
    fn test_non_semver_tags_yield_no_tag_pattern() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(root)
                .env("GIT_AUTHOR_NAME", "ci-test")
                .env("GIT_AUTHOR_EMAIL", "ci@test.local")
                .env("GIT_COMMITTER_NAME", "ci-test")
                .env("GIT_COMMITTER_EMAIL", "ci@test.local")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .expect("git command failed")
        };

        git(&["init"]);
        git(&["commit", "--allow-empty", "-m", "init"]);
        git(&["tag", "release-1.0"]);  // no "v" prefix — should not match

        let triggers = resolve_triggers(&ctx_on(root, "main"));
        assert!(triggers.tag_pattern.is_none());
    }
}
