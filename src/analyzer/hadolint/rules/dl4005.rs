//! DL4005: Use SHELL to change the default shell
//!
//! Instead of using shell commands to change the shell, use the SHELL instruction.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL4005",
        Severity::Warning,
        "Use `SHELL` to change the default shell.",
        |instr, _shell| {
            match instr {
                Instruction::Run(args) => {
                    let cmd_text = match &args.arguments {
                        crate::analyzer::hadolint::parser::instruction::Arguments::Text(t) => t.as_str(),
                        crate::analyzer::hadolint::parser::instruction::Arguments::List(l) => {
                            if l.is_empty() {
                                return true;
                            }
                            l.first().map(|s| s.as_str()).unwrap_or("")
                        }
                    };

                    // Check for commands that try to change shell
                    !cmd_text.contains("ln -s")
                        || !cmd_text.contains("/bin/sh")
                }
                _ => true,
            }
        },
    )
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
    fn test_shell_instruction() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nSHELL [\"/bin/bash\", \"-c\"]");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL4005"));
    }

    #[test]
    fn test_ln_s_shell() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN ln -s /bin/bash /bin/sh");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL4005"));
    }

    #[test]
    fn test_normal_run() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nRUN echo hello");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL4005"));
    }
}
