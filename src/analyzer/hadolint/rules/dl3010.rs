//! DL3010: Use ADD for extracting archives into an image
//!
//! ADD can automatically extract tar archives. Use ADD instead of
//! COPY + RUN tar for better efficiency.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3010",
        Severity::Info,
        "Use ADD for extracting archives into an image.",
        |instr, _shell| {
            match instr {
                Instruction::Copy(args, _) => {
                    // Check if any source looks like a local tar archive
                    !args.sources.iter().any(|src| is_local_archive(src))
                }
                _ => true,
            }
        },
    )
}

/// Check if source is a local archive file (not URL)
fn is_local_archive(src: &str) -> bool {
    // Skip URLs
    if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("ftp://") {
        return false;
    }

    // Skip variables
    if src.starts_with('$') {
        return false;
    }

    // Check for archive extensions
    let archive_extensions = [
        ".tar",
        ".tar.gz",
        ".tgz",
        ".tar.bz2",
        ".tbz2",
        ".tar.xz",
        ".txz",
        ".tar.zst",
        ".tar.lz",
        ".tar.lzma",
    ];

    let lower = src.to_lowercase();
    archive_extensions.iter().any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::config::HadolintConfig;
    use crate::analyzer::hadolint::lint::{LintResult, lint};

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_copy_tar_file() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nCOPY app.tar.gz /app/");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3010"));
    }

    #[test]
    fn test_copy_tgz_file() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nCOPY archive.tgz /tmp/");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3010"));
    }

    #[test]
    fn test_copy_regular_file() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nCOPY app.js /app/");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3010"));
    }

    #[test]
    fn test_copy_directory() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nCOPY src/ /app/");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3010"));
    }
}
