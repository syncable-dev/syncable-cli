//! DL3060: yarn cache clean missing after yarn install
//!
//! Clean up yarn cache after installing packages.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3060",
        Severity::Info,
        "`yarn cache clean` missing after `yarn install`.",
        |instr, shell| match instr {
            Instruction::Run(_) => {
                if let Some(shell) = shell {
                    let has_install = shell.any_command(|cmd| {
                        cmd.name == "yarn" && cmd.has_any_arg(&["install", "add"])
                    });

                    if !has_install {
                        return true;
                    }

                    shell.any_command(|cmd| {
                        (cmd.name == "yarn"
                            && cmd.has_any_arg(&["cache"])
                            && cmd.arguments.iter().any(|a| a == "clean"))
                            || (cmd.name == "rm"
                                && cmd
                                    .arguments
                                    .iter()
                                    .any(|a| a.contains("yarn") && a.contains("cache")))
                    })
                } else {
                    true
                }
            }
            _ => true,
        },
    )
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
    fn test_yarn_without_clean() {
        let result = lint_dockerfile("FROM node:18\nRUN yarn install");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3060"));
    }

    #[test]
    fn test_yarn_with_clean() {
        let result = lint_dockerfile("FROM node:18\nRUN yarn install && yarn cache clean");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3060"));
    }

    #[test]
    fn test_yarn_add_without_clean() {
        let result = lint_dockerfile("FROM node:18\nRUN yarn add express");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3060"));
    }
}
