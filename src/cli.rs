use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sync-ctl")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "DevOps CLI toolbox for AI coding agents and developers")]
#[command(
    long_about = "Analyze tech stacks, scan for security issues and CVEs, validate IaC files, optimize Kubernetes resources, and deploy to cloud providers. Works standalone or through AI coding agent skills (Claude Code, Codex, Gemini CLI, Cursor, Windsurf)."
)]
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

    /// Clear the update check cache and force a new check
    #[arg(long, global = true)]
    pub clear_update_cache: bool,

    /// Disable telemetry data collection
    #[arg(long, global = true)]
    pub disable_telemetry: bool,
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

        /// Show detailed analysis information (legacy format)
        #[arg(short, long, conflicts_with = "display")]
        detailed: bool,

        /// Display format for analysis results
        #[arg(long, value_enum, default_value = "matrix")]
        display: Option<DisplayFormat>,

        /// Only analyze specific aspects (languages, frameworks, dependencies)
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,

        /// Color scheme for terminal output (auto-detect, dark, light)
        #[arg(long, value_enum, default_value = "auto")]
        color_scheme: Option<ColorScheme>,

        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
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

        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
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

        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
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

        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
    },

    /// Perform comprehensive security analysis
    Security {
        /// Path to the project directory to analyze
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: PathBuf,

        /// Security scan mode (lightning, fast, balanced, thorough, paranoid)
        #[arg(long, value_enum, default_value = "thorough")]
        mode: SecurityScanMode,

        /// Include low severity findings
        #[arg(long)]
        include_low: bool,

        /// Skip secrets detection
        #[arg(long)]
        no_secrets: bool,

        /// Skip code pattern analysis
        #[arg(long)]
        no_code_patterns: bool,

        /// Skip infrastructure analysis (not implemented yet)
        #[arg(long, hide = true)]
        no_infrastructure: bool,

        /// Skip compliance checks (not implemented yet)
        #[arg(long, hide = true)]
        no_compliance: bool,

        /// Compliance frameworks to check (not implemented yet)
        #[arg(long, value_delimiter = ',', hide = true)]
        frameworks: Vec<String>,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,

        /// Export report to file
        #[arg(long)]
        output: Option<PathBuf>,

        /// Exit with error code on security findings
        #[arg(long)]
        fail_on_findings: bool,

        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
    },

    /// Manage vulnerability scanning tools
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },

    /// Analyze Kubernetes manifests for resource optimization opportunities
    Optimize {
        /// Path to Kubernetes manifests (file or directory)
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,

        /// Connect to a live Kubernetes cluster for metrics-based recommendations
        /// Uses current kubeconfig context, or specify a context name
        #[arg(long, short = 'k', value_name = "CONTEXT", default_missing_value = "current", num_args = 0..=1)]
        cluster: Option<String>,

        /// Prometheus URL for historical metrics (e.g., http://localhost:9090)
        #[arg(long, value_name = "URL")]
        prometheus: Option<String>,

        /// Target namespace(s) for cluster analysis (comma-separated, or * for all)
        #[arg(long, short = 'n', value_name = "NAMESPACE")]
        namespace: Option<String>,

        /// Analysis period for historical metrics (e.g., 7d, 30d)
        #[arg(long, short = 'p', default_value = "7d")]
        period: String,

        /// Minimum severity to report (critical, warning, info)
        #[arg(long, short = 's')]
        severity: Option<String>,

        /// Minimum waste percentage threshold (0-100)
        #[arg(long, short = 't')]
        threshold: Option<u8>,

        /// Safety margin percentage for recommendations (default: 20)
        #[arg(long)]
        safety_margin: Option<u8>,

        /// Include info-level suggestions
        #[arg(long)]
        include_info: bool,

        /// Include system namespaces (kube-system, etc.)
        #[arg(long)]
        include_system: bool,

        /// Output format (table, json, yaml)
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,

        /// Write report to file
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Generate fix suggestions
        #[arg(long)]
        fix: bool,

        /// Apply fixes to manifest files (requires --fix or --full with live cluster)
        #[arg(long, requires = "fix")]
        apply: bool,

        /// Preview changes without applying (dry-run mode)
        #[arg(long)]
        dry_run: bool,

        /// Backup directory for original files before applying fixes
        #[arg(long, value_name = "DIR")]
        backup_dir: Option<PathBuf>,

        /// Minimum confidence threshold for auto-apply (0-100, default: 70)
        #[arg(long, default_value = "70")]
        min_confidence: u8,

        /// Cloud provider for cost estimation (aws, gcp, azure, onprem)
        #[arg(long, value_name = "PROVIDER")]
        cloud_provider: Option<String>,

        /// Region for cloud pricing (e.g., us-east-1, us-central1)
        #[arg(long, value_name = "REGION", default_value = "us-east-1")]
        region: String,

        /// Run comprehensive analysis (includes kubelint security checks and helmlint validation)
        #[arg(long, short = 'f')]
        full: bool,

        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
    },

    /// Retrieve stored output from a previous --agent command
    Retrieve {
        /// Reference ID (e.g., "security_a1b2c3d4") or "latest" for most recent
        #[arg(value_name = "REF_ID")]
        ref_id: Option<String>,

        /// Filter query (e.g., "severity:critical", "file:path", "section:frameworks")
        #[arg(long, short = 'q')]
        query: Option<String>,

        /// List all stored outputs
        #[arg(long)]
        list: bool,

        /// Maximum number of results to return (default: 20)
        #[arg(long, short = 'l', default_value = "20")]
        limit: usize,

        /// Number of results to skip (for pagination)
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// [DEPRECATED] Start an interactive AI chat session. Use AI coding agent skills instead.
    #[command(hide = true)]
    Chat {
        /// Path to the project directory (default: current directory)
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: PathBuf,

        /// LLM provider to use (uses saved preference by default)
        #[arg(long, value_enum, default_value = "auto")]
        provider: ChatProvider,

        /// Model to use (e.g., gpt-4o, claude-3-5-sonnet-latest, llama3.2)
        #[arg(long)]
        model: Option<String>,

        /// Run a single query instead of interactive mode
        #[arg(long)]
        query: Option<String>,

        /// Resume a previous session (accepts: "latest", session number, or UUID)
        #[arg(long, short = 'r')]
        resume: Option<String>,

        /// List available sessions for this project and exit
        #[arg(long)]
        list_sessions: bool,

        /// Start AG-UI server for frontend connectivity (SSE/WebSocket)
        #[arg(long)]
        ag_ui: bool,

        /// AG-UI server port (default: 9090)
        #[arg(long, default_value = "9090", requires = "ag_ui")]
        ag_ui_port: u16,
    },

    /// Authenticate with the Syncable platform
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },

    /// Manage Syncable projects
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },

    /// Manage Syncable organizations
    Org {
        #[command(subcommand)]
        command: OrgCommand,
    },

    /// Manage environments within a project
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },

    /// Deploy services to the Syncable platform (launches wizard by default)
    Deploy {
        /// Path to the project directory (default: current directory)
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: PathBuf,

        #[command(subcommand)]
        command: Option<DeployCommand>,
    },

    /// [DEPRECATED] Run as dedicated AG-UI agent server. Use AI coding agent skills instead.
    #[command(hide = true)]
    Agent {
        /// Path to the project directory
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: PathBuf,

        /// Port for AG-UI server
        #[arg(long, short, default_value = "9090")]
        port: u16,

        /// Host address to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// LLM provider to use
        #[arg(long, value_enum, default_value = "auto")]
        provider: ChatProvider,

        /// Model to use
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ToolsCommand {
    /// Check which vulnerability scanning tools are installed
    Status {
        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,

        /// Check tools for specific languages only
        #[arg(long, value_delimiter = ',')]
        languages: Option<Vec<String>>,
    },

    /// Install missing vulnerability scanning tools
    Install {
        /// Install tools for specific languages only
        #[arg(long, value_delimiter = ',')]
        languages: Option<Vec<String>>,

        /// Also install OWASP Dependency Check (large download)
        #[arg(long)]
        include_owasp: bool,

        /// Perform a dry run to show what would be installed
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Verify that installed tools are working correctly
    Verify {
        /// Test tools for specific languages only
        #[arg(long, value_delimiter = ',')]
        languages: Option<Vec<String>>,

        /// Show detailed verification output
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show tool installation guides for manual setup
    Guide {
        /// Show guide for specific languages only
        #[arg(long, value_delimiter = ',')]
        languages: Option<Vec<String>>,

        /// Show platform-specific instructions
        #[arg(long)]
        platform: Option<String>,
    },
}

/// Authentication subcommands for the Syncable platform
#[derive(Subcommand)]
pub enum AuthCommand {
    /// Log in to Syncable (opens browser for authentication)
    Login {
        /// Don't open browser automatically
        #[arg(long)]
        no_browser: bool,
    },

    /// Log out and clear stored credentials
    Logout,

    /// Show current authentication status
    Status,

    /// Print current access token (for scripting)
    Token {
        /// Print raw token without formatting
        #[arg(long)]
        raw: bool,
    },
}

/// Project management subcommands
#[derive(Subcommand)]
pub enum ProjectCommand {
    /// List projects in the current organization
    List {
        /// Organization ID to list projects from (uses current org if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Select a project to work with
    Select {
        /// Project ID to select
        id: String,
    },

    /// Show current organization and project context
    Current,

    /// Show details of a project
    Info {
        /// Project ID (uses current project if not specified)
        id: Option<String>,
    },
}

/// Organization management subcommands
#[derive(Subcommand)]
pub enum OrgCommand {
    /// List organizations you belong to
    List {
        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Select an organization to work with
    Select {
        /// Organization ID to select
        id: String,
    },
}

/// Environment management subcommands
#[derive(Subcommand)]
pub enum EnvCommand {
    /// List environments in the current project
    List {
        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Select an environment to work with
    Select {
        /// Environment ID to select
        id: String,
    },
}

/// Deployment subcommands
#[derive(Subcommand)]
pub enum DeployCommand {
    /// Launch interactive deployment wizard
    Wizard {
        /// Path to the project directory (default: current directory)
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: PathBuf,
    },

    /// Create a new environment for the current project
    NewEnv,

    /// Check deployment status
    Status {
        /// The deployment task ID (from deploy command output)
        task_id: String,

        /// Watch for status updates (poll until complete)
        #[arg(short, long)]
        watch: bool,
    },

    /// Preview deployment recommendation (non-interactive, JSON output for agents)
    Preview {
        /// Path to project or service subdirectory
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,

        /// Override service name (default: derived from directory name)
        #[arg(long)]
        service_name: Option<String>,

        /// Override cloud provider (gcp, hetzner, azure)
        #[arg(long)]
        provider: Option<String>,

        /// Override region
        #[arg(long)]
        region: Option<String>,

        /// Override machine type
        #[arg(long)]
        machine_type: Option<String>,

        /// Override detected port
        #[arg(long)]
        port: Option<u16>,

        /// Make service publicly accessible
        #[arg(long)]
        public: bool,
    },

    /// Deploy a service non-interactively (for agents and CI/CD)
    Run {
        /// Path to project or service subdirectory
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,

        /// Override service name (default: derived from directory name)
        #[arg(long)]
        service_name: Option<String>,

        /// Cloud provider (gcp, hetzner, azure)
        #[arg(long)]
        provider: Option<String>,

        /// Region
        #[arg(long)]
        region: Option<String>,

        /// Machine type
        #[arg(long)]
        machine_type: Option<String>,

        /// Port to expose
        #[arg(long)]
        port: Option<u16>,

        /// Make service publicly accessible
        #[arg(long)]
        public: bool,

        /// CPU allocation (for GCP/Azure, e.g. "1000m", "2")
        #[arg(long)]
        cpu: Option<String>,

        /// Memory allocation (for GCP/Azure, e.g. "512Mi", "2Gi")
        #[arg(long)]
        memory: Option<String>,

        /// Min instances/replicas
        #[arg(long)]
        min_instances: Option<i32>,

        /// Max instances/replicas
        #[arg(long)]
        max_instances: Option<i32>,

        /// Environment variable as KEY=VALUE (non-secret, repeatable)
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env_vars: Vec<String>,

        /// Secret key name (user prompted in terminal for value, repeatable)
        #[arg(long = "secret")]
        secrets: Vec<String>,

        /// Load environment variables from a .env file
        #[arg(long)]
        env_file: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DisplayFormat {
    /// Compact matrix/dashboard view (modern, easy to scan)
    Matrix,
    /// Detailed vertical view (legacy format with all details)
    Detailed,
    /// Brief summary only
    Summary,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ColorScheme {
    /// Auto-detect terminal background (default)
    Auto,
    /// Dark background terminal colors
    Dark,
    /// Light background terminal colors
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SeverityThreshold {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SecurityScanMode {
    /// Lightning fast scan - critical files only (.env, configs)
    Lightning,
    /// Fast scan - smart sampling with priority patterns
    Fast,
    /// Balanced scan - good coverage with performance optimizations (recommended)
    Balanced,
    /// Thorough scan - comprehensive analysis of all files
    Thorough,
    /// Paranoid scan - most comprehensive including low-severity findings
    Paranoid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ChatProvider {
    /// OpenAI (GPT-4o, GPT-4, etc.)
    Openai,
    /// Anthropic (Claude 3)
    Anthropic,
    /// AWS Bedrock (Claude via AWS)
    Bedrock,
    /// Ollama (local LLM, no API key needed)
    Ollama,
    /// Use saved default from config file
    #[default]
    Auto,
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
