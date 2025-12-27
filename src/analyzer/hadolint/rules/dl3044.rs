//! DL3044: Do not refer to an environment variable within the same ENV statement
//!
//! ENV variable references within the same statement may not work as expected.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3044",
        Severity::Error,
        "Do not refer to an environment variable within the same `ENV` statement where it is defined.",
        |instr, _shell| {
            match instr {
                Instruction::Env(pairs) => {
                    // Check if any value references a variable defined earlier in the same statement
                    // For each pair, only check against variables defined BEFORE it
                    let mut defined_vars: Vec<&str> = Vec::new();

                    for (key, value) in pairs {
                        for var in &defined_vars {
                            // Check for $VAR or ${VAR} patterns
                            if value.contains(&format!("${}", var))
                                || value.contains(&format!("${{{}}}", var))
                            {
                                return false;
                            }
                        }
                        // Add this key to defined vars for checking subsequent pairs
                        defined_vars.push(key.as_str());
                    }
                    true
                }
                _ => true,
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
    fn test_self_reference() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nENV PATH=/app:$PATH");
        // Note: PATH is not defined in this statement, so it's OK
        // This rule checks for referencing a var defined IN THE SAME statement
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3044"));
    }

    #[test]
    fn test_same_statement_reference() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nENV FOO=bar BAR=$FOO");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3044"));
    }

    #[test]
    fn test_no_reference() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nENV FOO=bar BAR=baz");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3044"));
    }
}
