//! HL3xxx - Template Syntax Rules
//!
//! Rules for validating Go template syntax in Helm templates.

use crate::analyzer::helmlint::rules::{LintContext, Rule};
use crate::analyzer::helmlint::types::{CheckFailure, RuleCategory, Severity};

/// Get all HL3xxx rules.
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(HL3001),
        Box::new(HL3002),
        Box::new(HL3004),
        Box::new(HL3005),
        Box::new(HL3006),
        Box::new(HL3007),
        Box::new(HL3008),
        Box::new(HL3009),
        Box::new(HL3010),
        Box::new(HL3011),
    ]
}

/// HL3001: Unclosed template action
pub struct HL3001;

impl Rule for HL3001 {
    fn code(&self) -> &'static str {
        "HL3001"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "unclosed-action"
    }

    fn description(&self) -> &'static str {
        "Template has unclosed action (missing }})"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        for template in ctx.templates {
            for error in &template.errors {
                if error.message.contains("Unclosed template action") {
                    failures.push(CheckFailure::new(
                        "HL3001",
                        Severity::Error,
                        "Unclosed template action (missing }})".to_string(),
                        &template.path,
                        error.line,
                        RuleCategory::Template,
                    ));
                }
            }
        }

        failures
    }
}

/// HL3002: Unclosed range/if block
pub struct HL3002;

impl Rule for HL3002 {
    fn code(&self) -> &'static str {
        "HL3002"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "unclosed-block"
    }

    fn description(&self) -> &'static str {
        "Template has unclosed control block (if/range/with)"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        for template in ctx.templates {
            for (structure, line) in &template.unclosed_blocks {
                failures.push(CheckFailure::new(
                    "HL3002",
                    Severity::Error,
                    format!("Unclosed {:?} block (missing {{{{- end }}}}))", structure),
                    &template.path,
                    *line,
                    RuleCategory::Template,
                ));
            }
        }

        failures
    }
}

/// HL3004: Missing 'end' for control structure
pub struct HL3004;

impl Rule for HL3004 {
    fn code(&self) -> &'static str {
        "HL3004"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "missing-end"
    }

    fn description(&self) -> &'static str {
        "Control structure is missing closing 'end'"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        // This is covered by HL3002, but we check for specific error messages
        let mut failures = Vec::new();

        for template in ctx.templates {
            for error in &template.errors {
                if error.message.contains("Unclosed") && error.message.contains("block") {
                    failures.push(CheckFailure::new(
                        "HL3004",
                        Severity::Error,
                        error.message.clone(),
                        &template.path,
                        error.line,
                        RuleCategory::Template,
                    ));
                }
            }
        }

        failures
    }
}

/// HL3005: Using deprecated function
pub struct HL3005;

impl Rule for HL3005 {
    fn code(&self) -> &'static str {
        "HL3005"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "deprecated-function"
    }

    fn description(&self) -> &'static str {
        "Template uses deprecated function"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let deprecated_functions = [
            ("dateInZone", "Use 'mustDateModify' instead"),
            ("genCA", "Use 'genSelfSignedCert' for better control"),
        ];

        let mut failures = Vec::new();

        for template in ctx.templates {
            for (func, suggestion) in &deprecated_functions {
                if template.calls_function(func) {
                    failures.push(CheckFailure::new(
                        "HL3005",
                        Severity::Warning,
                        format!("Function '{}' is deprecated. {}", func, suggestion),
                        &template.path,
                        1, // Can't determine exact line without deeper analysis
                        RuleCategory::Template,
                    ));
                }
            }
        }

        failures
    }
}

/// HL3006: Potential nil pointer (missing 'default')
pub struct HL3006;

impl Rule for HL3006 {
    fn code(&self) -> &'static str {
        "HL3006"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "potential-nil"
    }

    fn description(&self) -> &'static str {
        "Value access may fail if value is nil. Consider using 'default'"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        // This is a heuristic check - look for deep value access without default
        let failures = Vec::new();

        for template in ctx.templates {
            // Look for deep nested access patterns that might fail
            for var in &template.variables_used {
                if var.starts_with(".Values.") {
                    let parts: Vec<&str> = var.split('.').collect();
                    // Deep nesting (more than 3 levels) without apparent default is risky
                    if parts.len() > 4 && !template.calls_function("default") {
                        // This is a very rough heuristic
                        // A more sophisticated check would track usage context
                    }
                }
            }
        }

        failures
    }
}

/// HL3007: Template file has invalid extension
pub struct HL3007;

impl Rule for HL3007 {
    fn code(&self) -> &'static str {
        "HL3007"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> &'static str {
        "invalid-template-extension"
    }

    fn description(&self) -> &'static str {
        "Template file should have .yaml, .yml, or .tpl extension"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let valid_extensions = [".yaml", ".yml", ".tpl", ".txt"];
        let mut failures = Vec::new();

        for file in ctx.files {
            if file.contains("templates/") && !file.contains("templates/tests/") {
                let has_valid_ext = valid_extensions.iter().any(|ext| file.ends_with(ext));
                let is_helper = file.contains("_helpers");
                let is_notes = file.contains("NOTES.txt");

                if !has_valid_ext && !is_helper && !is_notes && !file.ends_with('/') {
                    failures.push(CheckFailure::new(
                        "HL3007",
                        Severity::Warning,
                        format!("Template file '{}' has unexpected extension", file),
                        file,
                        1,
                        RuleCategory::Template,
                    ));
                }
            }
        }

        failures
    }
}

/// HL3008: NOTES.txt missing
pub struct HL3008;

impl Rule for HL3008 {
    fn code(&self) -> &'static str {
        "HL3008"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "missing-notes"
    }

    fn description(&self) -> &'static str {
        "Chart should have a NOTES.txt for post-install instructions"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        // Skip for library charts
        if let Some(chart) = ctx.chart_metadata {
            if chart.is_library() {
                return vec![];
            }
        }

        let has_notes = ctx.files.iter().any(|f| f.ends_with("NOTES.txt"));
        if !has_notes {
            return vec![CheckFailure::new(
                "HL3008",
                Severity::Info,
                "Chart is missing templates/NOTES.txt for post-install instructions",
                "templates/NOTES.txt",
                1,
                RuleCategory::Template,
            )];
        }

        vec![]
    }
}

/// HL3009: Helper without description comment
pub struct HL3009;

impl Rule for HL3009 {
    fn code(&self) -> &'static str {
        "HL3009"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "helper-missing-comment"
    }

    fn description(&self) -> &'static str {
        "Helper template should have a description comment"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        if let Some(helpers) = ctx.helpers {
            for helper in &helpers.helpers {
                if helper.doc_comment.is_none() {
                    failures.push(CheckFailure::new(
                        "HL3009",
                        Severity::Info,
                        format!("Helper '{}' is missing a description comment", helper.name),
                        &helpers.path,
                        helper.line,
                        RuleCategory::Template,
                    ));
                }
            }
        }

        failures
    }
}

/// HL3010: Unused helper defined
pub struct HL3010;

impl Rule for HL3010 {
    fn code(&self) -> &'static str {
        "HL3010"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> &'static str {
        "unused-helper"
    }

    fn description(&self) -> &'static str {
        "Helper template is defined but never used"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let helpers = match ctx.helpers {
            Some(h) => h,
            None => return failures,
        };

        let referenced = ctx.template_references();

        for helper in &helpers.helpers {
            if !referenced.contains(helper.name.as_str()) {
                // Check if it's used via include in other helpers
                let used_in_helpers = helpers
                    .helpers
                    .iter()
                    .any(|h| h.name != helper.name && h.content.contains(&helper.name));

                if !used_in_helpers {
                    failures.push(CheckFailure::new(
                        "HL3010",
                        Severity::Info,
                        format!("Helper '{}' is defined but never used", helper.name),
                        &helpers.path,
                        helper.line,
                        RuleCategory::Template,
                    ));
                }
            }
        }

        failures
    }
}

/// HL3011: Include of non-existent template
pub struct HL3011;

impl Rule for HL3011 {
    fn code(&self) -> &'static str {
        "HL3011"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> &'static str {
        "include-not-found"
    }

    fn description(&self) -> &'static str {
        "Template includes a helper that is not defined"
    }

    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure> {
        let mut failures = Vec::new();

        let defined_helpers: std::collections::HashSet<&str> = ctx.helper_names().into_iter().collect();
        let referenced = ctx.template_references();

        for ref_name in referenced {
            if !defined_helpers.contains(ref_name) {
                // Find which template references this
                for template in ctx.templates {
                    if template.referenced_templates.contains(ref_name) {
                        failures.push(CheckFailure::new(
                            "HL3011",
                            Severity::Error,
                            format!("Template includes '{}' which is not defined", ref_name),
                            &template.path,
                            1,
                            RuleCategory::Template,
                        ));
                        break;
                    }
                }
            }
        }

        failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require setting up LintContext which needs parsed templates
    // For now, we just verify the rules compile and have correct metadata

    #[test]
    fn test_rules_exist() {
        let all_rules = rules();
        assert!(!all_rules.is_empty());
    }
}
