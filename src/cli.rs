use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sync-ctl")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Generate Infrastructure as Code from your codebase")]
#[command(long_about = "A powerful CLI tool that analyzes your codebase and automatically generates optimized Infrastructure as Code configurations including Dockerfiles, Docker Compose files, and Terraform configurations.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to configuration file
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging (-v for info, -vv for debug, -vvv for trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output in JSON format where applicable
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze a project and display detected components
    Analyze {
        /// Path to the project directory to analyze
        #[arg(value_name = "PROJECT_PATH")]
        path: PathBuf,

        /// Output analysis results in JSON format
        #[arg(short, long)]
        json: bool,

        /// Show detailed analysis information
        #[arg(short, long)]
        detailed: bool,

        /// Only analyze specific aspects (languages, frameworks, dependencies)
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,
    },

    /// Generate IaC files for a project
    Generate {
        /// Path to the project directory to analyze
        #[arg(value_name = "PROJECT_PATH")]
        path: PathBuf,

        /// Output directory for generated files
        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output: Option<PathBuf>,

        /// Generate Dockerfile
        #[arg(long)]
        dockerfile: bool,

        /// Generate Docker Compose file
        #[arg(long)]
        compose: bool,

        /// Generate Terraform configuration
        #[arg(long)]
        terraform: bool,

        /// Generate all supported IaC files
        #[arg(long, conflicts_with_all = ["dockerfile", "compose", "terraform"])]
        all: bool,

        /// Perform a dry run without creating files
        #[arg(long)]
        dry_run: bool,

        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },

    /// Validate existing IaC files against best practices
    Validate {
        /// Path to the directory containing IaC files
        #[arg(value_name = "PATH")]
        path: PathBuf,

        /// Types of files to validate
        #[arg(long, value_delimiter = ',')]
        types: Option<Vec<String>>,

        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,
    },

    /// Show supported languages and frameworks
    Support {
        /// Show only languages
        #[arg(long)]
        languages: bool,

        /// Show only frameworks
        #[arg(long)]
        frameworks: bool,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Analyze project dependencies in detail
    Dependencies {
        /// Path to the project directory to analyze
        #[arg(value_name = "PROJECT_PATH")]
        path: PathBuf,

        /// Show license information for dependencies
        #[arg(long)]
        licenses: bool,

        /// Check for known vulnerabilities
        #[arg(long)]
        vulnerabilities: bool,

        /// Show only production dependencies
        #[arg(long, conflicts_with = "dev_only")]
        prod_only: bool,

        /// Show only development dependencies
        #[arg(long, conflicts_with = "prod_only")]
        dev_only: bool,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Check dependencies for known vulnerabilities
    Vulnerabilities {
        /// Check vulnerabilities in a specific path
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Show only vulnerabilities with severity >= threshold
        #[arg(long, value_enum)]
        severity: Option<SeverityThreshold>,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,

        /// Export report to file
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SeverityThreshold {
    Low,
    Medium,
    High,
    Critical,
}

impl Cli {
    /// Initialize logging based on verbosity level
    pub fn init_logging(&self) {
        if self.quiet {
            return;
        }

        let level = match self.verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };

        env_logger::Builder::from_default_env()
            .filter_level(level)
            .init();
    }
} 