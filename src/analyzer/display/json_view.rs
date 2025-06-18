//! JSON view display functionality

use crate::analyzer::MonorepoAnalysis;

/// Display JSON output
pub fn display_json_view(analysis: &MonorepoAnalysis) {
    match serde_json::to_string_pretty(analysis) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing to JSON: {}", e),
    }
}

/// Display JSON output - returns string
pub fn display_json_view_to_string(analysis: &MonorepoAnalysis) -> String {
    match serde_json::to_string_pretty(analysis) {
        Ok(json) => json,
        Err(e) => format!("Error serializing to JSON: {}", e),
    }
} 