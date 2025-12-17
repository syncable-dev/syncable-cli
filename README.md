# 🚀 Syncable IaC CLI

> Automatically generate optimized Docker, Kubernetes, and cloud infrastructure configurations by analyzing your codebase.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)


## ⚡ Quick Start
[![Crates.io Downloads](https://img.shields.io/crates/d/syncable-cli)](https://crates.io/crates/syncable-cli)

**Syncable IaC CLI** analyzes your project and automatically generates production-ready infrastructure configurations. Supporting **260+ technologies** across 5 major language ecosystems, it understands your stack and creates optimized IaC files tailored to your specific needs.

## ⚡ Quick Start


```bash
# Install (Cross-platform)
cargo install syncable-cli

# Windows users can also use:
# powershell -c "iwr -useb https://raw.githubusercontent.com/syncable-dev/syncable-cli/main/install.ps1 | iex"

# Analyze any project
sync-ctl analyze /path/to/your/project  # Unix/Linux/macOS
sync-ctl analyze C:\path\to\your\project  # Windows

# Check for vulnerabilities
sync-ctl vulnerabilities

# Run security analysis (multiple modes available)
sync-ctl security                   # Thorough scan (default)
sync-ctl security --mode lightning  # Ultra-fast critical files only
sync-ctl security --mode paranoid   # Most comprehensive scan

# AI Agent - Interactive DevOps assistant
sync-ctl chat

# Force update check (clears cache)
sync-ctl --clear-update-cache analyze .

# Get help with any command
sync-ctl --help                     # Show all available commands
sync-ctl analyze --help            # Show analyze command options
sync-ctl security --help           # Show security scanning options
sync-ctl vulnerabilities --help    # Show vulnerability check options
```

That's it! The CLI will detect your languages, frameworks, dependencies, and provide detailed insights about your project structure. The tool includes smart update notifications to keep you on the latest version.

## 🎯 What It Does

Syncable IaC CLI is like having a DevOps expert analyze your codebase:

1. **📊 Analyzes** - Detects languages, frameworks, dependencies, ports, and architecture patterns
2. **🔍 Audits** - Checks for security vulnerabilities and configuration issues  
3. **🚀 Generates** - Creates optimized Dockerfiles, Compose files, and Terraform configs (coming soon)

### Example Output

```bash
$ sync-ctl analyze ./my-express-app

═══════════════════════════════════════════════════════════════════════════════════════════════════
📊 PROJECT ANALYSIS DASHBOARD
═══════════════════════════════════════════════════════════════════════════════════════════════════

┌─ Architecture Overview ──────────────────────────────────────────────────────┐
│ Type:                                                         Single Project │
│ Pattern:                                                           Fullstack │
│ Full-stack app with frontend/backend  separation                             │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ Technology Stack ───────────────────────────────────────────────────────────┐
│ Languages:                                           JavaScript, TypeScript  │
│ Frameworks:                                    Express, React, Tailwind CSS  │
│ Databases:                                                PostgreSQL, Redis  │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 📋 Key Features

### 🔍 Comprehensive Analysis
- **Multi-language support** - JavaScript/TypeScript, Python, Rust, Go, Java/Kotlin
- **260+ technologies** - From React to Spring Boot, Django to Actix-web
- **Architecture detection** - Monolithic, microservices, serverless, and more
- **Monorepo support** - Analyzes complex multi-project repositories

### 🛡️ Turbo Security Engine (Covering Javascript / Python ---- Rust-, Go- & Java- Coming soon)
- **10-100x faster scanning** - Rust-powered multi-pattern matching with smart file discovery
- **5 scan modes** - From lightning-fast critical checks to comprehensive audits
- **Smart gitignore analysis** - Understands git status and provides risk assessments
- **260+ secret patterns** - Detects API keys, tokens, certificates, and credentials
- **Zero false positives** - Advanced context-aware filtering excludes test data and documentation

### 🐳 Docker Intelligence
- **Dockerfile analysis** - Understand existing Docker configurations
- **Multi-stage detection** - Identifies build optimization patterns
- **Service mapping** - Traces dependencies between containers
- **Network topology** - Visualizes service communication

### 🔄 Smart Update System
- **Intelligent caching** - Checks every 2 hours when no update available
- **Immediate notifications** - Shows updates instantly when available
- **Clear instructions** - Provides multiple update methods with step-by-step guidance
- **Zero-maintenance** - Automatically keeps you informed of new releases

### 🤖 AI Agent
- **Interactive chat** - Natural language DevOps assistant powered by OpenAI/Anthropic
- **Code generation** - Creates Dockerfiles, Terraform, Helm charts, and CI/CD configs
- **Project-aware** - Analyzes your codebase to generate optimized configurations
- **IDE integration** - Native diff views in VS Code for file changes

## 🛠️ Installation

### Via Cargo (Recommended - Cross Platform)
```bash
cargo install syncable-cli
```

### Quick Install Scripts

#### Linux/macOS
```bash
curl -sSL https://install.syncable.dev | sh
```

#### Windows (PowerShell)
```powershell
# Download and run the PowerShell installer
iwr -useb https://raw.githubusercontent.com/syncable-dev/syncable-cli/main/install.ps1 | iex

# Or download first and run (safer)
Invoke-WebRequest -Uri https://raw.githubusercontent.com/syncable-dev/syncable-cli/main/install.ps1 -OutFile install.ps1
powershell -ExecutionPolicy Bypass -File install.ps1
```

### From Source
```bash
git clone https://github.com/syncable-dev/syncable-cli.git
cd syncable-cli
cargo install --path .
```

### Platform-Specific Notes

**Windows Users:**
- **Rust**: Install from [rustup.rs](https://rustup.rs/) if you don't have it
- **PATH**: Cargo installs to `%USERPROFILE%\.cargo\bin` - add to PATH if needed
- **Tools**: Some security tools may require manual installation or package managers like Scoop/Chocolatey

**Linux/macOS Users:**
- Most security tools can be auto-installed via the installer script
- Tools are installed to `~/.local/bin` which may need to be added to your PATH

## 📖 Usage Guide

### Basic Commands

```bash
# Analyze with different display formats
sync-ctl analyze                    # Matrix view (default)
sync-ctl analyze --display detailed  # Detailed view
sync-ctl analyze --json             # JSON output

# Vulnerabilities analysis
sync-ctl vulnerabilities            # Dependency vulnerability scan

# Security analysis with turbo engine (10-100x faster)
sync-ctl security                   # Thorough scan (default) 
sync-ctl security --mode lightning  # Critical files only (.env, configs)
sync-ctl security --mode fast       # Smart sampling with priority patterns
sync-ctl security --mode balanced   # Good coverage with optimizations
sync-ctl security --mode paranoid   # Most comprehensive including low-severity
sync-ctl vulnerabilities            # Dependency vulnerability scan

# Dependency analysis
sync-ctl dependencies --licenses    # Show license information
sync-ctl dependencies --vulnerabilities  # Check for known CVEs
```

### 🤖 AI Agent

```bash
# Start interactive chat
sync-ctl chat

# Use specific provider/model
sync-ctl chat --provider openai --model gpt-4o
sync-ctl chat --provider anthropic --model claude-sonnet-4-20250514

# Single query mode
sync-ctl chat --query "Create a Dockerfile for this project"
```

**Commands in chat:**
- `/model` - Switch AI model
- `/provider` - Switch provider (OpenAI/Anthropic)
- `/clear` - Clear conversation
- `/exit` - Exit chat

**IDE Integration (VS Code):**

For native diff views when the agent modifies files:

1. Install [Syncable IDE Companion](https://marketplace.visualstudio.com/items?itemName=syncable.syncable-ide-companion)
2. Run `sync-ctl chat` from VS Code's integrated terminal
3. File changes open in VS Code's diff viewer instead of terminal

### Security Scan Modes

The turbo security engine offers 5 scan modes optimized for different use cases:

| Mode | Speed | Coverage | Use Case | Typical Time |
|------|-------|----------|----------|--------------|
| **Lightning** | 🚀 Fastest | Critical files only | Pre-commit hooks, CI checks 
| **Fast** | ⚡ Very Fast | Smart sampling | Development workflow 
| **Balanced** | 🎯 Optimized | Good coverage | Regular security checks 
| **Thorough** | 🔍 Complete | Comprehensive | Security audits (default) 
| **Paranoid** | 🕵️ Maximum | Everything + low severity | Compliance, releases 

## 🛡️ Security Detection Deep Dive

### What We Detect

The turbo security engine scans for 260+ secret patterns across multiple categories:

#### 🔑 API Keys & Tokens
- **Cloud Providers**: AWS Access Keys, GCP Service Account Keys, Azure Storage Keys
- **Services**: Stripe API Keys, Twilio Auth Tokens, GitHub Personal Access Tokens
- **Databases**: MongoDB Connection Strings, Redis URLs, PostgreSQL passwords
- **CI/CD**: Jenkins API Tokens, CircleCI Keys, GitLab CI Variables

#### 🔐 Cryptographic Material  
- **Private Keys**: RSA, ECDSA, Ed25519 private keys (.pem, .key files)
- **Certificates**: X.509 certificates, SSL/TLS certs
- **Keystores**: Java KeyStore files, PKCS#12 files
- **SSH Keys**: OpenSSH private keys, SSH certificates

#### 📧 Authentication Secrets
- **JWT Secrets**: JSON Web Token signing keys
- **OAuth**: Client secrets, refresh tokens
- **SMTP**: Email server credentials, SendGrid API keys
- **LDAP**: Bind credentials, directory service passwords

#### 🌐 Environment Variables
- **Suspicious Names**: Any variable containing "password", "secret", "key", "token"
- **Base64 Encoded**: Automatically detects encoded secrets
- **URLs with Auth**: Database URLs, API endpoints with embedded credentials

### Smart Git Status Analysis

Our security engine provides intelligent risk assessment based on git status:

| Status | Risk Level | Meaning | Action Needed |
|--------|------------|---------|---------------|
| 🟢 **SAFE** | Low | File properly ignored by .gitignore | ✅ No action needed |
| 🔵 **OK** | Low | File appears safe for version control | ✅ Monitor for changes |
| 🟡 **EXPOSED** | High | Contains secrets but NOT in .gitignore | ⚠️ Add to .gitignore immediately |
| 🔴 **TRACKED** | Critical | Contains secrets AND tracked by git | 🚨 Remove from git history |

#### Why Some Files Are "OK" Despite Not Being Gitignored

Files are marked as **OK** when they contain patterns that look like secrets but are actually safe:

- **Documentation**: Code in README files, API examples, tutorials
- **Test Data**: Mock API keys, placeholder values, example configurations  
- **Source Code**: String literals that match patterns but aren't real secrets
- **Lock Files**: Package hashes in `package-lock.json`, `pnpm-lock.yaml`, `cargo.lock`
- **Build Artifacts**: Compiled code, minified files, generated documentation

### Advanced False Positive Filtering

Our engine uses sophisticated techniques to minimize false positives:

#### 🎯 Context-Aware Detection
```bash
# ❌ FALSE POSITIVE - Will be ignored
const API_KEY = "your_api_key_here";  // Documentation example
const EXAMPLE_TOKEN = "sk-example123"; // Clearly a placeholder

# ✅ REAL SECRET - Will be detected  
const STRIPE_KEY = "sk_live_4eC39HqLyjWDarjtT1zdp7dc";
```

#### 📝 Documentation Exclusions
- Comments in any language (`//`, `#`, `/* */`, `<!-- -->`)
- Markdown code blocks and documentation files
- README files, CHANGELOG, API docs
- Example configurations and sample files

#### 🧪 Test Data Recognition
- Files in `/test/`, `/tests/`, `/spec/`, `__test__` directories
- Filenames containing "test", "spec", "mock", "fixture", "example"
- Common test patterns like "test123", "dummy", "fake"

#### 📦 Dependency File Intelligence
- Automatically excludes: `node_modules/`, `vendor/`, `target/`
- Recognizes lock files: `yarn.lock`, `pnpm-lock.yaml`, `go.sum`
- Skips binary files, images, and compiled artifacts

### Display Modes

Choose the output format that works best for you:

- **Matrix** (default) - Compact dashboard view
- **Detailed** - Comprehensive vertical layout  
- **Summary** - Brief overview for CI/CD
- **JSON** - Machine-readable format

### Example Security Output

```bash
$ sync-ctl security --mode thorough

🛡️  Security Analysis Results
════════════════════════════════════════════════════════════════════════════════

┌─ Security Summary ───────────────────────────────────────┐
│ Overall Score:                                    85/100 │
│ Risk Level:                                        High  │ 
│ Total Findings:                                        3 │
│ Files Analyzed:                                       47 │
│ Scan Mode:                                      Thorough │
└──────────────────────────────────────────────────────────┘

┌─ Security Findings ────────────────────────────────────────────────────────┐
│ 1. ./.env.local                                                            │
│    Type: ENV VAR | Severity: Critical | Position: 3:15 | Status: EXPOSED   │
│                                                                            │
│ 2. ./config/database.js                                                    │
│    Type: API KEY | Severity: High | Position: 12:23 | Status: TRACKED      │
│                                                                            │
│ 3. ./docs/api-example.md                                                   │
│    Type: API KEY | Severity: Critical | Position: 45:8 | Status: OK        │
└────────────────────────────────────────────────────────────────────────────┘

┌─ Key Recommendations ───────────────────────────────────────────────────────┐
│ 1. 🚨 Add .env.local to .gitignore immediately                              │
│ 2. 🔐 Move database credentials to environment variables                    │
│ 3. ✅ API example in docs is safely documented                              │
└─────────────────────────────────────────────────────────────────────────────┘

════════════════════════════════════════════════════════════════════════════════
```



### Advanced Configuration

Create `.syncable.toml` in your project root:

```toml
[analysis]
include_dev_dependencies = true
ignore_patterns = ["vendor", "node_modules", "target"]

[security]
# Scan configuration
default_mode = "thorough"              # Default scan mode
fail_on_high_severity = true           # Exit with error on high/critical findings
check_secrets = true                   # Enable secret detection
check_code_patterns = true             # Enable code security pattern analysis

# Performance tuning
max_file_size_mb = 10                  # Skip files larger than 10MB
worker_threads = 0                     # Auto-detect CPU cores (0 = auto)
enable_cache = true                    # Enable result caching
cache_size_mb = 100                    # Cache size limit

# Pattern filtering
priority_extensions = [                # Scan these extensions first
  "env", "key", "pem", "json", "yml", "yaml", 
  "toml", "ini", "conf", "config"
]
```

#### Command-Line Options

```bash
# Scan mode selection
sync-ctl security --mode lightning    # Fastest, critical files only
sync-ctl security --mode paranoid     # Slowest, most comprehensive

# Output control
sync-ctl security --json              # JSON output for automation
sync-ctl security --output report.json # Save to file

# Filtering options  
sync-ctl security --include-low       # Include low-severity findings
sync-ctl security --no-secrets        # Skip secret detection
sync-ctl security --no-code-patterns  # Skip code pattern analysis

# CI/CD integration
sync-ctl security --fail-on-findings  # Exit with error code if issues found
```

## 🌟 Technology Coverage

<details>
<summary><b>View Supported Technologies (260+)</b></summary>

### By Language

- **JavaScript/TypeScript** (46) - React, Vue, Angular, Next.js, Express, Nest.js, and more
- **Python** (76) - Django, Flask, FastAPI, NumPy, TensorFlow, PyTorch, and more
- **Java/JVM** (98) - Spring Boot, Micronaut, Hibernate, Kafka, Elasticsearch, and more
- **Go** (21) - Gin, Echo, Fiber, gRPC, Kubernetes client, and more
- **Rust** (20) - Actix-web, Axum, Rocket, Tokio, SeaORM, and more

### Package Managers
- npm, yarn, pnpm, bun (JavaScript/TypeScript)
- pip, poetry, pipenv, conda (Python)
- Maven, Gradle (Java)
- Cargo (Rust)
- Go modules (Go)

</details>

## 🚀 Roadmap

### ✅ Phase 1: Analysis Engine (Complete)
- Project analysis and technology detection
- Vulnerability scanning with 260+ supported packages
- Turbo Security Engine with 5 scan modes

### ✅ Phase 2: AI Agent (Complete)
- Interactive chat with OpenAI/Anthropic
- Dockerfile, Terraform, Helm chart generation
- VS Code IDE integration for native diffs

### 📅 Phase 3: Coming Soon
- Kubernetes manifest generation
- CI/CD pipeline templates
- Multi-cloud Terraform modules

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Run tests
cargo test

# Check code quality
cargo clippy

# Format code
cargo fmt
```


## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

Built with Rust 🦀 and powered by the open-source community.

---

**Need help?** Check our [documentation](https://github.com/syncable-dev/syncable-cli/wiki) or [open an issue](https://github.com/syncable-dev/syncable-cli/issues).

[![Star on GitHub](https://img.shields.io/github/stars/syncable-dev/syncable-cli?style=social)](https://github.com/syncable-dev/syncable-cli)

