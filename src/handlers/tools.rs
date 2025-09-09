use crate::{
    analyzer::{tool_management::ToolInstaller, dependency_parser::Language},
    cli::{ToolsCommand, OutputFormat},
};
use std::collections::HashMap;
use termcolor::{ColorChoice, StandardStream, WriteColor, ColorSpec, Color};

pub async fn handle_tools(command: ToolsCommand) -> crate::Result<()> {
    match command {
        ToolsCommand::Status { format, languages } => handle_tools_status(format, languages),
        ToolsCommand::Install { languages, include_owasp, dry_run, yes } => {
            handle_tools_install(languages, include_owasp, dry_run, yes)
        }
        ToolsCommand::Verify { languages, detailed } => handle_tools_verify(languages, detailed),
        ToolsCommand::Guide { languages, platform } => handle_tools_guide(languages, platform),
    }
}

fn handle_tools_status(format: OutputFormat, languages: Option<Vec<String>>) -> crate::Result<()> {
    let mut installer = ToolInstaller::new();
    
    // Determine which languages to check
    let langs_to_check = get_languages_to_check(languages);
    
    println!("🔧 Checking vulnerability scanning tools status...\n");
    
    match format {
        OutputFormat::Table => display_status_table(&mut installer, &langs_to_check)?,
        OutputFormat::Json => display_status_json(&mut installer, &langs_to_check),
    }
    
    Ok(())
}

fn handle_tools_install(
    languages: Option<Vec<String>>,
    include_owasp: bool,
    dry_run: bool,
    yes: bool,
) -> crate::Result<()> {
    let mut installer = ToolInstaller::new();
    
    // Determine which languages to install tools for
    let langs_to_install = get_languages_to_install(languages);
    
    if dry_run {
        return handle_dry_run(&mut installer, &langs_to_install, include_owasp);
    }
    
    if !yes && !confirm_installation()? {
        println!("Installation cancelled.");
        return Ok(());
    }
    
    println!("🛠️  Installing vulnerability scanning tools...");
    
    match installer.ensure_tools_for_languages(&langs_to_install) {
        Ok(()) => {
            println!("✅ Tool installation completed!");
            installer.print_tool_status(&langs_to_install);
            print_setup_instructions();
        }
        Err(e) => {
            eprintln!("❌ Tool installation failed: {}", e);
            eprintln!("\n🔧 Manual installation may be required for some tools.");
            eprintln!("   Run 'sync-ctl tools guide' for manual installation instructions.");
            return Err(e);
        }
    }
    
    Ok(())
}

fn handle_tools_verify(languages: Option<Vec<String>>, detailed: bool) -> crate::Result<()> {
    let mut installer = ToolInstaller::new();
    
    // Determine which languages to verify
    let langs_to_verify = get_languages_to_verify(languages);
    
    println!("🔍 Verifying vulnerability scanning tools...\n");
    
    let mut all_working = true;
    
    for language in &langs_to_verify {
        let (tool_name, is_working) = get_tool_for_language(&mut installer, language);
        
        print!("  {} {:?}: {}", 
               if is_working { "✅" } else { "❌" }, 
               language,
               tool_name);
        
        if is_working {
            println!(" - working correctly");
            
            if detailed {
                print_version_info(tool_name);
            }
        } else {
            println!(" - not working or missing");
            all_working = false;
        }
    }
    
    if all_working {
        println!("\n✅ All tools are working correctly!");
    } else {
        println!("\n❌ Some tools are missing or not working.");
        println!("   Run 'sync-ctl tools install' to install missing tools.");
    }
    
    Ok(())
}

fn handle_tools_guide(languages: Option<Vec<String>>, platform: Option<String>) -> crate::Result<()> {
    let target_platform = platform.unwrap_or_else(|| {
        match std::env::consts::OS {
            "macos" => "macOS".to_string(),
            "linux" => "Linux".to_string(),
            "windows" => "Windows".to_string(),
            other => other.to_string(),
        }
    });
    
    println!("📚 Vulnerability Scanning Tools Installation Guide");
    println!("Platform: {}", target_platform);
    println!("{}", "=".repeat(60));
    
    let langs_to_show = get_languages_to_show(languages);
    
    for language in &langs_to_show {
        print_language_guide(language, &target_platform);
    }
    
    print_universal_scanners_info();
    print_general_tips();
    
    Ok(())
}

// Helper functions

fn get_languages_to_check(languages: Option<Vec<String>>) -> Vec<Language> {
    if let Some(lang_names) = languages {
        lang_names.iter()
            .filter_map(|name| Language::from_string(name))
            .collect()
    } else {
        vec![
            Language::Rust,
            Language::JavaScript,
            Language::TypeScript,
            Language::Python,
            Language::Go,
            Language::Java,
            Language::Kotlin,
        ]
    }
}

fn get_languages_to_install(languages: Option<Vec<String>>) -> Vec<Language> {
    if let Some(lang_names) = languages {
        lang_names.iter()
            .filter_map(|name| Language::from_string(name))
            .collect()
    } else {
        vec![
            Language::Rust,
            Language::JavaScript,
            Language::TypeScript,
            Language::Python,
            Language::Go,
            Language::Java,
        ]
    }
}

fn get_languages_to_verify(languages: Option<Vec<String>>) -> Vec<Language> {
    if let Some(lang_names) = languages {
        lang_names.iter()
            .filter_map(|name| Language::from_string(name))
            .collect()
    } else {
        vec![
            Language::Rust,
            Language::JavaScript,
            Language::TypeScript,
            Language::Python,
            Language::Go,
            Language::Java,
        ]
    }
}

fn get_languages_to_show(languages: Option<Vec<String>>) -> Vec<Language> {
    if let Some(lang_names) = languages {
        lang_names.iter()
            .filter_map(|name| Language::from_string(name))
            .collect()
    } else {
        vec![
            Language::Rust,
            Language::JavaScript,
            Language::TypeScript,
            Language::Python,
            Language::Go,
            Language::Java,
        ]
    }
}

fn get_tool_for_language<'a>(installer: &mut ToolInstaller, language: &Language) -> (&'a str, bool) {
    match language {
        Language::Rust => ("cargo-audit", installer.test_tool_availability("cargo-audit")),
        Language::JavaScript | Language::TypeScript => {
            // Check all JavaScript package managers, prioritize bun
            if installer.test_tool_availability("bun") {
                ("bun", true)
            } else if installer.test_tool_availability("npm") {
                ("npm", true)
            } else if installer.test_tool_availability("yarn") {
                ("yarn", true)
            } else if installer.test_tool_availability("pnpm") {
                ("pnpm", true)
            } else {
                ("npm", false)
            }
        },
        Language::Python => ("pip-audit", installer.test_tool_availability("pip-audit")),
        Language::Go => ("govulncheck", installer.test_tool_availability("govulncheck")),
        Language::Java | Language::Kotlin => ("grype", installer.test_tool_availability("grype")),
        _ => ("unknown", false),
    }
}

fn display_status_table(installer: &mut ToolInstaller, langs_to_check: &[Language]) -> crate::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    
    println!("📋 Vulnerability Scanning Tools Status");
    println!("{}", "=".repeat(50));
    
    // Use the enhanced tool status display from ToolInstaller
    installer.print_tool_status(langs_to_check);
    
    // Also check universal tools
    println!("🔍 Universal Scanners:");
    let grype_available = installer.test_tool_availability("grype");
    print!("  {} Grype: ", if grype_available { "✅" } else { "❌" });
    if grype_available {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        println!("installed");
    } else {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        println!("missing - Install with: brew install grype or download from GitHub");
    }
    stdout.reset()?;
    
    Ok(())
}

fn display_status_json(installer: &mut ToolInstaller, langs_to_check: &[Language]) {
    let mut status = HashMap::new();
    
    for language in langs_to_check {
        let (tool_name, is_available) = get_tool_for_language(installer, language);
        
        if tool_name == "unknown" {
            continue;
        }
        
        status.insert(format!("{:?}", language), serde_json::json!({
            "tool": tool_name,
            "available": is_available
        }));
    }
    
    println!("{}", serde_json::to_string_pretty(&status).unwrap());
}

fn handle_dry_run(installer: &mut ToolInstaller, langs_to_install: &[Language], include_owasp: bool) -> crate::Result<()> {
    println!("🔍 Dry run: Tools that would be installed:");
    println!("{}", "=".repeat(50));
    
    for language in langs_to_install {
        let (tool_name, is_available) = get_tool_for_language(installer, language);
        
        if tool_name == "unknown" {
            continue;
        }
        
        if !is_available {
            println!("  📦 Would install {} for {:?}", tool_name, language);
        } else {
            println!("  ✅ {} already installed for {:?}", tool_name, language);
        }
    }
    
    if include_owasp && !installer.test_tool_availability("dependency-check") {
        println!("  📦 Would install OWASP Dependency Check (large download)");
    }
    
    Ok(())
}

fn confirm_installation() -> crate::Result<bool> {
    use std::io::{self, Write};
    print!("🔧 Install missing vulnerability scanning tools? [y/N]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_lowercase().starts_with('y'))
}

fn print_setup_instructions() {
    println!("\n💡 Setup Instructions:");
    println!("  • Add ~/.local/bin to your PATH for manually installed tools");
    println!("  • Add ~/go/bin to your PATH for Go tools");
    println!("  • Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):");
    println!("    export PATH=\"$HOME/.local/bin:$HOME/go/bin:$PATH\"");
}

fn print_version_info(tool_name: &str) {
    use std::process::Command;
    let version_result = match tool_name {
        "cargo-audit" => Command::new("cargo").args(&["audit", "--version"]).output(),
        "npm" => Command::new("npm").arg("--version").output(),
        "bun" => Command::new("bun").arg("--version").output(),
        "yarn" => Command::new("yarn").arg("--version").output(),
        "pnpm" => Command::new("pnpm").arg("--version").output(),
        "pip-audit" => Command::new("pip-audit").arg("--version").output(),
        "govulncheck" => Command::new("govulncheck").arg("-version").output(),
        "grype" => Command::new("grype").arg("version").output(),
        _ => return,
    };
    
    if let Ok(output) = version_result {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("    Version: {}", version.trim());
        }
    }
}

fn print_language_guide(language: &Language, target_platform: &str) {
    match language {
        Language::Rust => {
            println!("\n🦀 Rust - cargo-audit");
            println!("  Install: cargo install cargo-audit");
            println!("  Usage: cargo audit");
        }
        Language::JavaScript | Language::TypeScript => {
            println!("\n🌐 JavaScript/TypeScript - Multiple package managers");
            println!("  Bun (recommended for speed):");
            println!("    Install: curl -fsSL https://bun.sh/install | bash");
            match target_platform {
                "Windows" => println!("    Windows: irm bun.sh/install.ps1 | iex"),
                _ => {}
            }
            println!("    Usage: bun audit");
            println!("  npm (traditional):");
            println!("    Install: Download Node.js from https://nodejs.org/");
            match target_platform {
                "macOS" => println!("    Package manager: brew install node"),
                "Linux" => println!("    Package manager: sudo apt install nodejs npm (Ubuntu/Debian)"),
                _ => {}
            }
            println!("    Usage: npm audit");
            println!("  yarn:");
            println!("    Install: npm install -g yarn");
            println!("    Usage: yarn audit");
            println!("  pnpm:");
            println!("    Install: npm install -g pnpm");
            println!("    Usage: pnpm audit");
        }
        Language::Python => {
            println!("\n🐍 Python - pip-audit");
            println!("  Install: pipx install pip-audit (recommended)");
            println!("  Alternative: pip3 install --user pip-audit");
            println!("  Also available: safety (pip install safety)");
            println!("  Usage: pip-audit");
        }
        Language::Go => {
            println!("\n🐹 Go - govulncheck");
            println!("  Install: go install golang.org/x/vuln/cmd/govulncheck@latest");
            println!("  Note: Make sure ~/go/bin is in your PATH");
            println!("  Usage: govulncheck ./...");
        }
        Language::Java => {
            println!("\n☕ Java - Multiple options");
            println!("  Grype (recommended):");
            match target_platform {
                "macOS" => println!("    Install: brew install anchore/grype/grype"),
                "Linux" => println!("    Install: Download from https://github.com/anchore/grype/releases"),
                _ => println!("    Install: Download from https://github.com/anchore/grype/releases"),
            }
            println!("    Usage: grype .");
            println!("  OWASP Dependency Check:");
            match target_platform {
                "macOS" => println!("    Install: brew install dependency-check"),
                _ => println!("    Install: Download from https://github.com/jeremylong/DependencyCheck/releases"),
            }
            println!("    Usage: dependency-check --project myproject --scan .");
        }
        _ => {}
    }
}

fn print_universal_scanners_info() {
    println!("\n🔍 Universal Scanners:");
    println!("  Grype: Works with multiple ecosystems");
    println!("  Trivy: Container and filesystem scanning");
    println!("  Snyk: Commercial solution with free tier");
}

fn print_general_tips() {
    println!("\n💡 Tips:");
    println!("  • Run 'sync-ctl tools status' to check current installation");
    println!("  • Run 'sync-ctl tools install' for automatic installation");
    println!("  • Add tool directories to your PATH for easier access");
} 