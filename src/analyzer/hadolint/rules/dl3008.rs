//! DL3008: Pin versions in apt-get install
//!
//! Package versions should be pinned in apt-get install to ensure
//! reproducible builds.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3008",
        Severity::Warning,
        "Pin versions in apt get install. Instead of `apt-get install <package>` use `apt-get install <package>=<version>`",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // Get apt-get install packages
                        let packages = apt_get_packages(shell);
                        // All packages should have versions pinned
                        packages.iter().all(|pkg| is_version_pinned(pkg))
                    } else {
                        true
                    }
                }
                _ => true,
            }
        },
    )
}

/// Extract packages from apt-get install commands.
fn apt_get_packages(shell: &ParsedShell) -> Vec<String> {
    let mut packages = Vec::new();

    for cmd in &shell.commands {
        if cmd.name == "apt-get" && cmd.arguments.iter().any(|a| a == "install") {
            // Get arguments that aren't flags and aren't "install"
            let args: Vec<&str> = cmd
                .args_no_flags()
                .into_iter()
                .filter(|a| *a != "install")
                // Filter out -t/--target-release arguments
                .collect();

            packages.extend(args.into_iter().map(|s| s.to_string()));
        }
    }

    packages
}

/// Check if a package has a version pinned.
fn is_version_pinned(package: &str) -> bool {
    // Version pinned: package=version
    package.contains('=')
        // APT pinning: package/release
        || package.contains('/')
        // Local .deb file
        || package.ends_with(".deb")
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

        let instr = Instruction::Run(RunArgs::shell("apt-get install -y nginx=1.18.0-0ubuntu1"));
        let shell = ParsedShell::parse("apt-get install -y nginx=1.18.0-0ubuntu1");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_unpinned_version() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("apt-get install -y nginx"));
        let shell = ParsedShell::parse("apt-get install -y nginx");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "DL3008");
    }

    #[test]
    fn test_apt_pinning() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("apt-get install -y nginx/focal"));
        let shell = ParsedShell::parse("apt-get install -y nginx/focal");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }

    #[test]
    fn test_update_only() {
        let rule = rule();
        let mut state = RuleState::new();

        let instr = Instruction::Run(RunArgs::shell("apt-get update"));
        let shell = ParsedShell::parse("apt-get update");
        rule.check(&mut state, 1, &instr, Some(&shell));
        assert!(state.failures.is_empty());
    }
}
