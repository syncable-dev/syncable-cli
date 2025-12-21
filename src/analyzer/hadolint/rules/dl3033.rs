//! DL3033: Pin versions in yum install
//!
//! Yum packages should be pinned to specific versions.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3033",
        Severity::Warning,
        "Specify version with `yum install -y <package>-<version>`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "yum" && cmd.has_any_arg(&["install"]) {
                                // Get packages (args after install, excluding flags)
                                let packages = get_yum_packages(cmd);
                                // Check if any package is unpinned
                                packages.iter().any(|pkg| !is_pinned_yum_package(pkg))
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

/// Extract package names from yum install command
fn get_yum_packages(cmd: &crate::analyzer::hadolint::shell::Command) -> Vec<&str> {
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

/// Check if yum package is pinned
fn is_pinned_yum_package(pkg: &str) -> bool {
    // Skip flags
    if pkg.starts_with('-') {
        return true;
    }

    // Skip local RPM files
    if pkg.ends_with(".rpm") {
        return true;
    }

    // Yum version formats: package-version or package-version-release
    // Simple heuristic: contains a hyphen followed by a digit
    let parts: Vec<&str> = pkg.rsplitn(2, '-').collect();
    if parts.len() >= 2 {
        let potential_version = parts[0];
        // Version typically starts with a digit
        potential_version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
    } else {
        false
    }
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
    fn test_yum_install_unpinned() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum install -y nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3033"));
    }

    #[test]
    fn test_yum_install_pinned() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum install -y nginx-1.20.1");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3033"));
    }

    #[test]
    fn test_yum_install_local_rpm() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum install -y /tmp/package.rpm");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3033"));
    }

    #[test]
    fn test_yum_update() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum update -y");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3033"));
    }
}
