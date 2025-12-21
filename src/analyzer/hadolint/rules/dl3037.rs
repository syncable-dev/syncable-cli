//! DL3037: Pin versions in zypper install
//!
//! zypper packages should be pinned to specific versions.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3037",
        Severity::Warning,
        "Specify version with `zypper install <package>=<version>`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "zypper" && cmd.has_any_arg(&["install", "in"]) {
                                let packages = get_zypper_packages(cmd);
                                packages.iter().any(|pkg| !is_pinned_zypper_package(pkg))
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

fn get_zypper_packages(cmd: &crate::analyzer::hadolint::shell::Command) -> Vec<&str> {
    let mut packages = Vec::new();
    let mut found_install = false;

    for arg in &cmd.arguments {
        if arg == "install" || arg == "in" {
            found_install = true;
            continue;
        }
        if found_install && !arg.starts_with('-') {
            packages.push(arg.as_str());
        }
    }

    packages
}

fn is_pinned_zypper_package(pkg: &str) -> bool {
    if pkg.starts_with('-') {
        return true;
    }
    if pkg.ends_with(".rpm") {
        return true;
    }
    // zypper uses = or >= for version pinning
    pkg.contains('=') || pkg.contains(">=") || pkg.contains("<=")
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
    fn test_zypper_unpinned() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n install nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3037"));
    }

    #[test]
    fn test_zypper_pinned() {
        let result = lint_dockerfile("FROM opensuse:latest\nRUN zypper -n install nginx=1.20.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3037"));
    }
}
