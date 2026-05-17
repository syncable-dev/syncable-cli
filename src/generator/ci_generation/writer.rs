//! CI-20 — CI File Writer & Conflict Detection
//!
//! Writes generated CI files to the correct platform-specific paths.
//! Before writing each file the writer:
//!
//! 1. Validates the content is parseable YAML via a `serde_yaml` round-trip.
//! 2. Checks whether the target path already exists.
//! 3. If it exists and content differs, records a conflict with a unified diff.
//!    The caller decides whether to overwrite (pass `force = true`) or skip.
//!
//! ## Output paths by format
//!
//! | Format           | Path written                         |
//! |------------------|--------------------------------------|
//! | GitHub Actions   | `.github/workflows/ci.yml`           |
//! | Azure Pipelines  | `azure-pipelines.yml`                |
//! | Cloud Build      | `cloudbuild.yaml`                    |
//! | Secrets doc      | `.syncable/SECRETS_REQUIRED.md`      |
//!
//! `write_ci_files` always writes all files for which content was provided;
//! callers build the `Vec<CiFile>` from the `CiPipeline` they assembled.
//! A `WriteSummary` is returned so the CLI can display a results table.

use std::io::BufRead;
use std::path::{Path, PathBuf};

use similar::{ChangeTag, TextDiff};

use crate::cli::CiFormat;

// ── Public types ──────────────────────────────────────────────────────────────

/// Classifies the kind of file being written — used for display and path resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CiFileKind {
    /// Main pipeline YAML (`.github/workflows/ci.yml`, `azure-pipelines.yml`, etc.)
    Pipeline(CiFormat),
    /// `.syncable/SECRETS_REQUIRED.md`
    SecretsDoc,
    /// Any other file with an explicit relative path.
    Other(String),
}

/// A generated file ready to be written.
#[derive(Debug, Clone)]
pub struct CiFile {
    /// Content string (YAML or Markdown depending on kind).
    pub content: String,
    /// What kind of file this is — drives path resolution.
    pub kind: CiFileKind,
}

impl CiFile {
    /// Constructs a pipeline YAML file for the given format.
    pub fn pipeline(content: String, format: CiFormat) -> Self {
        Self { content, kind: CiFileKind::Pipeline(format) }
    }

    /// Constructs a secrets documentation file.
    pub fn secrets_doc(content: String) -> Self {
        Self { content, kind: CiFileKind::SecretsDoc }
    }
}

/// User's chosen resolution when a conflict is detected during interactive mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Replace the existing file with the generated content.
    Overwrite,
    /// Write both versions into the file using git-style conflict markers.
    Merge,
    /// Leave the existing file unchanged.
    Skip,
}

/// Result of writing a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    /// File did not exist; was created.
    Created,
    /// File existed and was identical — no write needed.
    Unchanged,
    /// File existed with different content and `force = true` → overwritten.
    Overwritten,
    /// File existed with different content and `force = false` → not written.
    Skipped,
    /// File written with git-style conflict markers for manual resolution.
    Merged,
    /// Generated content failed the YAML validation round-trip.
    InvalidYaml(String),
}

/// Per-file result entry in `WriteSummary`.
#[derive(Debug, Clone)]
pub struct FileResult {
    /// The resolved absolute path that was (or would have been) written.
    pub path: PathBuf,
    pub outcome: WriteOutcome,
    /// Unified diff when `outcome == Overwritten | Skipped` and content differs.
    pub diff: Option<String>,
}

/// Aggregated result returned by `write_ci_files`.
#[derive(Debug, Clone, Default)]
pub struct WriteSummary {
    pub results: Vec<FileResult>,
}

impl WriteSummary {
    pub fn created(&self) -> usize {
        self.results.iter().filter(|r| r.outcome == WriteOutcome::Created).count()
    }
    pub fn overwritten(&self) -> usize {
        self.results.iter().filter(|r| r.outcome == WriteOutcome::Overwritten).count()
    }
    pub fn skipped(&self) -> usize {
        self.results.iter().filter(|r| r.outcome == WriteOutcome::Skipped).count()
    }
    pub fn invalid(&self) -> usize {
        self.results.iter().filter(|r| matches!(r.outcome, WriteOutcome::InvalidYaml(_))).count()
    }
    pub fn merged(&self) -> usize {
        self.results.iter().filter(|r| r.outcome == WriteOutcome::Merged).count()
    }
    pub fn has_conflicts(&self) -> bool {
        self.results.iter().any(|r| r.outcome == WriteOutcome::Skipped)
    }

    /// Returns a human-readable single-line summary.
    pub fn display_line(&self) -> String {
        format!(
            "{} created, {} overwritten, {} merged, {} skipped, {} invalid",
            self.created(),
            self.overwritten(),
            self.merged(),
            self.skipped(),
            self.invalid(),
        )
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Writes `files` into `output_dir`, respecting the `force` flag.
///
/// `force = true`  — overwrite any existing files without prompting.
/// `force = false` — skip files that differ from their existing on-disk version
///                   and record them as `Skipped` with a diff in the summary.
///
/// Callers that need interactive conflict resolution should inspect
/// `summary.has_conflicts()` and re-invoke with their chosen policy.
pub fn write_ci_files(
    files: &[CiFile],
    output_dir: &Path,
    force: bool,
) -> crate::Result<WriteSummary> {
    let mut summary = WriteSummary::default();

    for file in files {
        let path = resolve_path(output_dir, &file.kind);
        let result = write_one(file, &path, force)?;
        summary.results.push(result);
    }

    Ok(summary)
}

/// Interactive variant of `write_ci_files`.
///
/// Runs a first pass with `force = false` to detect conflicts, then for each
/// `Skipped` file reads one line from `reader` to ask the user what to do:
///
/// - `o` → overwrite (replace existing file with generated content)
/// - `m` → merge (write both versions with git-style conflict markers)
/// - `s` / anything else → skip (keep existing file)
///
/// `reader` is generic over `BufRead` so tests can inject a cursor instead of
/// reading from real stdin.
pub fn write_ci_files_interactive<R: BufRead>(
    files: &[CiFile],
    output_dir: &Path,
    reader: &mut R,
) -> crate::Result<WriteSummary> {
    let mut summary = write_ci_files(files, output_dir, false)?;

    for (file, result) in files.iter().zip(summary.results.iter_mut()) {
        if result.outcome != WriteOutcome::Skipped {
            continue;
        }
        let diff = result.diff.as_deref().unwrap_or("");
        let resolution = prompt_conflict_resolution(&result.path, diff, reader);
        match resolution {
            ConflictResolution::Overwrite => {
                do_write(&result.path, &file.content)?;
                result.outcome = WriteOutcome::Overwritten;
            }
            ConflictResolution::Merge => {
                let existing = std::fs::read_to_string(&result.path)?;
                let merged = conflict_markers(&existing, &file.content);
                do_write(&result.path, &merged)?;
                result.outcome = WriteOutcome::Merged;
            }
            ConflictResolution::Skip => {}
        }
    }

    Ok(summary)
}

/// Reads a single conflict-resolution choice from `reader`.
///
/// Prints a prompt line to stderr (non-blocking in tests). Parses:
/// - `"o"` → `Overwrite`
/// - `"m"` → `Merge`
/// - anything else (including `"s"`) → `Skip`
pub fn prompt_conflict_resolution<R: BufRead>(
    path: &Path,
    _diff: &str,
    reader: &mut R,
) -> ConflictResolution {
    eprintln!(
        "  conflict: {}  [o]verwrite / [m]erge / [s]kip?",
        path.display()
    );
    let mut line = String::new();
    let _ = reader.read_line(&mut line);
    match line.trim() {
        "o" => ConflictResolution::Overwrite,
        "m" => ConflictResolution::Merge,
        _ => ConflictResolution::Skip,
    }
}

/// Renders a formatted table summarising `WriteSummary` results.
///
/// Uses the box-drawing style consistent with the rest of the codebase.
/// Returns a `String` so the caller decides when/how to print it.
pub fn render_summary_table(summary: &WriteSummary) -> String {
    const PATH_W: usize = 44;
    const OUT_W: usize = 12;
    const LINE_W: usize = PATH_W + OUT_W + 5; // borders + padding

    let ruler = "─".repeat(LINE_W);
    let mut out = String::new();

    out.push_str(&format!("┌─ CI Files Written {}┐\n", "─".repeat(LINE_W - 20)));
    out.push_str(&format!(
        "│  {:<PATH_W$}  {:<OUT_W$}│\n",
        "File",
        "Outcome",
        PATH_W = PATH_W,
        OUT_W = OUT_W
    ));
    out.push_str(&format!("│  {}│\n", ruler));

    for r in &summary.results {
        let label = outcome_label(&r.outcome);
        // Show only the last two path components for readability
        let display = compact_path(&r.path);
        out.push_str(&format!(
            "│  {:<PATH_W$}  {:<OUT_W$}│\n",
            display,
            label,
            PATH_W = PATH_W,
            OUT_W = OUT_W
        ));
    }

    out.push_str(&format!("├{}┤\n", ruler));
    out.push_str(&format!("│  {:<width$}│\n", summary.display_line(), width = LINE_W - 2));
    out.push_str(&format!("└{}┘\n", ruler));

    out
}

/// Resolves the on-disk path for a `CiFileKind` relative to `output_dir`.
pub fn resolve_path(output_dir: &Path, kind: &CiFileKind) -> PathBuf {
    match kind {
        CiFileKind::Pipeline(fmt) => output_dir.join(pipeline_path(fmt)),
        CiFileKind::SecretsDoc => output_dir.join(".syncable").join("SECRETS_REQUIRED.md"),
        CiFileKind::Other(rel) => output_dir.join(rel),
    }
}

/// Maps a `CiFormat` to the conventional relative file path.
pub fn pipeline_path(format: &CiFormat) -> &'static str {
    match format {
        CiFormat::GithubActions => ".github/workflows/ci.yml",
        CiFormat::AzurePipelines => "azure-pipelines.yml",
        CiFormat::CloudBuild => "cloudbuild.yaml",
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Validates, diffs, and conditionally writes a single `CiFile`.
fn write_one(file: &CiFile, path: &Path, force: bool) -> crate::Result<FileResult> {
    // Validate YAML for pipeline files; Markdown does not need round-trip.
    if matches!(file.kind, CiFileKind::Pipeline(_)) {
        if let Err(e) = validate_yaml(&file.content) {
            return Ok(FileResult {
                path: path.to_path_buf(),
                outcome: WriteOutcome::InvalidYaml(e),
                diff: None,
            });
        }
    }

    // Check for conflict with existing file
    if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        if existing == file.content {
            return Ok(FileResult {
                path: path.to_path_buf(),
                outcome: WriteOutcome::Unchanged,
                diff: None,
            });
        }

        let diff = build_diff(&existing, &file.content);

        if force {
            do_write(path, &file.content)?;
            return Ok(FileResult {
                path: path.to_path_buf(),
                outcome: WriteOutcome::Overwritten,
                diff: Some(diff),
            });
        } else {
            return Ok(FileResult {
                path: path.to_path_buf(),
                outcome: WriteOutcome::Skipped,
                diff: Some(diff),
            });
        }
    }

    // New file — create parent directories and write
    do_write(path, &file.content)?;
    Ok(FileResult {
        path: path.to_path_buf(),
        outcome: WriteOutcome::Created,
        diff: None,
    })
}

/// Creates parent directories and writes `content` to `path`.
fn do_write(path: &Path, content: &str) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

/// Round-trips `content` through `serde_yaml` to confirm it is parseable.
/// Returns the error message on failure.
fn validate_yaml(content: &str) -> Result<(), String> {
    serde_yaml::from_str::<serde_yaml::Value>(content)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Writes both `old` and `new` into a single file using git-style conflict markers.
fn conflict_markers(old: &str, new: &str) -> String {
    format!("<<<<<<< current\n{}=======\n{}>>>>>>> generated\n", old, new)
}

/// Returns a short human-readable label for a `WriteOutcome`.
fn outcome_label(outcome: &WriteOutcome) -> &'static str {
    match outcome {
        WriteOutcome::Created => "created",
        WriteOutcome::Unchanged => "unchanged",
        WriteOutcome::Overwritten => "overwritten",
        WriteOutcome::Skipped => "skipped",
        WriteOutcome::Merged => "merged",
        WriteOutcome::InvalidYaml(_) => "invalid yaml",
    }
}

/// Returns the last two components of `path` joined by `/` for compact display.
fn compact_path(path: &Path) -> String {
    let parts: Vec<_> = path.components().collect();
    if parts.len() >= 2 {
        let n = parts.len();
        format!("{}/{}",
            parts[n - 2].as_os_str().to_string_lossy(),
            parts[n - 1].as_os_str().to_string_lossy()
        )
    } else {
        path.display().to_string()
    }
}

/// Builds a compact unified diff for display purposes.
fn build_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        out.push_str(&format!("{}{}", prefix, change));
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("syncable_writer_test_{}_{}", name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const VALID_YAML: &str = "name: CI\non:\n  push:\n    branches: [main]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n";
    const INVALID_YAML: &str = "name: CI\n  bad_indent:\n    - key: [unclosed";

    // ── resolve_path ───────────────────────────────────────────────────────

    #[test]
    fn test_github_actions_path() {
        let p = resolve_path(Path::new("/project"), &CiFileKind::Pipeline(CiFormat::GithubActions));
        assert_eq!(p, PathBuf::from("/project/.github/workflows/ci.yml"));
    }

    #[test]
    fn test_azure_pipelines_path() {
        let p = resolve_path(Path::new("/project"), &CiFileKind::Pipeline(CiFormat::AzurePipelines));
        assert_eq!(p, PathBuf::from("/project/azure-pipelines.yml"));
    }

    #[test]
    fn test_cloud_build_path() {
        let p = resolve_path(Path::new("/project"), &CiFileKind::Pipeline(CiFormat::CloudBuild));
        assert_eq!(p, PathBuf::from("/project/cloudbuild.yaml"));
    }

    #[test]
    fn test_secrets_doc_path() {
        let p = resolve_path(Path::new("/project"), &CiFileKind::SecretsDoc);
        assert_eq!(p, PathBuf::from("/project/.syncable/SECRETS_REQUIRED.md"));
    }

    // ── write_ci_files — new files ─────────────────────────────────────────

    #[test]
    fn test_creates_new_pipeline_file() {
        let dir = tmp_dir("new");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        let summary = write_ci_files(&files, &dir, false).unwrap();
        assert_eq!(summary.created(), 1);
        assert!(dir.join(".github/workflows/ci.yml").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_creates_parent_directories() {
        let dir = tmp_dir("parents");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        assert!(dir.join(".github").join("workflows").is_dir());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_creates_secrets_doc_file() {
        let dir = tmp_dir("secrets_doc");
        let files = vec![CiFile::secrets_doc("# Secrets\n".to_string())];
        let summary = write_ci_files(&files, &dir, false).unwrap();
        assert_eq!(summary.created(), 1);
        assert!(dir.join(".syncable").join("SECRETS_REQUIRED.md").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── write_ci_files — YAML validation ──────────────────────────────────

    #[test]
    fn test_invalid_yaml_results_in_invalid_outcome() {
        let dir = tmp_dir("invalid");
        let files = vec![CiFile::pipeline(INVALID_YAML.to_string(), CiFormat::GithubActions)];
        let summary = write_ci_files(&files, &dir, false).unwrap();
        assert_eq!(summary.invalid(), 1);
        assert_eq!(summary.created(), 0);
        // File must NOT be written
        assert!(!dir.join(".github/workflows/ci.yml").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_markdown_bypasses_yaml_validation() {
        // SecretsDoc is Markdown — invalid YAML characters are fine
        let dir = tmp_dir("md_bypass");
        let files = vec![CiFile::secrets_doc("# Secrets\n: not valid yaml but ok\n".to_string())];
        let summary = write_ci_files(&files, &dir, false).unwrap();
        assert_eq!(summary.invalid(), 0);
        assert_eq!(summary.created(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── write_ci_files — conflict handling ────────────────────────────────

    #[test]
    fn test_unchanged_file_not_rewritten() {
        let dir = tmp_dir("unchanged");
        // Write once
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        // Write again with identical content
        let summary = write_ci_files(&files, &dir, false).unwrap();
        assert_eq!(summary.results[0].outcome, WriteOutcome::Unchanged);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_conflict_without_force_gives_skipped() {
        let dir = tmp_dir("conflict_skip");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        // Write conflicting content without force
        let new_content = VALID_YAML.replace("CI", "CI-MODIFIED");
        let files2 = vec![CiFile::pipeline(new_content, CiFormat::GithubActions)];
        let summary = write_ci_files(&files2, &dir, false).unwrap();
        assert_eq!(summary.skipped(), 1);
        assert!(summary.has_conflicts());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_conflict_with_force_gives_overwritten() {
        let dir = tmp_dir("conflict_force");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        let new_content = VALID_YAML.replace("CI", "CI-MODIFIED");
        let files2 = vec![CiFile::pipeline(new_content.clone(), CiFormat::GithubActions)];
        let summary = write_ci_files(&files2, &dir, true).unwrap();
        assert_eq!(summary.overwritten(), 1);
        let written = std::fs::read_to_string(dir.join(".github/workflows/ci.yml")).unwrap();
        assert_eq!(written, new_content);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_conflict_includes_diff() {
        let dir = tmp_dir("diff");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        let new_content = VALID_YAML.replace("CI", "CI-MODIFIED");
        let files2 = vec![CiFile::pipeline(new_content, CiFormat::GithubActions)];
        let summary = write_ci_files(&files2, &dir, false).unwrap();
        assert!(summary.results[0].diff.is_some());
        let diff = summary.results[0].diff.as_ref().unwrap();
        assert!(diff.contains('-') || diff.contains('+'));
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── WriteSummary display ───────────────────────────────────────────────

    #[test]
    fn test_display_line_format() {
        let dir = tmp_dir("display");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        let summary = write_ci_files(&files, &dir, false).unwrap();
        let line = summary.display_line();
        assert!(line.contains("1 created"));
        assert!(line.contains("0 skipped"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_render_summary_table_contains_headers() {
        let dir = tmp_dir("table");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        let summary = write_ci_files(&files, &dir, false).unwrap();
        let table = render_summary_table(&summary);
        assert!(table.contains("CI Files Written"));
        assert!(table.contains("File"));
        assert!(table.contains("Outcome"));
        assert!(table.contains("created"));
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── prompt_conflict_resolution ────────────────────────────────────────

    #[test]
    fn test_prompt_overwrite() {
        let mut reader = std::io::Cursor::new("o\n");
        let res = prompt_conflict_resolution(Path::new("/tmp/ci.yml"), "", &mut reader);
        assert_eq!(res, ConflictResolution::Overwrite);
    }

    #[test]
    fn test_prompt_merge() {
        let mut reader = std::io::Cursor::new("m\n");
        let res = prompt_conflict_resolution(Path::new("/tmp/ci.yml"), "", &mut reader);
        assert_eq!(res, ConflictResolution::Merge);
    }

    #[test]
    fn test_prompt_skip() {
        let mut reader = std::io::Cursor::new("s\n");
        let res = prompt_conflict_resolution(Path::new("/tmp/ci.yml"), "", &mut reader);
        assert_eq!(res, ConflictResolution::Skip);
    }

    #[test]
    fn test_prompt_unrecognised_defaults_to_skip() {
        let mut reader = std::io::Cursor::new("x\n");
        let res = prompt_conflict_resolution(Path::new("/tmp/ci.yml"), "", &mut reader);
        assert_eq!(res, ConflictResolution::Skip);
    }

    // ── write_ci_files_interactive ────────────────────────────────────────

    #[test]
    fn test_interactive_overwrite_resolves_conflict() {
        let dir = tmp_dir("interactive_ow");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        let new_content = VALID_YAML.replace("CI", "CI-MODIFIED");
        let files2 = vec![CiFile::pipeline(new_content.clone(), CiFormat::GithubActions)];
        // Simulate user typing "o" at the prompt
        let mut reader = std::io::Cursor::new("o\n");
        let summary = write_ci_files_interactive(&files2, &dir, &mut reader).unwrap();
        assert_eq!(summary.overwritten(), 1);
        let written = std::fs::read_to_string(dir.join(".github/workflows/ci.yml")).unwrap();
        assert_eq!(written, new_content);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_interactive_merge_writes_conflict_markers() {
        let dir = tmp_dir("interactive_merge");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        let new_content = VALID_YAML.replace("CI", "CI-MODIFIED");
        let files2 = vec![CiFile::pipeline(new_content, CiFormat::GithubActions)];
        // Simulate user typing "m" at the prompt
        let mut reader = std::io::Cursor::new("m\n");
        let summary = write_ci_files_interactive(&files2, &dir, &mut reader).unwrap();
        assert_eq!(summary.merged(), 1);
        let written = std::fs::read_to_string(dir.join(".github/workflows/ci.yml")).unwrap();
        assert!(written.contains("<<<<<<< current"));
        assert!(written.contains(">>>>>>> generated"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_interactive_skip_leaves_existing_file() {
        let dir = tmp_dir("interactive_skip");
        let files = vec![CiFile::pipeline(VALID_YAML.to_string(), CiFormat::GithubActions)];
        write_ci_files(&files, &dir, false).unwrap();
        let new_content = VALID_YAML.replace("CI", "CI-MODIFIED");
        let files2 = vec![CiFile::pipeline(new_content, CiFormat::GithubActions)];
        let mut reader = std::io::Cursor::new("s\n");
        let summary = write_ci_files_interactive(&files2, &dir, &mut reader).unwrap();
        assert_eq!(summary.skipped(), 1);
        let written = std::fs::read_to_string(dir.join(".github/workflows/ci.yml")).unwrap();
        // Original content must be intact
        assert_eq!(written, VALID_YAML);
        std::fs::remove_dir_all(&dir).ok();
    }
}
