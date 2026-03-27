//! Rule system framework for hadolint-rs.
//!
//! Provides the infrastructure for defining and running Dockerfile linting rules.
//! The design matches hadolint's fold-based architecture:
//!
//! - `simple_rule` - Stateless rules that check each instruction independently
//! - `custom_rule` - Stateful rules that accumulate state across instructions
//! - `very_custom_rule` - Rules with custom finalization logic
//! - `onbuild` - Wrapper to also check ONBUILD-wrapped instructions

use crate::analyzer::hadolint::parser::instruction::Instruction;
use crate::analyzer::hadolint::shell::ParsedShell;
use crate::analyzer::hadolint::types::{CheckFailure, RuleCode, Severity};

pub mod dl1001;
pub mod dl3000;
pub mod dl3001;
pub mod dl3002;
pub mod dl3003;
pub mod dl3004;
pub mod dl3005;
pub mod dl3006;
pub mod dl3007;
pub mod dl3008;
pub mod dl3009;
pub mod dl3010;
pub mod dl3011;
pub mod dl3012;
pub mod dl3013;
pub mod dl3014;
pub mod dl3015;
pub mod dl3016;
pub mod dl3017;
pub mod dl3018;
pub mod dl3019;
pub mod dl3020;
pub mod dl3021;
pub mod dl3022;
pub mod dl3023;
pub mod dl3024;
pub mod dl3025;
pub mod dl3026;
pub mod dl3027;
pub mod dl3028;
pub mod dl3029;
pub mod dl3030;
pub mod dl3031;
pub mod dl3032;
pub mod dl3033;
pub mod dl3034;
pub mod dl3035;
pub mod dl3036;
pub mod dl3037;
pub mod dl3038;
pub mod dl3039;
pub mod dl3040;
pub mod dl3041;
pub mod dl3042;
pub mod dl3043;
pub mod dl3044;
pub mod dl3045;
pub mod dl3046;
pub mod dl3047;
pub mod dl3048;
pub mod dl3049;
pub mod dl3050;
pub mod dl3051;
pub mod dl3052;
pub mod dl3053;
pub mod dl3054;
pub mod dl3055;
pub mod dl3056;
pub mod dl3057;
pub mod dl3058;
pub mod dl3059;
pub mod dl3060;
pub mod dl3061;
pub mod dl3062;
pub mod dl4000;
pub mod dl4001;
pub mod dl4003;
pub mod dl4004;
pub mod dl4005;
pub mod dl4006;

/// A rule that can check Dockerfile instructions.
pub trait Rule: Send + Sync {
    /// Check an instruction and potentially add failures to the state.
    fn check(
        &self,
        state: &mut RuleState,
        line: u32,
        instruction: &Instruction,
        shell: Option<&ParsedShell>,
    );

    /// Finalize the rule and return any additional failures.
    /// Called after all instructions have been processed.
    fn finalize(&self, state: RuleState) -> Vec<CheckFailure> {
        state.failures
    }

    /// Get the rule code.
    fn code(&self) -> &RuleCode;

    /// Get the default severity.
    fn severity(&self) -> Severity;

    /// Get the rule message.
    fn message(&self) -> &str;
}

/// State for rule execution.
#[derive(Debug, Clone, Default)]
pub struct RuleState {
    /// Accumulated failures.
    pub failures: Vec<CheckFailure>,
    /// Custom state data (serialized).
    pub data: RuleData,
}

impl RuleState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a failure.
    pub fn add_failure(
        &mut self,
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        line: u32,
    ) {
        self.failures
            .push(CheckFailure::new(code, severity, message, line));
    }
}

/// Custom data storage for stateful rules.
#[derive(Debug, Clone, Default)]
pub struct RuleData {
    /// Integer values.
    pub ints: std::collections::HashMap<&'static str, i64>,
    /// Boolean values.
    pub bools: std::collections::HashMap<&'static str, bool>,
    /// String values.
    pub strings: std::collections::HashMap<&'static str, String>,
    /// String set values.
    pub string_sets: std::collections::HashMap<&'static str, std::collections::HashSet<String>>,
}

impl RuleData {
    pub fn get_int(&self, key: &'static str) -> i64 {
        self.ints.get(key).copied().unwrap_or(0)
    }

    pub fn set_int(&mut self, key: &'static str, value: i64) {
        self.ints.insert(key, value);
    }

    pub fn get_bool(&self, key: &'static str) -> bool {
        self.bools.get(key).copied().unwrap_or(false)
    }

    pub fn set_bool(&mut self, key: &'static str, value: bool) {
        self.bools.insert(key, value);
    }

    pub fn get_string(&self, key: &'static str) -> Option<&str> {
        self.strings.get(key).map(|s| s.as_str())
    }

    pub fn set_string(&mut self, key: &'static str, value: impl Into<String>) {
        self.strings.insert(key, value.into());
    }

    pub fn get_string_set(&self, key: &'static str) -> Option<&std::collections::HashSet<String>> {
        self.string_sets.get(key)
    }

    pub fn insert_to_set(&mut self, key: &'static str, value: impl Into<String>) {
        self.string_sets
            .entry(key)
            .or_default()
            .insert(value.into());
    }

    pub fn set_contains(&self, key: &'static str, value: &str) -> bool {
        self.string_sets
            .get(key)
            .map(|s| s.contains(value))
            .unwrap_or(false)
    }
}

/// A simple stateless rule.
pub struct SimpleRule<F>
where
    F: Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync,
{
    code: RuleCode,
    severity: Severity,
    message: String,
    check_fn: F,
}

impl<F> SimpleRule<F>
where
    F: Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync,
{
    /// Create a new simple rule.
    pub fn new(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        check_fn: F,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            check_fn,
        }
    }
}

impl<F> Rule for SimpleRule<F>
where
    F: Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync,
{
    fn check(
        &self,
        state: &mut RuleState,
        line: u32,
        instruction: &Instruction,
        shell: Option<&ParsedShell>,
    ) {
        if !(self.check_fn)(instruction, shell) {
            state.add_failure(self.code.clone(), self.severity, self.message.clone(), line);
        }
    }

    fn code(&self) -> &RuleCode {
        &self.code
    }

    fn severity(&self) -> Severity {
        self.severity
    }

    fn message(&self) -> &str {
        &self.message
    }
}

/// Create a simple stateless rule.
pub fn simple_rule<F>(
    code: impl Into<RuleCode>,
    severity: Severity,
    message: impl Into<String>,
    check_fn: F,
) -> SimpleRule<F>
where
    F: Fn(&Instruction, Option<&ParsedShell>) -> bool + Send + Sync,
{
    SimpleRule::new(code, severity, message, check_fn)
}

/// A stateful rule with custom step function.
pub struct CustomRule<F>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
{
    code: RuleCode,
    severity: Severity,
    message: String,
    step_fn: F,
}

impl<F> CustomRule<F>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
{
    /// Create a new custom rule.
    pub fn new(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        step_fn: F,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            step_fn,
        }
    }
}

impl<F> Rule for CustomRule<F>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
{
    fn check(
        &self,
        state: &mut RuleState,
        line: u32,
        instruction: &Instruction,
        shell: Option<&ParsedShell>,
    ) {
        (self.step_fn)(state, line, instruction, shell);
    }

    fn code(&self) -> &RuleCode {
        &self.code
    }

    fn severity(&self) -> Severity {
        self.severity
    }

    fn message(&self) -> &str {
        &self.message
    }
}

/// Create a custom stateful rule.
pub fn custom_rule<F>(
    code: impl Into<RuleCode>,
    severity: Severity,
    message: impl Into<String>,
    step_fn: F,
) -> CustomRule<F>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
{
    CustomRule::new(code, severity, message, step_fn)
}

/// A rule with custom finalization.
pub struct VeryCustomRule<F, D>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
    D: Fn(RuleState) -> Vec<CheckFailure> + Send + Sync,
{
    code: RuleCode,
    severity: Severity,
    message: String,
    step_fn: F,
    done_fn: D,
}

impl<F, D> VeryCustomRule<F, D>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
    D: Fn(RuleState) -> Vec<CheckFailure> + Send + Sync,
{
    /// Create a new very custom rule.
    pub fn new(
        code: impl Into<RuleCode>,
        severity: Severity,
        message: impl Into<String>,
        step_fn: F,
        done_fn: D,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            step_fn,
            done_fn,
        }
    }
}

impl<F, D> Rule for VeryCustomRule<F, D>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
    D: Fn(RuleState) -> Vec<CheckFailure> + Send + Sync,
{
    fn check(
        &self,
        state: &mut RuleState,
        line: u32,
        instruction: &Instruction,
        shell: Option<&ParsedShell>,
    ) {
        (self.step_fn)(state, line, instruction, shell);
    }

    fn finalize(&self, state: RuleState) -> Vec<CheckFailure> {
        (self.done_fn)(state)
    }

    fn code(&self) -> &RuleCode {
        &self.code
    }

    fn severity(&self) -> Severity {
        self.severity
    }

    fn message(&self) -> &str {
        &self.message
    }
}

/// Create a rule with custom finalization.
pub fn very_custom_rule<F, D>(
    code: impl Into<RuleCode>,
    severity: Severity,
    message: impl Into<String>,
    step_fn: F,
    done_fn: D,
) -> VeryCustomRule<F, D>
where
    F: Fn(&mut RuleState, u32, &Instruction, Option<&ParsedShell>) + Send + Sync,
    D: Fn(RuleState) -> Vec<CheckFailure> + Send + Sync,
{
    VeryCustomRule::new(code, severity, message, step_fn, done_fn)
}

/// Get all enabled rules.
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        // DL1xxx rules (deprecation warnings)
        Box::new(dl1001::rule()),
        // Simple DL3xxx rules
        Box::new(dl3000::rule()),
        Box::new(dl3001::rule()),
        Box::new(dl3003::rule()),
        Box::new(dl3004::rule()),
        Box::new(dl3005::rule()),
        Box::new(dl3007::rule()),
        Box::new(dl3010::rule()),
        Box::new(dl3011::rule()),
        Box::new(dl3017::rule()),
        Box::new(dl3020::rule()),
        Box::new(dl3021::rule()),
        Box::new(dl3025::rule()),
        Box::new(dl3026::rule()),
        Box::new(dl3027::rule()),
        Box::new(dl3029::rule()),
        Box::new(dl3031::rule()),
        Box::new(dl3035::rule()),
        Box::new(dl3039::rule()),
        Box::new(dl3043::rule()),
        Box::new(dl3044::rule()),
        Box::new(dl3046::rule()),
        Box::new(dl3048::rule()),
        Box::new(dl3049::rule()),
        Box::new(dl3050::rule()),
        Box::new(dl3051::rule()),
        Box::new(dl3052::rule()),
        Box::new(dl3053::rule()),
        Box::new(dl3054::rule()),
        Box::new(dl3055::rule()),
        Box::new(dl3056::rule()),
        Box::new(dl3058::rule()),
        Box::new(dl3061::rule()),
        // DL4xxx simple rules
        Box::new(dl4000::rule()),
        Box::new(dl4005::rule()),
        Box::new(dl4006::rule()),
        // Stateful rules
        Box::new(dl3002::rule()),
        Box::new(dl3006::rule()),
        Box::new(dl3012::rule()),
        Box::new(dl3022::rule()),
        Box::new(dl3023::rule()),
        Box::new(dl3024::rule()),
        Box::new(dl3045::rule()),
        Box::new(dl3047::rule()),
        Box::new(dl3057::rule()),
        Box::new(dl3059::rule()),
        Box::new(dl3062::rule()),
        Box::new(dl4001::rule()),
        Box::new(dl4003::rule()),
        Box::new(dl4004::rule()),
        // Shell-dependent rules
        Box::new(dl3008::rule()),
        Box::new(dl3009::rule()),
        Box::new(dl3013::rule()),
        Box::new(dl3014::rule()),
        Box::new(dl3015::rule()),
        Box::new(dl3016::rule()),
        Box::new(dl3018::rule()),
        Box::new(dl3019::rule()),
        Box::new(dl3028::rule()),
        Box::new(dl3030::rule()),
        Box::new(dl3032::rule()),
        Box::new(dl3033::rule()),
        Box::new(dl3034::rule()),
        Box::new(dl3036::rule()),
        Box::new(dl3037::rule()),
        Box::new(dl3038::rule()),
        Box::new(dl3040::rule()),
        Box::new(dl3041::rule()),
        Box::new(dl3042::rule()),
        Box::new(dl3060::rule()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_rule() {
        let rule = simple_rule("TEST001", Severity::Warning, "Test message", |instr, _| {
            !matches!(instr, Instruction::Maintainer(_))
        });

        let mut state = RuleState::new();
        let instr = Instruction::Maintainer("test".to_string());
        rule.check(&mut state, 1, &instr, None);

        assert_eq!(state.failures.len(), 1);
        assert_eq!(state.failures[0].code.as_str(), "TEST001");
    }

    #[test]
    fn test_rule_data() {
        let mut data = RuleData::default();

        data.set_int("count", 5);
        assert_eq!(data.get_int("count"), 5);

        data.set_bool("seen", true);
        assert!(data.get_bool("seen"));

        data.set_string("name", "test");
        assert_eq!(data.get_string("name"), Some("test"));

        data.insert_to_set("aliases", "builder");
        assert!(data.set_contains("aliases", "builder"));
        assert!(!data.set_contains("aliases", "runner"));
    }
}
