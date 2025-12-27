use clap::Parser;
use syncable_cli::{
    analyzer::{self, analyze_monorepo, vulnerability::VulnerabilitySeverity},
    cli::{
        ChatProvider, Cli, ColorScheme, Commands, DisplayFormat, OutputFormat, SecurityScanMode,
        SeverityThreshold, ToolsCommand,
    },
    config, generator,
    telemetry::{self},
};

use colored::Colorize;
use dirs::cache_dir;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, SystemTime};
use syncable_cli::analyzer::display::BoxDrawer;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> syncable_cli::Result<()> {
    let cli = Cli::parse();

    // Handle update cache clearing
    if cli.clear_update_cache {
        clear_update_cache();
        println!("✅ Update cache cleared. Checking for updates now...");
    }

    // Suppress update banner when JSON output is requested
    let suppress_update_banner = cli.json
        || matches!(
            &cli.command,
            Commands::Analyze { json: true, .. }
                | Commands::Dependencies {
                    format: OutputFormat::Json,
                    ..
                }
                | Commands::Vulnerabilities {
                    format: OutputFormat::Json,
                    ..
                }
                | Commands::Security {
                    format: OutputFormat::Json,
                    ..
                }
                | Commands::Tools {
                    command: ToolsCommand::Status {
                        format: OutputFormat::Json,
                        ..
                    }
                }
        );
    check_for_update(suppress_update_banner).await;

    // Initialize logging
    cli.init_logging();

    log::debug!("Loading configuration...");

    // Load configuration
    let mut config = match config::load_config(cli.config.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    log::debug!(
        "Configuration loaded: telemetry enabled = {}",
        config.telemetry.enabled
    );

    // Override telemetry setting if CLI flag is set
    if cli.disable_telemetry {
        config.telemetry.enabled = false;
    }

    log::debug!("Initializing telemetry...");

    // Initialize telemetry
    if let Err(e) = telemetry::init_telemetry(&config).await {
        log::warn!("Failed to initialize telemetry: {}", e);
    } else {
        log::debug!("Telemetry initialized successfully");
    }

    // Check if telemetry client is available
    if telemetry::get_telemetry_client().is_some() {
        log::debug!("Telemetry client is available");
    } else {
        log::debug!("Telemetry client is NOT available");
    }

    // Get command name for telemetry
    let command_name = match &cli.command {
        Commands::Analyze { .. } => "analyze",
        Commands::Generate { .. } => "generate",
        Commands::Validate { .. } => "validate",
        Commands::Support { .. } => "support",
        Commands::Dependencies { .. } => "dependencies",
        Commands::Vulnerabilities { .. } => "vulnerabilities",
        Commands::Security { .. } => "security",
        Commands::Tools { .. } => "tools",
        Commands::Chat { .. } => "chat",
    };

    log::debug!("Command name: {}", command_name);

    // Execute command
    let result = match cli.command {
        Commands::Analyze {
            path,
            json,
            detailed,
            display,
            only,
            color_scheme,
        } => {
            // Determine analysis mode
            let analysis_mode = if json {
                "json"
            } else if detailed {
                "detailed"
            } else {
                match display {
                    Some(DisplayFormat::Matrix) | None => "matrix",
                    Some(DisplayFormat::Detailed) => "detailed",
                    Some(DisplayFormat::Summary) => "summary",
                }
            };

            // Create telemetry properties
            let mut properties = HashMap::new();
            properties.insert("analysis_mode".to_string(), json!(analysis_mode));

            if let Some(color) = color_scheme {
                let color_str = match color {
                    ColorScheme::Auto => "auto",
                    ColorScheme::Dark => "dark",
                    ColorScheme::Light => "light",
                };
                properties.insert("color_scheme".to_string(), json!(color_str));
            }

            if let Some(only_filters) = &only {
                properties.insert("only_filter".to_string(), json!(only_filters));
            }

            // Track Analyze Folder event with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_analyze_folder(properties);
            }

            match handle_analyze(path, json, detailed, display, only, color_scheme) {
                Ok(_output) => Ok(()), // The output was already printed by display_analysis_with_return
                Err(e) => Err(e),
            }
        }
        Commands::Generate {
            path,
            output,
            dockerfile,
            compose,
            terraform,
            all,
            dry_run,
            force,
        } => {
            // Create telemetry properties
            let mut properties = HashMap::new();

            if dockerfile {
                properties.insert("generate_dockerfile".to_string(), json!(true));
            }

            if compose {
                properties.insert("generate_compose".to_string(), json!(true));
            }

            if terraform {
                properties.insert("generate_terraform".to_string(), json!(true));
            }

            if all {
                properties.insert("generate_all".to_string(), json!(true));
            }

            if dry_run {
                properties.insert("dry_run".to_string(), json!(true));
            }

            if force {
                properties.insert("force_overwrite".to_string(), json!(true));
            }

            if output.is_some() {
                properties.insert("custom_output_dir".to_string(), json!(true));
            }

            // Track Generate command with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_generate(properties);
            }

            handle_generate(
                path, output, dockerfile, compose, terraform, all, dry_run, force,
            )
        }

        Commands::Validate { path, types, fix } => {
            // Create telemetry properties
            let mut properties = HashMap::new();

            if let Some(ref type_list) = types {
                properties.insert("validation_types".to_string(), json!(type_list));
            }

            if fix {
                properties.insert("auto_fix".to_string(), json!(true));
            }

            // Track Validate command with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_validate(properties);
            }

            handle_validate(path, types, fix)
        }
        Commands::Support {
            languages,
            frameworks,
            detailed,
        } => {
            // Create telemetry properties
            let mut properties = HashMap::new();

            if languages {
                properties.insert("show_languages".to_string(), json!(true));
            }

            if frameworks {
                properties.insert("show_frameworks".to_string(), json!(true));
            }

            if detailed {
                properties.insert("detailed".to_string(), json!(true));
            }

            // Track Support command with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_support(properties);
            }

            handle_support(languages, frameworks, detailed)
        }
        Commands::Dependencies {
            path,
            licenses,
            vulnerabilities,
            prod_only,
            dev_only,
            format,
        } => {
            // Create telemetry properties
            let mut properties = HashMap::new();

            if licenses {
                properties.insert("show_licenses".to_string(), json!(true));
            }

            if vulnerabilities {
                properties.insert("check_vulnerabilities".to_string(), json!(true));
            }

            if prod_only {
                properties.insert("prod_only".to_string(), json!(true));
            }

            if dev_only {
                properties.insert("dev_only".to_string(), json!(true));
            }

            // Honor global --json flag for output selection
            let effective_format = if cli.json { OutputFormat::Json } else { format };

            let format_str = match effective_format {
                OutputFormat::Table => "table",
                OutputFormat::Json => "json",
            };
            properties.insert("output_format".to_string(), json!(format_str));

            // Track Dependencies command with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_dependencies(properties);
            }

            handle_dependencies(
                path,
                licenses,
                vulnerabilities,
                prod_only,
                dev_only,
                effective_format,
            )
            .await
            .map(|_| ())
        }
        Commands::Vulnerabilities {
            path,
            severity,
            format,
            output,
        } => {
            // Create telemetry properties
            let mut properties = HashMap::new();

            if let Some(sev) = &severity {
                let severity_str = match sev {
                    SeverityThreshold::Low => "low",
                    SeverityThreshold::Medium => "medium",
                    SeverityThreshold::High => "high",
                    SeverityThreshold::Critical => "critical",
                };
                properties.insert("severity_threshold".to_string(), json!(severity_str));
            }

            // Honor global --json flag for output selection
            let effective_format = if cli.json { OutputFormat::Json } else { format };

            let format_str = match effective_format {
                OutputFormat::Table => "table",
                OutputFormat::Json => "json",
            };
            properties.insert("output_format".to_string(), json!(format_str));

            if output.is_some() {
                properties.insert("export_to_file".to_string(), json!(true));
            }

            // Track Vulnerabilities command with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_vulnerabilities(properties);
            }

            handle_vulnerabilities(path, severity, effective_format, output).await
        }
        Commands::Security {
            path,
            mode,
            include_low,
            no_secrets,
            no_code_patterns,
            no_infrastructure,
            no_compliance,
            frameworks,
            format,
            output,
            fail_on_findings,
        } => {
            // Create telemetry properties
            let mut properties = HashMap::new();

            let mode_str = match mode {
                SecurityScanMode::Lightning => "lightning",
                SecurityScanMode::Fast => "fast",
                SecurityScanMode::Balanced => "balanced",
                SecurityScanMode::Thorough => "thorough",
                SecurityScanMode::Paranoid => "paranoid",
            };
            properties.insert("scan_mode".to_string(), json!(mode_str));

            if include_low {
                properties.insert("include_low_severity".to_string(), json!(true));
            }

            if no_secrets {
                properties.insert("skip_secrets".to_string(), json!(true));
            }

            if no_code_patterns {
                properties.insert("skip_code_patterns".to_string(), json!(true));
            }

            if no_infrastructure {
                properties.insert("skip_infrastructure".to_string(), json!(true));
            }

            if no_compliance {
                properties.insert("skip_compliance".to_string(), json!(true));
            }

            if !frameworks.is_empty() {
                properties.insert("compliance_frameworks".to_string(), json!(frameworks));
            }

            // Honor global --json flag for output selection
            let effective_format = if cli.json { OutputFormat::Json } else { format };

            let format_str = match effective_format {
                OutputFormat::Table => "table",
                OutputFormat::Json => "json",
            };
            properties.insert("output_format".to_string(), json!(format_str));

            if output.is_some() {
                properties.insert("export_to_file".to_string(), json!(true));
            }

            if fail_on_findings {
                properties.insert("fail_on_findings".to_string(), json!(true));
            }

            // Track Security command with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_security(properties);
            }

            handle_security(
                path,
                mode,
                include_low,
                no_secrets,
                no_code_patterns,
                no_infrastructure,
                no_compliance,
                frameworks,
                effective_format,
                output,
                fail_on_findings,
            )
        }
        Commands::Tools { command } => {
            // Create telemetry properties based on the subcommand
            let mut properties = HashMap::new();

            match &command {
                ToolsCommand::Status { format, languages } => {
                    properties.insert("subcommand".to_string(), json!("status"));

                    let format_str = match format {
                        OutputFormat::Table => "table",
                        OutputFormat::Json => "json",
                    };
                    properties.insert("output_format".to_string(), json!(format_str));

                    if let Some(langs) = languages {
                        properties.insert("languages".to_string(), json!(langs));
                    }
                }
                ToolsCommand::Install {
                    languages,
                    include_owasp,
                    dry_run,
                    yes: _,
                } => {
                    properties.insert("subcommand".to_string(), json!("install"));

                    if let Some(langs) = languages {
                        properties.insert("languages".to_string(), json!(langs));
                    }

                    if *include_owasp {
                        properties.insert("include_owasp".to_string(), json!(true));
                    }

                    if *dry_run {
                        properties.insert("dry_run".to_string(), json!(true));
                    }
                }
                ToolsCommand::Verify {
                    languages,
                    detailed,
                } => {
                    properties.insert("subcommand".to_string(), json!("verify"));

                    if let Some(langs) = languages {
                        properties.insert("languages".to_string(), json!(langs));
                    }

                    if *detailed {
                        properties.insert("detailed".to_string(), json!(true));
                    }
                }
                ToolsCommand::Guide {
                    languages,
                    platform,
                } => {
                    properties.insert("subcommand".to_string(), json!("guide"));

                    if let Some(langs) = languages {
                        properties.insert("languages".to_string(), json!(langs));
                    }

                    if let Some(platform) = platform {
                        properties.insert("platform".to_string(), json!(platform));
                    }
                }
            }

            // Track Tools command with properties
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_tools(properties);
            }

            handle_tools(command).await
        }

        Commands::Chat {
            path,
            provider,
            model,
            query,
        } => {
            let mut properties = HashMap::new();

            let provider_str = match provider {
                ChatProvider::Openai => "openai",
                ChatProvider::Anthropic => "anthropic",
                ChatProvider::Bedrock => "bedrock",
                ChatProvider::Ollama => "ollama",
                ChatProvider::Auto => "auto",
            };
            properties.insert("provider".to_string(), json!(provider_str));

            if let Some(m) = &model {
                properties.insert("model".to_string(), json!(m));
            }

            properties.insert(
                "mode".to_string(),
                json!(if query.is_some() {
                    "query"
                } else {
                    "interactive"
                }),
            );

            // Track Chat command
            if let Some(telemetry_client) = telemetry::get_telemetry_client() {
                telemetry_client.track_event("chat", properties.clone());
            }

            syncable_cli::run_command(Commands::Chat {
                path,
                provider,
                model,
                query,
            })
            .await
        }
    };

    // Flush telemetry events before exiting
    if let Some(telemetry_client) = telemetry::get_telemetry_client() {
        telemetry_client.flush().await;
    }

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    Ok(())
}

fn clear_update_cache() {
    let cache_dir_path = cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("syncable-cli");
    let cache_file = cache_dir_path.join("version_cache.json");

    if cache_file.exists() {
        match fs::remove_file(&cache_file) {
            Ok(_) => {
                if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                    eprintln!("🗑️  Removed update cache file: {}", cache_file.display());
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to remove update cache: {}", e);
            }
        }
    } else if std::env::var("SYNC_CTL_DEBUG").is_ok() {
        eprintln!(
            "🗑️  No update cache file found at: {}",
            cache_file.display()
        );
    }
}

async fn check_for_update(suppress_output: bool) {
    // In JSON mode (or when suppressed), avoid any banner or network I/O
    if suppress_output {
        return;
    }
    let cache_dir_path = cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("syncable-cli");
    let cache_file = cache_dir_path.join("version_cache.json");
    let now = SystemTime::now();

    // Smart cache system: only cache when no update is available
    // Check every 2 hours when no update was found, immediately when an update might be available
    let should_check = if let Ok(metadata) = fs::metadata(&cache_file) {
        if let Ok(modified) = metadata.modified() {
            let cache_duration = now.duration_since(modified).unwrap_or(Duration::ZERO);

            // Read cached data to determine cache strategy
            if let Ok(cache_content) = fs::read_to_string(&cache_file) {
                if let Ok(cache_data) = serde_json::from_str::<serde_json::Value>(&cache_content) {
                    let cached_latest = cache_data["latest_version"].as_str().unwrap_or("");
                    let current = env!("CARGO_PKG_VERSION");

                    // If cached version is newer than current, check immediately
                    if !cached_latest.is_empty() && is_version_newer(current, cached_latest) {
                        if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                            eprintln!("🔍 Update available in cache, showing immediately");
                        }
                        show_update_notification(current, cached_latest);
                        return;
                    }

                    // If no update in cache, check every 2 hours
                    cache_duration >= Duration::from_secs(60 * 60 * 2)
                } else {
                    true // Invalid cache, check now
                }
            } else {
                true // Can't read cache, check now
            }
        } else {
            true // Can't get modified time, check now
        }
    } else {
        true // No cache file, check now
    };

    if !should_check {
        if std::env::var("SYNC_CTL_DEBUG").is_ok() {
            eprintln!("🔍 Update check skipped - checked recently and no update available");
        }
        return;
    }

    // Debug logging
    if std::env::var("SYNC_CTL_DEBUG").is_ok() {
        eprintln!("🔍 Checking for updates...");
    }

    // Query GitHub releases API
    let client = reqwest::Client::builder()
        .user_agent(format!("syncable-cli/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(5))
        .build();

    match client {
        Ok(client) => {
            let result = client
                .get("https://api.github.com/repos/syncable-dev/syncable-cli/releases/latest")
                .send()
                .await;

            match result {
                Ok(response) => {
                    if !response.status().is_success() {
                        if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                            eprintln!("⚠️  GitHub API returned status: {}", response.status());
                        }
                        return;
                    }

                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let latest = json["tag_name"]
                                .as_str()
                                .unwrap_or("")
                                .trim_start_matches('v'); // Remove 'v' prefix if present
                            let current = env!("CARGO_PKG_VERSION");

                            if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                                eprintln!(
                                    "📦 Current version: {}, Latest version: {}",
                                    current, latest
                                );
                            }

                            // Update cache with latest version info
                            let cache_data = serde_json::json!({
                                "latest_version": latest,
                                "current_version": current,
                                "checked_at": now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                                "update_available": is_version_newer(current, latest)
                            });

                            let _ = fs::create_dir_all(&cache_dir_path);
                            let _ = fs::write(
                                &cache_file,
                                serde_json::to_string_pretty(&cache_data).unwrap_or_default(),
                            );

                            // Show update notification if newer version is available
                            if !latest.is_empty()
                                && latest != current
                                && is_version_newer(current, latest)
                                && !suppress_output {
                                    show_update_notification(current, latest);
                                }
                        }
                        Err(e) => {
                            if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                                eprintln!("⚠️  Failed to parse GitHub API response: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                        eprintln!("⚠️  Failed to check for updates: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            if std::env::var("SYNC_CTL_DEBUG").is_ok() {
                eprintln!("⚠️  Failed to create HTTP client: {}", e);
            }
        }
    }
}

fn show_update_notification(current: &str, latest: &str) {
    use colored::*;

    let mut box_drawer = BoxDrawer::new(&"UPDATE AVAILABLE".bright_red().bold().to_string());

    // Version info line with prominent colors
    let version_info = format!(
        "New version: {} | Current: {}",
        latest.bright_green().bold(),
        current.bright_red()
    );
    box_drawer.add_value_only(&version_info);

    // Empty line for spacing
    box_drawer.add_value_only("");

    // Instructions header with emphasis
    box_drawer.add_value_only(
        &"To update, run one of these commands:"
            .bright_cyan()
            .bold()
            .to_string(),
    );
    box_drawer.add_value_only("");

    // Recommended method - highlighted as primary option
    box_drawer.add_line(
        &"RECOMMENDED".bright_green().bold().to_string(),
        &"(via Cargo)".green().to_string(),
        false,
    );
    let cargo_cmd = "cargo install syncable-cli"
        .bright_white()
        .on_blue()
        .bold()
        .to_string();
    box_drawer.add_value_only(&format!("  {}", cargo_cmd));
    box_drawer.add_value_only("");

    // Alternative method - neutral coloring
    box_drawer.add_line(
        &"ALTERNATIVE".yellow().bold().to_string(),
        &"(direct download)".yellow().to_string(),
        false,
    );
    let github_url = format!(
        "  Visit: {}",
        format!("github.com/syncable-dev/syncable-cli/releases/v{}", latest)
            .bright_blue()
            .underline()
    );
    box_drawer.add_value_only(&github_url);
    box_drawer.add_value_only("");

    // Install script method - secondary option
    box_drawer.add_line(
        &"SCRIPT".magenta().bold().to_string(),
        &"(automated installer)".magenta().to_string(),
        false,
    );
    let script_cmd = "curl -sSL install.syncable.dev | sh"
        .bright_white()
        .on_magenta()
        .bold()
        .to_string();
    box_drawer.add_value_only(&format!("  {}", script_cmd));

    // Add a helpful note
    box_drawer.add_value_only("");
    box_drawer.add_value_only(
        &"Tip: The Cargo method is fastest for existing Rust users"
            .dimmed()
            .italic()
            .to_string(),
    );

    println!("\n{}", box_drawer.draw());
}

// Helper function to compare semantic versions
fn is_version_newer(current: &str, latest: &str) -> bool {
    let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let latest_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..3 {
        let current_part = current_parts.get(i).unwrap_or(&0);
        let latest_part = latest_parts.get(i).unwrap_or(&0);

        if latest_part > current_part {
            return true;
        } else if latest_part < current_part {
            return false;
        }
    }

    false
}

pub fn handle_analyze(
    path: std::path::PathBuf,
    json: bool,
    detailed: bool,
    display: Option<DisplayFormat>,
    only: Option<Vec<String>>,
    color_scheme: Option<ColorScheme>,
) -> syncable_cli::Result<()> {
    // Call the handler from the handlers module which returns a string
    syncable_cli::handlers::analyze::handle_analyze(
        path,
        json,
        detailed,
        display,
        only,
        color_scheme,
    )?;

    Ok(())
}

fn handle_generate(
    path: std::path::PathBuf,
    _output: Option<std::path::PathBuf>,
    dockerfile: bool,
    compose: bool,
    terraform: bool,
    all: bool,
    dry_run: bool,
    _force: bool,
) -> syncable_cli::Result<()> {
    println!("🔍 Analyzing project for generation: {}", path.display());

    let monorepo_analysis = analyze_monorepo(&path)?;

    println!("✅ Analysis complete. Generating IaC files...");

    if monorepo_analysis.is_monorepo {
        println!(
            "📦 Detected monorepo with {} projects",
            monorepo_analysis.projects.len()
        );
        println!(
            "🚧 Monorepo IaC generation is coming soon! For now, generating for the overall structure."
        );
        println!(
            "💡 Tip: You can run generate commands on individual project directories for now."
        );
    }

    // For now, use the first/main project for generation
    // TODO: Implement proper monorepo IaC generation
    let main_project = &monorepo_analysis.projects[0];

    let generate_all = all || (!dockerfile && !compose && !terraform);

    if generate_all || dockerfile {
        println!("\n🐳 Generating Dockerfile...");
        let dockerfile_content = generator::generate_dockerfile(&main_project.analysis)?;

        if dry_run {
            println!("--- Dockerfile (dry run) ---");
            println!("{}", dockerfile_content);
        } else {
            std::fs::write("Dockerfile", dockerfile_content)?;
            println!("✅ Dockerfile generated successfully!");
        }
    }

    if generate_all || compose {
        println!("\n🐙 Generating Docker Compose file...");
        let compose_content = generator::generate_compose(&main_project.analysis)?;

        if dry_run {
            println!("--- docker-compose.yml (dry run) ---");
            println!("{}", compose_content);
        } else {
            std::fs::write("docker-compose.yml", compose_content)?;
            println!("✅ Docker Compose file generated successfully!");
        }
    }

    if generate_all || terraform {
        println!("\n🏗️  Generating Terraform configuration...");
        let terraform_content = generator::generate_terraform(&main_project.analysis)?;

        if dry_run {
            println!("--- main.tf (dry run) ---");
            println!("{}", terraform_content);
        } else {
            std::fs::write("main.tf", terraform_content)?;
            println!("✅ Terraform configuration generated successfully!");
        }
    }

    if !dry_run {
        println!("\n🎉 Generation complete! IaC files have been created in the current directory.");

        if monorepo_analysis.is_monorepo {
            println!("🔧 Note: Generated files are based on the main project structure.");
            println!("   Advanced monorepo support with per-project generation is coming soon!");
        }
    }

    Ok(())
}

fn handle_validate(
    _path: std::path::PathBuf,
    _types: Option<Vec<String>>,
    _fix: bool,
) -> syncable_cli::Result<()> {
    println!("🔍 Validating IaC files...");
    println!("⚠️  Validation feature is not yet implemented.");
    Ok(())
}

fn handle_support(languages: bool, frameworks: bool, _detailed: bool) -> syncable_cli::Result<()> {
    if languages || !frameworks {
        println!("🌐 Supported Languages:");
        println!("├── Rust");
        println!("├── JavaScript/TypeScript");
        println!("├── Python");
        println!("├── Go");
        println!("├── Java");
        println!("└── (More coming soon...)");
    }

    if frameworks || !languages {
        println!("\n🚀 Supported Frameworks:");
        println!("├── Web: Express.js, Next.js, React, Vue.js, Actix Web");
        println!("├── Database: PostgreSQL, MySQL, MongoDB, Redis");
        println!("├── Build Tools: npm, yarn, cargo, maven, gradle");
        println!("└── (More coming soon...)");
    }

    Ok(())
}

pub async fn handle_dependencies(
    path: std::path::PathBuf,
    licenses: bool,
    vulnerabilities: bool,
    _prod_only: bool,
    _dev_only: bool,
    format: OutputFormat,
) -> syncable_cli::Result<String> {
    syncable_cli::handlers::dependencies::handle_dependencies(
        path,
        licenses,
        vulnerabilities,
        _prod_only,
        _dev_only,
        format,
    )
    .await
}

pub async fn handle_vulnerabilities(
    path: std::path::PathBuf,
    severity: Option<SeverityThreshold>,
    format: OutputFormat,
    output: Option<std::path::PathBuf>,
) -> syncable_cli::Result<()> {
    let project_path = path.canonicalize().unwrap_or_else(|_| path.clone());

    println!(
        "🔍 Scanning for vulnerabilities in: {}",
        project_path.display()
    );

    // Parse dependencies
    let dependencies = analyzer::dependency_parser::DependencyParser::new()
        .parse_all_dependencies(&project_path)?;

    if dependencies.is_empty() {
        println!("No dependencies found to check.");
        return Ok(());
    }

    // Check vulnerabilities
    let checker = analyzer::vulnerability::VulnerabilityChecker::new();
    let report = checker
        .check_all_dependencies(&dependencies, &project_path)
        .await
        .map_err(|e| {
            syncable_cli::error::IaCGeneratorError::Analysis(
                syncable_cli::error::AnalysisError::DependencyParsing {
                    file: "vulnerability check".to_string(),
                    reason: e.to_string(),
                },
            )
        })?;

    // Filter by severity if requested
    let filtered_report = if let Some(threshold) = severity {
        let min_severity = match threshold {
            SeverityThreshold::Low => VulnerabilitySeverity::Low,
            SeverityThreshold::Medium => VulnerabilitySeverity::Medium,
            SeverityThreshold::High => VulnerabilitySeverity::High,
            SeverityThreshold::Critical => VulnerabilitySeverity::Critical,
        };

        let filtered_deps: Vec<_> = report
            .vulnerable_dependencies
            .into_iter()
            .filter_map(|mut dep| {
                dep.vulnerabilities.retain(|v| v.severity >= min_severity);
                if dep.vulnerabilities.is_empty() {
                    None
                } else {
                    Some(dep)
                }
            })
            .collect();

        use analyzer::vulnerability::VulnerabilityReport;
        let mut filtered = VulnerabilityReport {
            checked_at: report.checked_at,
            total_vulnerabilities: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            vulnerable_dependencies: filtered_deps,
        };

        // Recalculate counts
        for dep in &filtered.vulnerable_dependencies {
            for vuln in &dep.vulnerabilities {
                filtered.total_vulnerabilities += 1;
                match vuln.severity {
                    VulnerabilitySeverity::Critical => filtered.critical_count += 1,
                    VulnerabilitySeverity::High => filtered.high_count += 1,
                    VulnerabilitySeverity::Medium => filtered.medium_count += 1,
                    VulnerabilitySeverity::Low => filtered.low_count += 1,
                    VulnerabilitySeverity::Info => {}
                }
            }
        }

        filtered
    } else {
        report
    };

    // Format output
    let output_string = match format {
        OutputFormat::Table => {
            // Color formatting for output

            let mut output = String::new();

            output.push_str("\n🛡️  Vulnerability Scan Report\n");
            output.push_str(&format!("{}\n", "=".repeat(80).bright_blue()));
            output.push_str(&format!(
                "Scanned at: {}\n",
                filtered_report.checked_at.format("%Y-%m-%d %H:%M:%S UTC")
            ));
            output.push_str(&format!("Path: {}\n", project_path.display()));

            if let Some(threshold) = severity {
                output.push_str(&format!("Severity filter: >= {:?}\n", threshold));
            }

            output.push_str("\nSummary:\n");
            output.push_str(&format!(
                "Total vulnerabilities: {}\n",
                filtered_report.total_vulnerabilities
            ));

            if filtered_report.total_vulnerabilities > 0 {
                output.push_str("\nBy Severity:\n");
                if filtered_report.critical_count > 0 {
                    output.push_str(&format!(
                        "  🔴 CRITICAL: {}\n",
                        filtered_report.critical_count
                    ));
                }
                if filtered_report.high_count > 0 {
                    output.push_str(&format!("  🔴 HIGH: {}\n", filtered_report.high_count));
                }
                if filtered_report.medium_count > 0 {
                    output.push_str(&format!("  🟡 MEDIUM: {}\n", filtered_report.medium_count));
                }
                if filtered_report.low_count > 0 {
                    output.push_str(&format!("  🔵 LOW: {}\n", filtered_report.low_count));
                }

                output.push_str(&format!("\n{}\n", "-".repeat(80)));
                output.push_str("Vulnerable Dependencies:\n\n");

                for vuln_dep in &filtered_report.vulnerable_dependencies {
                    output.push_str(&format!(
                        "📦 {} v{} ({})\n",
                        vuln_dep.name,
                        vuln_dep.version,
                        vuln_dep.language.as_str()
                    ));

                    for vuln in &vuln_dep.vulnerabilities {
                        let severity_str = match vuln.severity {
                            VulnerabilitySeverity::Critical => "CRITICAL",
                            VulnerabilitySeverity::High => "HIGH",
                            VulnerabilitySeverity::Medium => "MEDIUM",
                            VulnerabilitySeverity::Low => "LOW",
                            VulnerabilitySeverity::Info => "INFO",
                        };

                        output.push_str(&format!("\n  ⚠️  {} [{}]\n", vuln.id, severity_str));
                        output.push_str(&format!("     {}\n", vuln.title));

                        if !vuln.description.is_empty() && vuln.description != vuln.title {
                            // Wrap description
                            let wrapped = textwrap::fill(&vuln.description, 70);
                            for line in wrapped.lines() {
                                output.push_str(&format!("     {}\n", line));
                            }
                        }

                        if let Some(ref cve) = vuln.cve {
                            output.push_str(&format!("     CVE: {}\n", cve));
                        }

                        if let Some(ref ghsa) = vuln.ghsa {
                            output.push_str(&format!("     GHSA: {}\n", ghsa));
                        }

                        output.push_str(&format!("     Affected: {}\n", vuln.affected_versions));

                        if let Some(ref patched) = vuln.patched_versions {
                            output.push_str(&format!("     ✅ Fix: Upgrade to {}\n", patched));
                        }
                    }
                    output.push('\n');
                }
            } else {
                output.push_str("\n✅ No vulnerabilities found!\n");
            }

            output
        }
        OutputFormat::Json => serde_json::to_string_pretty(&filtered_report)?,
    };

    // Output results
    if let Some(output_path) = output {
        std::fs::write(&output_path, output_string)?;
        println!("Report saved to: {}", output_path.display());
    } else {
        println!("{}", output_string);
    }

    // Exit with non-zero code if critical/high vulnerabilities found
    if filtered_report.critical_count > 0 || filtered_report.high_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

pub fn handle_security(
    path: std::path::PathBuf,
    mode: SecurityScanMode,
    include_low: bool,
    no_secrets: bool,
    no_code_patterns: bool,
    no_infrastructure: bool,
    no_compliance: bool,
    frameworks: Vec<String>,
    format: OutputFormat,
    output: Option<std::path::PathBuf>,
    fail_on_findings: bool,
) -> syncable_cli::Result<()> {
    // Call the handler from the handlers module which both prints and returns a string
    let _result = syncable_cli::handlers::security::handle_security(
        path,
        mode,
        include_low,
        no_secrets,
        no_code_patterns,
        no_infrastructure,
        no_compliance,
        frameworks,
        format,
        output,
        fail_on_findings,
    )?;

    // The handler already prints everything, so we just return Ok
    // The returned string is available if needed by other consumers (like AI agents)
    Ok(())
}

async fn handle_tools(command: ToolsCommand) -> syncable_cli::Result<()> {
    syncable_cli::handlers::tools::handle_tools(command).await
}
