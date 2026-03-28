use crate::{
    analyzer::analyze_monorepo,
    analyzer::display::{
        ColorScheme as DisplayColorScheme, DisplayMode, display_analysis_to_string,
        display_analysis_with_return, init_color_adapter,
    },
    cli::{ColorScheme, DisplayFormat},
};

pub fn handle_analyze(
    path: std::path::PathBuf,
    json: bool,
    detailed: bool,
    display: Option<DisplayFormat>,
    _only: Option<Vec<String>>,
    color_scheme: Option<ColorScheme>,
    quiet: bool,
) -> crate::Result<String> {
    // Initialize color adapter based on user preference
    if let Some(scheme) = color_scheme {
        let display_scheme = match scheme {
            ColorScheme::Auto => {
                // Let the color adapter auto-detect
                DisplayColorScheme::Dark // This will be overridden by auto-detection in ColorAdapter::new()
            }
            ColorScheme::Dark => DisplayColorScheme::Dark,
            ColorScheme::Light => DisplayColorScheme::Light,
        };

        // Only initialize if not auto - auto-detection happens by default
        if !matches!(scheme, ColorScheme::Auto) {
            init_color_adapter(display_scheme);
        }
    }

    if !quiet {
        println!("🔍 Analyzing project: {}", path.display());
    }

    let monorepo_analysis = analyze_monorepo(&path)?;

    let mode = if json {
        DisplayMode::Json
    } else if detailed {
        // Legacy flag for backward compatibility
        DisplayMode::Detailed
    } else {
        match display {
            Some(DisplayFormat::Matrix) | None => DisplayMode::Matrix,
            Some(DisplayFormat::Detailed) => DisplayMode::Detailed,
            Some(DisplayFormat::Summary) => DisplayMode::Summary,
        }
    };

    let output = if quiet {
        display_analysis_to_string(&monorepo_analysis, mode)
    } else {
        display_analysis_with_return(&monorepo_analysis, mode)
    };

    Ok(output)
}
