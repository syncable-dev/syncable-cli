//! DL3051: Label `org.opencontainers.image.created` is empty or not a valid date
//!
//! The created label should contain a valid RFC3339 date.

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::rules::{SimpleRule, simple_rule};
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::Severity;

pub fn rule() -> SimpleRule<impl Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync> {
    simple_rule(
        "DL3051",
        Severity::Warning,
        "Label `org.opencontainers.image.created` is empty or not a valid RFC3339 date.",
        |instr, _shell| match instr {
            Instruction::Label(pairs) => {
                for (key, value) in pairs {
                    if key == "org.opencontainers.image.created"
                        && (value.is_empty() || !is_valid_rfc3339(value))
                    {
                        return false;
                    }
                }
                true
            }
            _ => true,
        },
    )
}

fn is_valid_rfc3339(date: &str) -> bool {
    // Basic RFC3339 validation (YYYY-MM-DDTHH:MM:SSZ or with timezone offset)
    // Full format: 2023-01-15T14:30:00Z or 2023-01-15T14:30:00+00:00
    if date.len() < 20 {
        return false;
    }

    let chars: Vec<char> = date.chars().collect();

    // Check date part
    if chars.len() < 10 {
        return false;
    }

    // YYYY-MM-DD
    if !chars[0..4].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if chars[4] != '-' {
        return false;
    }
    if !chars[5..7].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if chars[7] != '-' {
        return false;
    }
    if !chars[8..10].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // T separator
    if chars.get(10) != Some(&'T') && chars.get(10) != Some(&'t') {
        return false;
    }

    // HH:MM:SS
    if chars.len() < 19 {
        return false;
    }
    if !chars[11..13].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if chars[13] != ':' {
        return false;
    }
    if !chars[14..16].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if chars[16] != ':' {
        return false;
    }
    if !chars[17..19].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Timezone (Z or +/-HH:MM)
    if chars.len() == 20 && chars[19] == 'Z' {
        return true;
    }

    // Allow fractional seconds before timezone
    let tz_start = if chars.get(19) == Some(&'.') {
        // Find where fractional seconds end
        let mut i = 20;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        i
    } else {
        19
    };

    if chars.len() > tz_start {
        let tz_char = chars[tz_start];
        if tz_char == 'Z' || tz_char == 'z' {
            return true;
        }
        if (tz_char == '+' || tz_char == '-') && chars.len() >= tz_start + 6 {
            return true;
        }
    }

    false
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
    fn test_valid_date() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nLABEL org.opencontainers.image.created=\"2023-01-15T14:30:00Z\"",
        );
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DL3051"));
    }

    #[test]
    fn test_empty_date() {
        let result =
            lint_dockerfile("FROM ubuntu:20.04\nLABEL org.opencontainers.image.created=\"\"");
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3051"));
    }

    #[test]
    fn test_invalid_date() {
        let result = lint_dockerfile(
            "FROM ubuntu:20.04\nLABEL org.opencontainers.image.created=\"not-a-date\"",
        );
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DL3051"));
    }
}
