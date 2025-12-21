//! DL3013: Pin versions in pip install
//!
//! Package versions should be pinned in pip install to ensure
//! reproducible builds.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3013",
        Severity::Warning,
        "Pin versions in pip. Instead of `pip install <package>` use `pip install <package>==<version>` or `pip install --requirement <requirements file>`",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // Get pip install packages
                        let packages = pip_packages(shell);
                        // Check if using requirements file
                        let uses_requirements = uses_requirements_file(shell);
                        // All packages should have versions pinned or use requirements
                        uses_requirements || packages.iter().all(|pkg| is_pip_version_pinned(pkg))
                    } else {
                        true
                    }
                }
                _ => true,
            }
        },
    )
}

/// Extract packages from pip install commands.
fn pip_packages(shell: &ParsedShell) -> Vec<String> {
    let mut packages = Vec::new();

    for cmd in &shell.commands {
        if cmd.is_pip_install() {
            // Get arguments that aren't flags and aren't pip-related commands
            let skip_args = ["install", "pip", "-m"];
            let args: Vec<&str> = cmd
                .args_no_flags()
                .into_iter()
                .filter(|a| !skip_args.contains(a))
                .collect();

            packages.extend(args.into_iter().map(|s| s.to_string()));
        }
    }

    packages
}

/// Check if pip uses a requirements file.
fn uses_requirements_file(shell: &ParsedShell) -> bool {
    shell.any_command(|cmd| {
        cmd.is_pip_install() && (cmd.has_any_flag(&["r", "requirement"]) || cmd.has_flag("constraint"))
    })
}

/// Check if a pip package has a version pinned.
fn is_pip_version_pinned(package: &str) -> bool {
    // Skip if it starts with - (it's a flag)
    if package.starts_with('-') {
        return true;
    }

    // Skip if it looks like a URL or path
    if package.contains("://") || package.starts_with('/') || package.starts_with('.') {
        return true;
    }

    // Version pinned: package==version or package>=version, etc.
    package.contains("==")
        || package.contains(">=")
        || package.contains("<=")
        || package.contains("!=")
        || package.contains("~=")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::hadolint::parser::instruction::RunArgs;
    use crate::analyzer::hadolint::rules::{Rule, RuleState};

    #[test]
    fn test_pinned_version() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("pip install requests==2.28.0"));
        let shell = ParsedShell::parse("pip install requests==2.28.0");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_unpinned_version() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("pip install requests"));
        let shell = ParsedShell::parse("pip install requests");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3013");
    }

    #[test]
    fn test_requirements_file() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("pip install -r requirements.txt"));
        let shell = ParsedShell::parse("pip install -r requirements.txt");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_min_version() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("pip install requests>=2.28.0"));
        let shell = ParsedShell::parse("pip install requests>=2.28.0");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }
}
