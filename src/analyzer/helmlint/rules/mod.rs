//! Rule system for helmlint.
//!
//! Provides the infrastructure for defining and running Helm chart linting rules.
//!
//! # Rule Categories
//!
//! - **HL1xxx**: Chart structure rules (Chart.yaml, file structure)
//! - **HL2xxx**: Values validation rules (values.yaml)
//! - **HL3xxx**: Template syntax rules (Go templates)
//! - **HL4xxx**: Security rules (container security)
//! - **HL5xxx**: Best practice rules (K8s best practices)

pub mod hl1xxx;
pub mod hl2xxx;
pub mod hl3xxx;
pub mod hl4xxx;
pub mod hl5xxx;

use std::collections::HashSet;
use std::path::Path;

use crate::analyzer::helmlint::parser::chart::ChartMetadata;
use crate::analyzer::helmlint::parser::helpers::ParsedHelpers;
use crate::analyzer::helmlint::parser::template::ParsedTemplate;
use crate::analyzer::helmlint::parser::values::ValuesFile;
use crate::analyzer::helmlint::types::{CheckFailure, Severity};

/// Context for running lint rules.
#[derive(Debug)]
pub struct LintContext<'a> {
    /// Path to the chart root directory.
    pub chart_path: &'a Path,
    /// Parsed Chart.yaml (if available).
    pub chart_metadata: Option<&'a ChartMetadata>,
    /// Parsed values.yaml (if available).
    pub values: Option<&'a ValuesFile>,
    /// Parsed helper templates.
    pub helpers: Option<&'a ParsedHelpers>,
    /// All parsed templates.
    pub templates: &'a [ParsedTemplate],
    /// All files in the chart.
    pub files: &'a HashSet<String>,
    /// All value references found in templates.
    pub template_value_refs: HashSet<String>,
}

impl<'a> LintContext<'a> {
    /// Create a new lint context.
    pub fn new(
        chart_path: &'a Path,
        chart_metadata: Option<&'a ChartMetadata>,
        values: Option<&'a ValuesFile>,
        helpers: Option<&'a ParsedHelpers>,
        templates: &'a [ParsedTemplate],
        files: &'a HashSet<String>,
    ) -> Self {
        // Collect all value references from templates
        let mut template_value_refs = HashSet::new();
        for template in templates {
            for var in &template.variables_used {
                if let Some(path) = var.strip_prefix(".Values.") {
                    template_value_refs.insert(path.to_string());
                }
            }
        }

        Self {
            chart_path,
            chart_metadata,
            values,
            helpers,
            templates,
            files,
            template_value_refs,
        }
    }

    /// Check if a file exists in the chart.
    pub fn has_file(&self, name: &str) -> bool {
        self.files.contains(name) || self.files.iter().any(|f| f.ends_with(name))
    }

    /// Check if a helper is defined.
    pub fn has_helper(&self, name: &str) -> bool {
        self.helpers.map(|h| h.has_helper(name)).unwrap_or(false)
    }

    /// Get all defined helper names.
    pub fn helper_names(&self) -> Vec<&str> {
        self.helpers
            .map(|h| h.names().collect())
            .unwrap_or_default()
    }

    /// Get all template references (from include/template calls).
    pub fn template_references(&self) -> HashSet<&str> {
        let mut refs = HashSet::new();
        for template in self.templates {
            for name in &template.referenced_templates {
                refs.insert(name.as_str());
            }
        }
        refs
    }
}

/// A lint rule that can check Helm charts.
pub trait Rule: Send + Sync {
    /// Get the rule code (e.g., "HL1001").
    fn code(&self) -> &'static str;

    /// Get the default severity.
    fn severity(&self) -> Severity;

    /// Get the rule name.
    fn name(&self) -> &'static str;

    /// Get the rule description.
    fn description(&self) -> &'static str;

    /// Check if this rule can be auto-fixed.
    fn is_fixable(&self) -> bool {
        false
    }

    /// Run the rule and return any violations.
    fn check(&self, ctx: &LintContext) -> Vec<CheckFailure>;
}

/// Get all available rules.
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();

    // HL1xxx - Chart Structure Rules
    rules.extend(hl1xxx::rules());

    // HL2xxx - Values Validation Rules
    rules.extend(hl2xxx::rules());

    // HL3xxx - Template Syntax Rules
    rules.extend(hl3xxx::rules());

    // HL4xxx - Security Rules
    rules.extend(hl4xxx::rules());

    // HL5xxx - Best Practice Rules
    rules.extend(hl5xxx::rules());

    rules
}

/// Get a rule by code.
pub fn get_rule(code: &str) -> Option<Box<dyn Rule>> {
    all_rules().into_iter().find(|r| r.code() == code)
}

/// List all rule codes.
pub fn list_rule_codes() -> Vec<&'static str> {
    vec![
        // HL1xxx
        "HL1001", "HL1002", "HL1003", "HL1004", "HL1005", "HL1006", "HL1007", "HL1008", "HL1009",
        "HL1010", "HL1011", "HL1012", "HL1013", "HL1014", "HL1015", "HL1016", "HL1017",
        // HL2xxx
        "HL2001", "HL2002", "HL2003", "HL2004", "HL2005", "HL2006", "HL2007", "HL2008", "HL2009",
        // HL3xxx
        "HL3001", "HL3002", "HL3003", "HL3004", "HL3005", "HL3006", "HL3007", "HL3008", "HL3009",
        "HL3010", "HL3011", // HL4xxx
        "HL4001", "HL4002", "HL4003", "HL4004", "HL4005", "HL4006", "HL4007", "HL4008", "HL4009",
        "HL4010", "HL4011", "HL4012", // HL5xxx
        "HL5001", "HL5002", "HL5003", "HL5004", "HL5005", "HL5006",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_rules_returns_rules() {
        let rules = all_rules();
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_rule_codes_unique() {
        let rules = all_rules();
        let mut codes = HashSet::new();
        for rule in rules {
            let code = rule.code();
            assert!(codes.insert(code), "Duplicate rule code: {}", code);
        }
    }
}
