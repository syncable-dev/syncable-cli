//! DL3041: Pin versions in dnf install
//!
//! dnf packages should be pinned to specific versions.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3041",
        Severity::Warning,
        "Specify version with `dnf install <package>-<version>`.",
        |instr, shell| match instr {
            Instruction::Run(_) => {
                if let Some(shell) = shell {
                    !shell.any_command(|cmd| {
                        if cmd.name == "dnf" && cmd.has_any_arg(&["install"]) {
                            let packages = get_dnf_packages(cmd);
                            packages.iter().any(|pkg| !is_pinned_dnf_package(pkg))
                        } else {
                            false
                        }
                    })
                } else {
                    true
                }
            }
            _ => true,
        },
    )
}

fn get_dnf_packages(cmd: &crate::analyzer::hadolint::shell::Command) -> Vec<&str> {
    let mut packages = Vec::new();
    let mut found_install = false;

    for arg in &cmd.arguments {
        if arg == "install" {
            found_install = true;
            continue;
        }
        if found_install && !arg.starts_with('-') {
            packages.push(arg.as_str());
        }
    }

    packages
}

fn is_pinned_dnf_package(pkg: &str) -> bool {
    if pkg.starts_with('-') {
        return true;
    }
    if pkg.ends_with(".rpm") {
        return true;
    }
    // dnf uses - for version: package-version-release
    let parts: Vec<&str> = pkg.rsplitn(2, '-').collect();
    if parts.len() >= 2 {
        let potential_version = parts[0];
        potential_version
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    } else {
        false
    }
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
    fn test_dnf_unpinned() {
        let result = lint_dockerfile("FROM fedora:latest\nRUN dnf install -y nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3041"));
    }

    #[test]
    fn test_dnf_pinned() {
        let result = lint_dockerfile("FROM fedora:latest\nRUN dnf install -y nginx-1.20.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3041"));
    }
}
