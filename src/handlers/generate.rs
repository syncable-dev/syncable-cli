use crate::{
    analyzer::analyze_monorepo,
    generator,
};

pub fn handle_generate(
    path: std::path::PathBuf,
    _output: Option<std::path::PathBuf>,
    dockerfile: bool,
    compose: bool,
    terraform: bool,
    all: bool,
    dry_run: bool,
    _force: bool,
) -> crate::Result<()> {
    println!("🔍 Analyzing project for generation: {}", path.display());
    
    let monorepo_analysis = analyze_monorepo(&path)?;
    
    println!("✅ Analysis complete. Generating IaC files...");
    
    if monorepo_analysis.is_monorepo {
        println!("📦 Detected monorepo with {} projects", monorepo_analysis.projects.len());
        println!("🚧 Monorepo IaC generation is coming soon! For now, generating for the overall structure.");
        println!("💡 Tip: You can run generate commands on individual project directories for now.");
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

pub fn handle_validate(
    _path: std::path::PathBuf,
    _types: Option<Vec<String>>,
    _fix: bool,
) -> crate::Result<()> {
    println!("🔍 Validating IaC files...");
    println!("⚠️  Validation feature is not yet implemented.");
    Ok(())
} 