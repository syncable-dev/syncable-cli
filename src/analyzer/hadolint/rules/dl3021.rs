//! DL3021: Use COPY instead of ADD for non-URL archives
//!
//! COPY is preferred over ADD unless you need ADD's special features
//! (URL download or auto-extraction from remote archives).

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3021",
        Severity::Error,
        "Use `COPY` instead of `ADD` for copying non-archive files.",
        |instr, _shell| {
            match instr {
                Instruction::Add(args, _) => {
                    // ADD is acceptable if:
                    // 1. Source is a URL (ADD auto-downloads)
                    // 2. Source is a local tar archive (ADD auto-extracts)
                    args.sources.iter().all(|src| {
                        is_url(src) || is_archive(src)
                    })
                }
                _ => true,
            }
        },
    )
}

/// Check if source is a URL
fn is_url(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://") || src.starts_with("ftp://")
}

/// Check if source is an archive that ADD will extract
fn is_archive(src: &str) -> bool {
    // Skip variables
    if src.starts_with('$') {
        return true;
    }

    let archive_extensions = [
        ".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz",
        ".tar.zst", ".tar.lz", ".tar.lzma", ".gz", ".bz2", ".xz"
    ];

    let lower = src.to_lowercase();
    archive_extensions.iter().any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::lint::{lint, LintResult};
    use crate::analyzer::hadolint::config::HadolintConfig;

    fn lint_dockerfile(content: &str) -> LintResult {
        lint(content, &HadolintConfig::default())
    }

    #[test]
    fn test_add_regular_file() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nADD config.json /etc/app/");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3021"));
    }

    #[test]
    fn test_add_url() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nADD https://example.com/file.tar.gz /tmp/");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3021"));
    }

    #[test]
    fn test_add_tar_archive() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nADD app.tar.gz /app/");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3021"));
    }

    #[test]
    fn test_add_directory() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nADD src/ /app/");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3021"));
    }
}
