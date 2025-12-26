//! Color adaptation for different terminal backgrounds
//!
//! This module provides color schemes that work well on both light and dark terminal backgrounds,
//! ensuring good readability regardless of the user's terminal theme.

use colored::*;
use std::env;

/// Represents the detected or configured terminal background type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorScheme {
    /// Dark background terminal (default assumption)
    Dark,
    /// Light background terminal
    Light,
}

/// Color adapter that provides appropriate colors based on terminal background
#[derive(Debug, Clone)]
pub struct ColorAdapter {
    scheme: ColorScheme,
}

impl ColorAdapter {
    /// Create a new ColorAdapter with automatic background detection
    pub fn new() -> Self {
        Self {
            scheme: Self::detect_terminal_background(),
        }
    }

    /// Create a ColorAdapter with a specific color scheme
    pub fn with_scheme(scheme: ColorScheme) -> Self {
        Self { scheme }
    }

    /// Detect terminal background based on environment variables and heuristics
    fn detect_terminal_background() -> ColorScheme {
        // Check COLORFGBG environment variable (format: "foreground;background")
        if let Ok(colorfgbg) = env::var("COLORFGBG") {
            if let Some(bg_str) = colorfgbg.split(';').nth(1) {
                if let Ok(bg_code) = bg_str.parse::<u8>() {
                    // Background colors 0-6 are dark, 7-15 are light/bright
                    // Be more aggressive about detecting light backgrounds
                    return if bg_code >= 7 {
                        ColorScheme::Light
                    } else {
                        ColorScheme::Dark
                    };
                }
            }
        }

        // Check for common light terminal setups
        if let Ok(term_program) = env::var("TERM_PROGRAM") {
            match term_program.as_str() {
                "Apple_Terminal" => {
                    // macOS Terminal.app - check for light theme indicators
                    // Many users have light themes, so be more aggressive
                    if let Ok(_term_session_id) = env::var("TERM_SESSION_ID") {
                        // If we can't detect definitively, assume light for Terminal.app
                        // since many users use the default light theme
                        return ColorScheme::Light;
                    }
                    return ColorScheme::Light; // Default to light for Terminal.app
                }
                "iTerm.app" => {
                    // iTerm2 - check for theme hints
                    if let Ok(_iterm_session_id) = env::var("ITERM_SESSION_ID") {
                        // Default to dark for iTerm as it's more commonly used with dark themes
                        return ColorScheme::Dark;
                    }
                }
                "vscode" | "code" => {
                    // VS Code integrated terminal - often follows editor theme
                    // VS Code light themes are common
                    return ColorScheme::Light;
                }
                _ => {}
            }
        }

        // Check terminal type and name hints
        if let Ok(term) = env::var("TERM") {
            match term.as_str() {
                term if term.contains("light") => return ColorScheme::Light,
                term if term.contains("256color") => {
                    // Modern terminals with 256 color support
                    // Check other indicators
                    if env::var("TERM_PROGRAM")
                        .unwrap_or_default()
                        .contains("Terminal")
                    {
                        return ColorScheme::Light; // macOS Terminal default
                    }
                }
                _ => {}
            }
        }

        // Check for SSH session - often indicates server/dark environment
        if env::var("SSH_CONNECTION").is_ok() || env::var("SSH_CLIENT").is_ok() {
            return ColorScheme::Dark;
        }

        // Check background color hints from other variables
        if let Ok(bg_hint) = env::var("BACKGROUND") {
            match bg_hint.to_lowercase().as_str() {
                "light" | "white" => return ColorScheme::Light,
                "dark" | "black" => return ColorScheme::Dark,
                _ => {}
            }
        }

        // More aggressive light detection for common desktop environments
        if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
            match desktop.to_lowercase().as_str() {
                "gnome" | "kde" | "xfce" => {
                    // Desktop environments often use light themes by default
                    return ColorScheme::Light;
                }
                _ => {}
            }
        }

        // Check if we're in a GUI environment (more likely to be light)
        if env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok() {
            // GUI environment - light themes are common
            // But don't override if we have other strong indicators
            if env::var("TERM_PROGRAM").is_err() {
                return ColorScheme::Light;
            }
        }

        // Default fallback - if we can't detect, prefer dark (most terminals)
        // But add a bias toward light for macOS users
        #[cfg(target_os = "macos")]
        {
            return ColorScheme::Light; // macOS Terminal.app default is light
        }

        #[cfg(not(target_os = "macos"))]
        {
            return ColorScheme::Dark; // Most other platforms default to dark
        }
    }

    /// Get the current color scheme
    pub fn scheme(&self) -> ColorScheme {
        self.scheme
    }

    // Header and borders
    pub fn header_text(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.bright_white().bold(),
            ColorScheme::Light => text.black().bold(),
        }
    }

    pub fn border(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.bright_blue(),
            ColorScheme::Light => text.blue(),
        }
    }

    // Primary content colors
    pub fn primary(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.yellow(),
            ColorScheme::Light => text.red().bold(),
        }
    }

    pub fn secondary(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.green(),
            ColorScheme::Light => text.green().bold(),
        }
    }

    // Technology stack colors
    pub fn language(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.blue(),
            ColorScheme::Light => text.blue().bold(),
        }
    }

    pub fn framework(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.magenta(),
            ColorScheme::Light => text.magenta().bold(),
        }
    }

    pub fn database(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.cyan(),
            ColorScheme::Light => text.cyan().bold(),
        }
    }

    pub fn technology(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.magenta(),
            ColorScheme::Light => text.purple().bold(),
        }
    }

    // Status and metadata colors
    pub fn info(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.cyan(),
            ColorScheme::Light => text.blue().bold(),
        }
    }

    pub fn success(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.green(),
            ColorScheme::Light => text.green().bold(),
        }
    }

    pub fn warning(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.yellow(),
            ColorScheme::Light => text.red(),
        }
    }

    pub fn error(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.red(),
            ColorScheme::Light => text.red().bold(),
        }
    }

    // Label colors (for key-value pairs)
    pub fn label(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.bright_white(),
            ColorScheme::Light => text.black().bold(),
        }
    }

    pub fn value(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.white(),
            ColorScheme::Light => text.black(),
        }
    }

    // Dimmed/subtle text
    pub fn dimmed(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.dimmed(),
            ColorScheme::Light => text.dimmed(),
        }
    }

    // Architecture pattern colors
    pub fn architecture_pattern(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.green(),
            ColorScheme::Light => text.green().bold(),
        }
    }

    // Project type colors
    pub fn project_type(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.yellow(),
            ColorScheme::Light => text.red().bold(),
        }
    }

    // Metrics and numbers
    pub fn metric(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.cyan(),
            ColorScheme::Light => text.blue().bold(),
        }
    }

    // File paths and names
    pub fn path(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.cyan().bold(),
            ColorScheme::Light => text.blue().bold(),
        }
    }

    // Confidence indicators
    pub fn confidence_high(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.green(),
            ColorScheme::Light => text.green().bold(),
        }
    }

    pub fn confidence_medium(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.yellow(),
            ColorScheme::Light => text.red(),
        }
    }

    pub fn confidence_low(&self, text: &str) -> ColoredString {
        match self.scheme {
            ColorScheme::Dark => text.red(),
            ColorScheme::Light => text.red().bold(),
        }
    }
}

impl Default for ColorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Global color adapter instance
static COLOR_ADAPTER: std::sync::OnceLock<ColorAdapter> = std::sync::OnceLock::new();

/// Get the global color adapter instance
pub fn get_color_adapter() -> &'static ColorAdapter {
    COLOR_ADAPTER.get_or_init(ColorAdapter::new)
}

/// Initialize the global color adapter with a specific scheme
pub fn init_color_adapter(scheme: ColorScheme) {
    let _ = COLOR_ADAPTER.set(ColorAdapter::with_scheme(scheme));
}

/// Helper macro for quick color access
#[macro_export]
macro_rules! color {
    ($method:ident, $text:expr) => {
        $crate::analyzer::display::color_adapter::get_color_adapter().$method($text)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_adapter_creation() {
        let adapter = ColorAdapter::new();
        assert!(matches!(
            adapter.scheme(),
            ColorScheme::Dark | ColorScheme::Light
        ));
    }

    #[test]
    #[ignore] // Flaky in CI - color codes stripped without terminal
    fn test_color_scheme_specific() {
        let dark_adapter = ColorAdapter::with_scheme(ColorScheme::Dark);
        let light_adapter = ColorAdapter::with_scheme(ColorScheme::Light);

        assert_eq!(dark_adapter.scheme(), ColorScheme::Dark);
        assert_eq!(light_adapter.scheme(), ColorScheme::Light);

        // Test that different schemes produce different outputs
        let test_text = "test";
        let dark_result = dark_adapter.header_text(test_text).to_string();
        let light_result = light_adapter.header_text(test_text).to_string();

        // Results should be different (due to different color codes)
        assert_ne!(dark_result, light_result);
    }

    #[test]
    fn test_color_methods() {
        let adapter = ColorAdapter::with_scheme(ColorScheme::Dark);
        let text = "test";

        // Ensure all color methods work without panicking
        let _ = adapter.header_text(text);
        let _ = adapter.border(text);
        let _ = adapter.primary(text);
        let _ = adapter.secondary(text);
        let _ = adapter.language(text);
        let _ = adapter.framework(text);
        let _ = adapter.database(text);
        let _ = adapter.technology(text);
        let _ = adapter.info(text);
        let _ = adapter.success(text);
        let _ = adapter.warning(text);
        let _ = adapter.error(text);
        let _ = adapter.label(text);
        let _ = adapter.value(text);
        let _ = adapter.dimmed(text);
        let _ = adapter.architecture_pattern(text);
        let _ = adapter.project_type(text);
        let _ = adapter.metric(text);
        let _ = adapter.path(text);
        let _ = adapter.confidence_high(text);
        let _ = adapter.confidence_medium(text);
        let _ = adapter.confidence_low(text);
    }
}
