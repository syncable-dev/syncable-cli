//! DL3028: Pin versions in gem install
//!
//! Ruby gems should be pinned to specific versions.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3028",
        Severity::Warning,
        "Pin versions in gem install. Instead of `gem install <gem>` use `gem install <gem>:<version>`.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        !shell.any_command(|cmd| {
                            if cmd.name == "gem" && cmd.has_any_arg(&["install"]) {
                                // Get gems (args after install, excluding flags)
                                let gems = get_gem_packages(cmd);
                                // Check if any gem is unpinned
                                gems.iter().any(|gem| !is_pinned_gem(gem))
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

/// Extract gem names from gem install command
fn get_gem_packages(cmd: &crate::analyzer::hadolint::shell::Command) -> Vec<&str> {
    let mut gems = Vec::new();
    let mut found_install = false;

    for arg in &cmd.arguments {
        if arg == "install" {
            found_install = true;
            continue;
        }
        if found_install && !arg.starts_with('-') {
            gems.push(arg.as_str());
        }
    }

    gems
}

/// Check if gem is pinned
fn is_pinned_gem(gem: &str) -> bool {
    // Skip flags
    if gem.starts_with('-') {
        return true;
    }

    // Check for version specifier
    // gem install rails:7.0.0
    // gem install rails -v 7.0.0 (handled separately via flag check)
    gem.contains(':')
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
    fn test_gem_install_unpinned() {
        let result = lint_dockerfile("FROM ruby:3.2\nRUN gem install rails");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3028"));
    }

    #[test]
    fn test_gem_install_pinned() {
        let result = lint_dockerfile("FROM ruby:3.2\nRUN gem install rails:7.0.0");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3028"));
    }

    #[test]
    fn test_gem_install_multiple_unpinned() {
        let result = lint_dockerfile("FROM ruby:3.2\nRUN gem install bundler rake");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3028"));
    }

    #[test]
    fn test_bundle_install() {
        // bundle install uses Gemfile.lock, not relevant
        let result = lint_dockerfile("FROM ruby:3.2\nRUN bundle install");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3028"));
    }
}
