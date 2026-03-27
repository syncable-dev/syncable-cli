//! DL3016: Pin versions in npm install
//!
//! npm packages should be pinned to specific versions.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3016",
        Severity::Warning,
        "Pin versions in npm. Instead of `npm install <package>` use `npm install <package>@<version>`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "npm" && cmd.has_any_arg(&["install", "i"]) {
                                // Get packages (args after install, excluding flags)
                                let packages = get_npm_packages(cmd);
                                // Check if any package is unpinned
                                packages.iter().any(|pkg| !is_pinned_npm_package(pkg))
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

/// Extract package names from npm install command
fn get_npm_packages(cmd: &crate::analyzer::hadolint::shell::Command) -> Vec<&str> {
    let mut packages = Vec::new();
    let mut found_install = false;

    for arg in &cmd.arguments {
        if arg == "install" || arg == "i" {
            found_install = true;
            continue;
        }
        if found_install && !arg.starts_with('-') {
            packages.push(arg.as_str());
        }
    }

    packages
}

/// Check if npm package is pinned
fn is_pinned_npm_package(pkg: &str) -> bool {
    // Skip scoped packages check - just check if version is present
    // Pinned formats: package@version, package@^version, package@~version
    // Also valid: local paths, git URLs, etc.

    // Skip flags
    if pkg.starts_with('-') {
        return true;
    }

    // Local paths are fine
    if pkg.starts_with('.') || pkg.starts_with('/') || pkg.starts_with("file:") {
        return true;
    }

    // Git URLs are fine
    if pkg.starts_with("git") || pkg.contains("github.com") || pkg.contains("gitlab.com") {
        return true;
    }

    // Check for @ version specifier (but not scoped package @org/name)
    if pkg.contains('@') {
        let parts: Vec<&str> = pkg.split('@').collect();
        // Scoped package: @org/name or @org/name@version
        if pkg.starts_with('@') {
            // @org/name@version - has 3 parts
            parts.len() >= 3
        } else {
            // name@version - has 2 parts
            parts.len() >= 2 && !parts[1].is_empty()
        }
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
    fn test_npm_install_unpinned() {
        let result = lint_dockerfile("FROM node:18\nRUN npm install express");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3016"));
    }

    #[test]
    fn test_npm_install_pinned() {
        let result = lint_dockerfile("FROM node:18\nRUN npm install express@4.18.2");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3016"));
    }

    #[test]
    fn test_npm_install_pinned_caret() {
        let result = lint_dockerfile("FROM node:18\nRUN npm install express@^4.18.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3016"));
    }

    #[test]
    fn test_npm_ci() {
        // npm ci uses package-lock.json, so no packages listed
        let result = lint_dockerfile("FROM node:18\nRUN npm ci");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3016"));
    }

    #[test]
    fn test_npm_install_global_unpinned() {
        let result = lint_dockerfile("FROM node:18\nRUN npm install -g typescript");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3016"));
    }

    #[test]
    fn test_npm_install_global_pinned() {
        let result = lint_dockerfile("FROM node:18\nRUN npm install -g typescript@5.0.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3016"));
    }
}
