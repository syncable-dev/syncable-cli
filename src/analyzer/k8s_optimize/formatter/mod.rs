//! Output formatting for optimization results.
//!
//! Supports multiple output formats: table, JSON, YAML, and plain text.

mod output;

pub use output::{OutputFormat, format_result, format_result_to_string};
