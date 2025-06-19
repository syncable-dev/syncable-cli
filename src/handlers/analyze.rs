use crate::{
    analyzer::{analyze_monorepo},
    analyzer::display::{display_analysis_with_return, DisplayMode},
    cli::DisplayFormat,
};

pub fn handle_analyze(
    path: std::path::PathBuf,
    json: bool,
    detailed: bool,
    display: Option<DisplayFormat>,
    _only: Option<Vec<String>>,
) -> crate::Result<String> {
    println!("🔍 Analyzing project: {}", path.display());
    
    let monorepo_analysis = analyze_monorepo(&path)?;
    
    let output = if json {
        display_analysis_with_return(&monorepo_analysis, DisplayMode::Json)
    } else {
        // Determine display mode
        let mode = if detailed {
            // Legacy flag for backward compatibility
            DisplayMode::Detailed
        } else {
            match display {
                Some(DisplayFormat::Matrix) | None => DisplayMode::Matrix,
                Some(DisplayFormat::Detailed) => DisplayMode::Detailed,
                Some(DisplayFormat::Summary) => DisplayMode::Summary,
            }
        };
        
        display_analysis_with_return(&monorepo_analysis, mode)
    };
    
    Ok(output)
} 