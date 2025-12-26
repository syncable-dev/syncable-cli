//! DL3024: FROM aliases must be unique
//!
//! Each FROM instruction should have a unique alias.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{CustomRule, RuleState, custom_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule()
-> CustomRule<impl Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync> {
    custom_rule(
        "DL3024",
        Severity::Error,
        "`FROM` aliases (stage names) must be unique.",
        |state, line, instr, _shell| {
            if let Instruction::From(base) = instr {
                if let Some(alias) = &base.alias {
                    let alias_str = alias.as_str();
                    if state.data.set_contains("seen_aliases", alias_str) {
                        state.add_failure(
                            "DL3024",
                            Severity::Error,
                            format!("Duplicate `FROM` alias `{}`.", alias_str),
                            line,
                        );
                    } else {
                        state.data.insert_to_set("seen_aliases", alias_str);
                    }
                }
            }
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
    fn test_duplicate_alias() {
        let result = lint_dockerfile(
            "FROM node:18 AS builder\nRUN npm ci\nFROM node:18-alpine AS builder\nRUN echo done",
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3024"));
    }

    #[test]
    fn test_unique_aliases() {
        let result = lint_dockerfile(
            "FROM node:18 AS builder\nRUN npm ci\nFROM node:18-alpine AS runner\nRUN echo done",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3024"));
    }

    #[test]
    fn test_no_aliases() {
        let result =
            lint_dockerfile("FROM node:18\nRUN npm ci\nFROM node:18-alpine\nRUN echo done");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3024"));
    }

    #[test]
    fn test_single_stage() {
        let result = lint_dockerfile("FROM node:18 AS builder\nRUN npm ci");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3024"));
    }
}
