//! DL3032: yum clean all after yum install
//!
//! Clean up yum cache after installing packages to reduce image size.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3032",
        Severity::Warning,
        "`yum clean all` missing after yum command.",
        |instr, shell| {
            match instr {
                Instruction::Run(_) => {
                    if let Some(shell) = shell {
                        // Check if yum install is used
                        let has_yum_install = shell.any_command(|cmd| {
                            cmd.name == "yum"
                                && cmd.has_any_arg(&["install", "groupinstall", "localinstall"])
                        });

                        if !has_yum_install {
                            return true;
                        }

                        // Check if cleanup is done
                        let has_cleanup = shell.any_command(|cmd| {
                            (cmd.name == "yum" && cmd.has_any_arg(&["clean"]))
                                || (cmd.name == "rm"
                                    && cmd
                                        .arguments
                                        .iter()
                                        .any(|arg| arg.contains("/var/cache/yum")))
                        });

                        has_cleanup
                    } else {
                        true
                    }
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
    fn test_yum_install_without_clean() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum install -y nginx");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3032"));
    }

    #[test]
    fn test_yum_install_with_clean() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum install -y nginx && yum clean all");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3032"));
    }

    #[test]
    fn test_yum_install_with_rm_cache() {
        let result =
            lint_dockerfile("FROM centos:7\nRUN yum install -y nginx && rm -rf /var/cache/yum");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3032"));
    }

    #[test]
    fn test_no_yum_install() {
        let result = lint_dockerfile("FROM centos:7\nRUN yum update");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3032"));
    }
}
