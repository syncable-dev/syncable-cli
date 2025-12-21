//! DL3018: Pin versions in apk add
//!
//! Alpine packages should be pinned to specific versions.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3018",
        Severity::Warning,
        "Pin versions in apk add. Instead of `apk add <package>` use `apk add <package>=<version>`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "apk" && cmd.has_any_arg(&["add"]) {
                                // Get packages (args after add, excluding flags)
                                let packages = get_apk_packages(cmd);
                                // Check if any package is unpinned
                                packages.iter().any(|pkg| !is_pinned_apk_package(pkg))
                            } else {
                                false
                            }
                        })
                    } else {
                        true
                    }
                }
                _ => true,
            }
        },
    )
}

/// Extract package names from apk add command
fn get_apk_packages(cmd: &crate::analyzer::hadolint::shell::Command) -> Vec<&str> {
    let mut packages = Vec::new();
    let mut found_add = false;

    for arg in &cmd.arguments {
        if arg == "add" {
            found_add = true;
            continue;
        }
        if found_add && !arg.starts_with('-') {
            packages.push(arg.as_str());
        }
    }

    packages
}

/// Check if apk package is pinned
fn is_pinned_apk_package(pkg: &str) -> bool {
    // Skip flags
    if pkg.starts_with('-') {
        return true;
    }

    // Skip virtual packages (start with .)
    if pkg.starts_with('.') {
        return true;
    }

    // Pinned formats: package=version or package~version
    pkg.contains('=') || pkg.contains('~')
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
    fn test_apk_add_unpinned() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk add nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3018"));
    }

    #[test]
    fn test_apk_add_pinned() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk add nginx=1.24.0-r0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3018"));
    }

    #[test]
    fn test_apk_add_pinned_tilde() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk add nginx~1.24");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3018"));
    }

    #[test]
    fn test_apk_add_no_cache_unpinned() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk add --no-cache curl");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3018"));
    }

    #[test]
    fn test_apk_update() {
        let result = lint_dockerfile("FROM alpine:3.18\nRUN apk update");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3018"));
    }
}
