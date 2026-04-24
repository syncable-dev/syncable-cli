//! CI-21 — Dry-Run & Pretty-Print Mode
//!
//! Renders all generated CI files and metadata to a `String` without touching
//! the filesystem.  The handler calls `print_dry_run` which delegates to
//! `render_dry_run` — keeping the rendering logic pure and testable.
//!
//! ## Output sections
//!
//! 1. **Header** — banner stating no files will be written.
//! 2. **File blocks** — for each `CiFile`: the would-create path, then the
//!    full content surrounded by faint separators.
//! 3. **Unresolved token table** — only emitted when tokens remain.
//! 4. **Summary line** — N files, M tokens unresolved.

use std::path::Path;

use colored::Colorize;

use crate::generator::ci_generation::{
    schema::CiPipeline,
    writer::{resolve_path, CiFile},
};

// ── Public API ─────────────────────────────────────────────────────────────────

/// Renders the dry-run output and prints it to stdout.
pub fn print_dry_run(files: &[CiFile], pipeline: &CiPipeline, output_dir: &Path) {
    print!("{}", render_dry_run(files, pipeline, output_dir));
}

/// Renders the dry-run output to a `String`.
///
/// Pure function — no I/O, fully testable.
pub fn render_dry_run(files: &[CiFile], pipeline: &CiPipeline, output_dir: &Path) -> String {
    let mut out = String::new();

    // ── Header ────────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n{}\n{}\n{}\n\n",
        "╭─ Dry Run ─ no files will be written ─────────────────────────────╮"
            .bright_cyan()
            .bold(),
        format!(
            "│  {} file{} would be generated                                      │",
            files.len(),
            if files.len() == 1 { "" } else { "s" }
        )
        .bright_cyan(),
        "╰───────────────────────────────────────────────────────────────────╯"
            .bright_cyan()
            .bold(),
    ));

    // ── File blocks ───────────────────────────────────────────────────────
    let sep = "─".repeat(68);
    for file in files {
        let path = resolve_path(output_dir, &file.kind);
        out.push_str(&format!(
            "  {} {}\n",
            "Would create:".dimmed(),
            path.display().to_string().cyan().bold(),
        ));
        out.push_str(&format!("{}\n", sep.dimmed()));
        out.push_str(&file.content);
        if !file.content.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("{}\n\n", sep.dimmed()));
    }

    // ── Unresolved token table ────────────────────────────────────────────
    if !pipeline.unresolved_tokens.is_empty() {
        out.push_str(&format!(
            "{}\n",
            "╭─ Unresolved Tokens ───────────────────────────────────────────────╮"
                .yellow()
                .bold()
        ));
        out.push_str(&format!(
            "│  {:<28} {:<20} {}\n",
            "Token".yellow().bold(),
            "Placeholder".yellow().bold(),
            "Hint".yellow().bold(),
        ));
        out.push_str(&format!("│  {}\n", "─".repeat(64).dimmed()));

        for token in &pipeline.unresolved_tokens {
            out.push_str(&format!(
                "│  {:<28} {:<20} {}\n",
                token.name.as_str().bright_white(),
                token.placeholder.as_str().bright_yellow(),
                token.hint.as_str().dimmed(),
            ));
        }
        out.push_str(&format!(
            "{}\n\n",
            "╰───────────────────────────────────────────────────────────────────╯"
                .yellow()
                .bold()
        ));
    }

    // ── Summary line ──────────────────────────────────────────────────────
    let token_count = pipeline.unresolved_tokens.len();
    let summary = format!(
        "  {} {} file{} to write  •  {} unresolved token{}",
        "→".bright_cyan(),
        files.len(),
        if files.len() == 1 { "" } else { "s" },
        token_count,
        if token_count == 1 { "" } else { "s" },
    );
    out.push_str(&format!("{}\n\n", summary));

    out
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CiFormat, CiPlatform};
    use crate::generator::ci_generation::{
        schema::{
            ArtifactStep, BuildStep, CacheStep, CiPipeline, DockerBuildStep, ImageScanStep,
            InstallStep, LintStep, RuntimeStep, SecretScanStep, TestStep, TriggerConfig,
            UnresolvedToken,
        },
        writer::CiFileKind,
    };

    fn make_pipeline(unresolved: Vec<UnresolvedToken>) -> CiPipeline {
        CiPipeline {
            project_name: "test-project".into(),
            platform: CiPlatform::Hetzner,
            format: CiFormat::GithubActions,
            triggers: TriggerConfig {
                push_branches: vec!["main".into()],
                pr_branches: vec!["main".into()],
                tag_pattern: None,
                scheduled: None,
            },
            runtime: RuntimeStep {
                action: "actions/setup-node@v4".into(),
                version: "20".into(),
            },
            cache: None,
            install: InstallStep { command: "npm ci".into() },
            lint: None,
            test: TestStep {
                command: "npm test".into(),
                coverage_flag: None,
                coverage_report_path: None,
            },
            build: None,
            docker_build: None,
            image_scan: None,
            secret_scan: SecretScanStep {
                github_token_expr: "${{ secrets.GITHUB_TOKEN }}".into(),
                gitleaks_license_secret: None,
            },
            upload_artifact: None,
            unresolved_tokens: unresolved,
        }
    }

    const YAML: &str = "name: CI\non:\n  push:\n    branches: [main]\n";

    fn make_files() -> Vec<CiFile> {
        vec![CiFile::pipeline(YAML.to_string(), CiFormat::GithubActions)]
    }

    #[test]
    fn test_render_contains_would_create_path() {
        let rendered = render_dry_run(&make_files(), &make_pipeline(vec![]), Path::new("/proj"));
        assert!(rendered.contains("Would create:") || rendered.contains("would create:") || rendered.contains(".github/workflows/ci.yml"));
    }

    #[test]
    fn test_render_contains_file_content() {
        let rendered = render_dry_run(&make_files(), &make_pipeline(vec![]), Path::new("/proj"));
        assert!(rendered.contains("name: CI"));
    }

    #[test]
    fn test_render_no_tokens_section_when_all_resolved() {
        let rendered = render_dry_run(&make_files(), &make_pipeline(vec![]), Path::new("/proj"));
        assert!(!rendered.contains("Unresolved Tokens"));
    }

    #[test]
    fn test_render_shows_token_table_when_unresolved() {
        let tokens = vec![
            UnresolvedToken::new("REGISTRY_URL", "Your container registry base URL", "url"),
        ];
        let rendered = render_dry_run(&make_files(), &make_pipeline(tokens), Path::new("/proj"));
        assert!(rendered.contains("Unresolved Tokens"));
        assert!(rendered.contains("REGISTRY_URL"));
        assert!(rendered.contains("{{REGISTRY_URL}}"));
    }

    #[test]
    fn test_render_summary_counts_files() {
        let rendered = render_dry_run(&make_files(), &make_pipeline(vec![]), Path::new("/proj"));
        assert!(rendered.contains("1 file"));
    }

    #[test]
    fn test_render_multiple_files() {
        let files = vec![
            CiFile::pipeline(YAML.to_string(), CiFormat::GithubActions),
            CiFile::secrets_doc("# Secrets\n".to_string()),
        ];
        let rendered = render_dry_run(&files, &make_pipeline(vec![]), Path::new("/proj"));
        assert!(rendered.contains("2 files"));
        assert!(rendered.contains("SECRETS_REQUIRED.md"));
    }

    #[test]
    fn test_render_zero_unresolved_label() {
        let rendered = render_dry_run(&make_files(), &make_pipeline(vec![]), Path::new("/proj"));
        assert!(rendered.contains("0 unresolved tokens"));
    }

    #[test]
    fn test_render_singular_token_label() {
        let tokens = vec![UnresolvedToken::new("FOO", "hint", "string")];
        let rendered = render_dry_run(&make_files(), &make_pipeline(tokens), Path::new("/proj"));
        assert!(rendered.contains("1 unresolved token"));
    }
}
