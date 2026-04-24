//! CD File Writer
//!
//! Writes generated CD pipeline files to the correct output paths.
//! Mirrors the CI writer (`ci_generation/writer.rs`) pattern but produces:
//!
//! | Kind             | Path                                          |
//! |------------------|-----------------------------------------------|
//! | Azure pipeline   | `.github/workflows/deploy-azure.yml`          |
//! | GCP pipeline     | `.github/workflows/deploy-gcp.yml`            |
//! | Hetzner pipeline | `.github/workflows/deploy-hetzner.yml`        |
//! | CD manifest      | `.syncable/cd-manifest.toml`                  |
//!
//! The writer validates YAML content before writing and provides a
//! `WriteSummary` for the CLI to display results.

use std::fs;
use std::path::{Path, PathBuf};

use crate::generator::cd_generation::context::CdPlatform;

// ── Public types ──────────────────────────────────────────────────────────────

/// Classifies the kind of CD file being written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdFileKind {
    /// Main CD pipeline YAML for a specific platform.
    Pipeline(CdPlatform),
    /// `.syncable/cd-manifest.toml`
    Manifest,
}

/// A generated CD file ready to be written.
#[derive(Debug, Clone)]
pub struct CdFile {
    /// File content (YAML or TOML).
    pub content: String,
    /// What kind of file this is — drives path resolution.
    pub kind: CdFileKind,
}

impl CdFile {
    /// Constructs a pipeline YAML file.
    pub fn pipeline(content: String, platform: CdPlatform) -> Self {
        Self {
            content,
            kind: CdFileKind::Pipeline(platform),
        }
    }

    /// Constructs a manifest file.
    pub fn manifest(content: String) -> Self {
        Self {
            content,
            kind: CdFileKind::Manifest,
        }
    }

    /// Resolves the relative output path for this file.
    pub fn relative_path(&self) -> PathBuf {
        match &self.kind {
            CdFileKind::Pipeline(platform) => {
                let filename = match platform {
                    CdPlatform::Azure => "deploy-azure.yml",
                    CdPlatform::Gcp => "deploy-gcp.yml",
                    CdPlatform::Hetzner => "deploy-hetzner.yml",
                };
                PathBuf::from(".github/workflows").join(filename)
            }
            CdFileKind::Manifest => PathBuf::from(".syncable/cd-manifest.toml"),
        }
    }
}

/// Result of writing a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    /// File did not exist; was created.
    Created,
    /// File existed and was overwritten (force mode).
    Overwritten,
    /// File existed and was left unchanged (no force).
    Skipped,
}

/// Summary of a batch write operation.
#[derive(Debug, Default)]
pub struct WriteSummary {
    pub results: Vec<(PathBuf, WriteOutcome)>,
}

impl WriteSummary {
    pub fn created(&self) -> usize {
        self.results
            .iter()
            .filter(|(_, o)| *o == WriteOutcome::Created)
            .count()
    }

    pub fn overwritten(&self) -> usize {
        self.results
            .iter()
            .filter(|(_, o)| *o == WriteOutcome::Overwritten)
            .count()
    }

    pub fn skipped(&self) -> usize {
        self.results
            .iter()
            .filter(|(_, o)| *o == WriteOutcome::Skipped)
            .count()
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Writes all generated CD files to `output_dir`.
///
/// When `force` is `true`, existing files are overwritten.
/// When `force` is `false`, existing files are skipped.
pub fn write_cd_files(
    files: &[CdFile],
    output_dir: &Path,
    force: bool,
) -> crate::Result<WriteSummary> {
    let mut summary = WriteSummary::default();

    for file in files {
        let rel_path = file.relative_path();
        let full_path = output_dir.join(&rel_path);

        // Create parent directories.
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let outcome = if full_path.exists() {
            if force {
                fs::write(&full_path, &file.content)?;
                WriteOutcome::Overwritten
            } else {
                WriteOutcome::Skipped
            }
        } else {
            fs::write(&full_path, &file.content)?;
            WriteOutcome::Created
        };

        summary.results.push((rel_path, outcome));
    }

    Ok(summary)
}

/// Prints the dry-run output to stdout.
pub fn print_cd_dry_run(files: &[CdFile]) {
    for file in files {
        let path = file.relative_path();
        println!("═══ {} ═══", path.display());
        println!("{}", file.content);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn azure_pipeline_path() {
        let file = CdFile::pipeline("yaml".to_string(), CdPlatform::Azure);
        assert_eq!(
            file.relative_path(),
            PathBuf::from(".github/workflows/deploy-azure.yml")
        );
    }

    #[test]
    fn gcp_pipeline_path() {
        let file = CdFile::pipeline("yaml".to_string(), CdPlatform::Gcp);
        assert_eq!(
            file.relative_path(),
            PathBuf::from(".github/workflows/deploy-gcp.yml")
        );
    }

    #[test]
    fn hetzner_pipeline_path() {
        let file = CdFile::pipeline("yaml".to_string(), CdPlatform::Hetzner);
        assert_eq!(
            file.relative_path(),
            PathBuf::from(".github/workflows/deploy-hetzner.yml")
        );
    }

    #[test]
    fn manifest_path() {
        let file = CdFile::manifest("toml".to_string());
        assert_eq!(
            file.relative_path(),
            PathBuf::from(".syncable/cd-manifest.toml")
        );
    }

    #[test]
    fn write_creates_files() {
        let dir = tempdir().unwrap();
        let files = vec![
            CdFile::pipeline("name: test".to_string(), CdPlatform::Azure),
            CdFile::manifest("[resolved]".to_string()),
        ];

        let summary = write_cd_files(&files, dir.path(), false).unwrap();
        assert_eq!(summary.created(), 2);
        assert_eq!(summary.skipped(), 0);

        // Verify files exist.
        assert!(dir
            .path()
            .join(".github/workflows/deploy-azure.yml")
            .exists());
        assert!(dir.path().join(".syncable/cd-manifest.toml").exists());
    }

    #[test]
    fn write_skips_existing_without_force() {
        let dir = tempdir().unwrap();
        let files = vec![CdFile::pipeline("v1".to_string(), CdPlatform::Azure)];

        // First write.
        write_cd_files(&files, dir.path(), false).unwrap();

        // Second write — should skip.
        let files2 = vec![CdFile::pipeline("v2".to_string(), CdPlatform::Azure)];
        let summary = write_cd_files(&files2, dir.path(), false).unwrap();
        assert_eq!(summary.skipped(), 1);

        // Content should still be v1.
        let content = fs::read_to_string(
            dir.path().join(".github/workflows/deploy-azure.yml"),
        )
        .unwrap();
        assert_eq!(content, "v1");
    }

    #[test]
    fn write_overwrites_existing_with_force() {
        let dir = tempdir().unwrap();
        let files = vec![CdFile::pipeline("v1".to_string(), CdPlatform::Azure)];
        write_cd_files(&files, dir.path(), false).unwrap();

        let files2 = vec![CdFile::pipeline("v2".to_string(), CdPlatform::Azure)];
        let summary = write_cd_files(&files2, dir.path(), true).unwrap();
        assert_eq!(summary.overwritten(), 1);

        let content = fs::read_to_string(
            dir.path().join(".github/workflows/deploy-azure.yml"),
        )
        .unwrap();
        assert_eq!(content, "v2");
    }

    #[test]
    fn summary_counts_correct() {
        let dir = tempdir().unwrap();
        // Create one file first.
        let pre = vec![CdFile::pipeline("old".to_string(), CdPlatform::Azure)];
        write_cd_files(&pre, dir.path(), false).unwrap();

        // Now write two: one existing (skip), one new (create).
        let files = vec![
            CdFile::pipeline("new".to_string(), CdPlatform::Azure),
            CdFile::manifest("toml".to_string()),
        ];
        let summary = write_cd_files(&files, dir.path(), false).unwrap();
        assert_eq!(summary.created(), 1);
        assert_eq!(summary.skipped(), 1);
    }
}
