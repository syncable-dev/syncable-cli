//! # Display Module
//!
//! Provides improved CLI output formatting with matrix/dashboard views for better readability
//! and easier parsing by both humans and LLMs.

// Sub-modules
mod box_drawer;
mod color_adapter;
mod detailed_view;
mod helpers;
mod json_view;
mod matrix_view;
mod summary_view;
mod utils;

// Re-export public items
pub use box_drawer::BoxDrawer;
pub use color_adapter::{ColorAdapter, ColorScheme, get_color_adapter, init_color_adapter};
pub use helpers::{format_project_category, get_category_emoji};
pub use utils::{strip_ansi_codes, truncate_to_width, visual_width};

use crate::analyzer::MonorepoAnalysis;

/// Display mode for analysis output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    /// Compact matrix view (default)
    Matrix,
    /// Detailed vertical view (legacy)
    Detailed,
    /// Summary only
    Summary,
    /// JSON output
    Json,
}

/// Main display function that routes to appropriate formatter
pub fn display_analysis(analysis: &MonorepoAnalysis, mode: DisplayMode) {
    match mode {
        DisplayMode::Matrix => matrix_view::display_matrix_view(analysis),
        DisplayMode::Detailed => detailed_view::display_detailed_view(analysis),
        DisplayMode::Summary => summary_view::display_summary_view(analysis),
        DisplayMode::Json => json_view::display_json_view(analysis),
    }
}

/// Main display function that returns a string instead of printing
pub fn display_analysis_to_string(analysis: &MonorepoAnalysis, mode: DisplayMode) -> String {
    match mode {
        DisplayMode::Matrix => matrix_view::display_matrix_view_to_string(analysis),
        DisplayMode::Detailed => detailed_view::display_detailed_view_to_string(analysis),
        DisplayMode::Summary => summary_view::display_summary_view_to_string(analysis),
        DisplayMode::Json => json_view::display_json_view_to_string(analysis),
    }
}

/// Combined function that both prints and returns a string
pub fn display_analysis_with_return(analysis: &MonorepoAnalysis, mode: DisplayMode) -> String {
    let output = display_analysis_to_string(analysis, mode);
    print!("{}", output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_modes() {
        // Test that display modes are properly defined
        assert_eq!(DisplayMode::Matrix, DisplayMode::Matrix);
        assert_ne!(DisplayMode::Matrix, DisplayMode::Detailed);
    }
}
