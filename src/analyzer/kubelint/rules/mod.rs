//! Rule trait and utilities.
//!
//! Rules are the base abstraction for linting logic.
//! Templates implement rules with configurable parameters.

use crate::analyzer::kubelint::context::{LintContext, Object};
use crate::analyzer::kubelint::types::Diagnostic;
