//! Test program to demonstrate color adaptation for different terminal backgrounds
//!
//! Run with: cargo run --example test_color_adaptation

use syncable_cli::analyzer::display::{ColorAdapter, ColorScheme};

fn main() {
    println!("🎨 Color Adaptation Test\n");
    println!("This example demonstrates how colors adapt to different terminal backgrounds.\n");

    // Test Dark Theme
    println!("{}", "=".repeat(80));
    println!("🌙 DARK TERMINAL THEME (Most terminals)");
    println!("{}", "=".repeat(80));

    let dark_adapter = ColorAdapter::with_scheme(ColorScheme::Dark);
    demonstrate_colors(&dark_adapter, "Dark");

    println!("\n");

    // Test Light Theme
    println!("{}", "=".repeat(80));
    println!(
        "☀️  LIGHT TERMINAL THEME (Light terminals like default macOS Terminal with light theme)"
    );
    println!("{}", "=".repeat(80));

    let light_adapter = ColorAdapter::with_scheme(ColorScheme::Light);
    demonstrate_colors(&light_adapter, "Light");

    println!("\n");

    // Test Auto-detection
    println!("{}", "=".repeat(80));
    println!("🔍 AUTO-DETECTED THEME (Your current terminal)");
    println!("{}", "=".repeat(80));

    let auto_adapter = ColorAdapter::new();
    let detected_scheme = match auto_adapter.scheme() {
        ColorScheme::Dark => "Dark",
        ColorScheme::Light => "Light",
    };
    println!("Detected scheme: {}\n", detected_scheme);
    demonstrate_colors(&auto_adapter, detected_scheme);

    println!("\n🏁 Test complete! Colors should be more readable on your terminal background.");
    println!("💡 Tip: Use --color-scheme light/dark/auto to override detection in the CLI");
}

fn demonstrate_colors(adapter: &ColorAdapter, theme_name: &str) {
    println!("Theme: {}\n", theme_name);

    // Headers and borders
    println!("📊 Headers and Borders:");
    println!(
        "  Header: {}",
        adapter.header_text("PROJECT ANALYSIS DASHBOARD")
    );
    println!(
        "  Border: {}",
        adapter.border("═══════════════════════════")
    );
    println!();

    // Labels and values (the main fix)
    println!("🏷️  Labels (Fixed Issue):");
    println!(
        "  {}: {}",
        adapter.label("Type"),
        adapter.value("Single Project")
    );
    println!(
        "  {}: {}",
        adapter.label("Pattern"),
        adapter.value("Monolithic")
    );
    println!(
        "  {}: {}",
        adapter.label("Languages"),
        adapter.value("JavaScript, TypeScript")
    );
    println!("  {}: {}", adapter.label("Dockerfiles"), adapter.value("0"));
    println!("  ^ These labels should now be readable on both backgrounds!");
    println!();

    // Primary content colors
    println!("🎯 Primary Content:");
    println!("  Primary: {}", adapter.primary("Main Project Name"));
    println!(
        "  Secondary: {}",
        adapter.secondary("Microservice Architecture")
    );
    println!();

    // Technology stack colors
    println!("🛠️  Technology Stack:");
    println!(
        "  Languages: {}",
        adapter.language("Rust, TypeScript, Python")
    );
    println!(
        "  Frameworks: {}",
        adapter.framework("Actix-web, Next.js, FastAPI")
    );
    println!("  Databases: {}", adapter.database("PostgreSQL, Redis"));
    println!(
        "  Technologies: {}",
        adapter.technology("Docker, Kubernetes")
    );
    println!();

    // Status colors
    println!("📈 Status Indicators:");
    println!(
        "  Info: {}",
        adapter.info("Analysis completed successfully")
    );
    println!("  Success: {}", adapter.success("All tests passed"));
    println!(
        "  Warning: {}",
        adapter.warning("Some dependencies are outdated")
    );
    println!("  Error: {}", adapter.error("Configuration file not found"));
    println!();

    // Additional labels and values
    println!("📋 Additional Labels:");
    println!(
        "  {}: {}",
        adapter.label("Project Type"),
        adapter.value("Full-stack Application")
    );
    println!(
        "  {}: {}",
        adapter.label("Confidence"),
        adapter.value("95.2%")
    );
    println!();

    // Architecture and metrics
    println!("🏗️  Architecture & Metrics:");
    println!(
        "  Architecture: {}",
        adapter.architecture_pattern("Microservices")
    );
    println!(
        "  Project Type: {}",
        adapter.project_type("Monorepo (3 projects)")
    );
    println!(
        "  Metrics: {}",
        adapter.metric("Duration: 2.3s | Files: 1,247")
    );
    println!("  Path: {}", adapter.path("/Users/dev/my-project"));
    println!();

    // Confidence levels
    println!("📊 Confidence Levels:");
    println!(
        "  High: {}",
        adapter.confidence_high("Framework detection (98%)")
    );
    println!(
        "  Medium: {}",
        adapter.confidence_medium("Database detection (72%)")
    );
    println!(
        "  Low: {}",
        adapter.confidence_low("Microservice detection (45%)")
    );
    println!();

    // Dimmed text
    println!("💭 Additional Info:");
    println!(
        "  Note: {}",
        adapter.dimmed("This is supplementary information")
    );
    println!();
}
