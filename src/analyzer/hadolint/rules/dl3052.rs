//! DL3052: Label `org.opencontainers.image.licenses` is not a valid SPDX expression
//!
//! The licenses label should contain a valid SPDX license identifier.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{simple_rule, SimpleRule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3052",
        Severity::Warning,
        "Label `org.opencontainers.image.licenses` is not a valid SPDX expression.",
        |instr, _shell| {
            match instr {
                Instruction::Label(pairs) => {
                    for (key, value) in pairs {
                        if key == "org.opencontainers.image.licenses" {
                            if value.is_empty() || !is_valid_spdx(value) {
                                return false;
                            }
                        }
                    }
                    true
                }
                _ => true,
            }
        },
    )
}

fn is_valid_spdx(license: &str) -> bool {
    // Common SPDX license identifiers
    let common_licenses = [
        "MIT", "Apache-2.0", "GPL-2.0", "GPL-2.0-only", "GPL-2.0-or-later",
        "GPL-3.0", "GPL-3.0-only", "GPL-3.0-or-later", "BSD-2-Clause",
        "BSD-3-Clause", "ISC", "MPL-2.0", "LGPL-2.1", "LGPL-2.1-only",
        "LGPL-2.1-or-later", "LGPL-3.0", "LGPL-3.0-only", "LGPL-3.0-or-later",
        "AGPL-3.0", "AGPL-3.0-only", "AGPL-3.0-or-later", "Unlicense",
        "CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0", "WTFPL", "Zlib", "0BSD",
        "EPL-1.0", "EPL-2.0", "EUPL-1.2", "PostgreSQL", "OFL-1.1",
        "Artistic-2.0", "BSL-1.0", "CDDL-1.0", "CDDL-1.1", "CPL-1.0",
    ];

    // Check for common licenses (case-insensitive)
    let license_upper = license.to_uppercase();

    // Handle compound expressions (AND, OR, WITH)
    let parts: Vec<&str> = license_upper
        .split(|c| c == '(' || c == ')' || c == ' ')
        .filter(|s| !s.is_empty() && *s != "AND" && *s != "OR" && *s != "WITH")
        .collect();

    if parts.is_empty() {
        return false;
    }

    parts.iter().all(|part| {
        common_licenses.iter().any(|l| l.to_uppercase() == *part)
    })
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
    fn test_valid_spdx() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.licenses=\"MIT\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3052"));
    }

    #[test]
    fn test_valid_compound_spdx() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.licenses=\"MIT OR Apache-2.0\"");
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3052"));
    }

    #[test]
    fn test_invalid_spdx() {
        let result = lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.licenses=\"NotALicense\"");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3052"));
    }
}
