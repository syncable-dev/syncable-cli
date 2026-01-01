# 🚀 Syncable CLI - Complete Command Reference

This document provides a comprehensive reference for all Syncable CLI commands, options, and workflows.

## 📑 Table of Contents

- [Global Options](#-global-options)
- [Commands](#-commands)
  - [analyze](#1-sync-ctl-analyze) - Project analysis
  - [generate](#2-sync-ctl-generate) - IaC generation
  - [validate](#3-sync-ctl-validate) - IaC validation (planned)
  - [support](#4-sync-ctl-support) - Show supported tech
  - [dependencies](#5-sync-ctl-dependencies) - Dependency analysis
  - [vulnerabilities](#6-sync-ctl-vulnerabilities) - CVE scanning
  - [security](#7-sync-ctl-security) - Comprehensive security scan
  - [tools](#8-sync-ctl-tools) - Tool management
  - [chat](#9-sync-ctl-chat) - AI assistant
  - [auth](#10-sync-ctl-auth) - Authentication
- [Configuration](#-configuration)
- [Common Workflows](#-common-workflows)
- [VS Code Integration](#-vs-code-integration)
- [Environment Variables](#-environment-variables)
- [Exit Codes](#-exit-codes)

---

## 🌐 Global Options

Available for all commands:

| Flag | Short | Description |
|------|-------|-------------|
| `--config <FILE>` | `-c` | Path to configuration file |
| `--verbose` | `-v` | Enable verbose logging (`-v` info, `-vv` debug, `-vvv` trace) |
| `--quiet` | `-q` | Suppress all output except errors |
| `--json` | | Output in JSON format where applicable |
| `--clear-update-cache` | | Clear update check cache and force a new check |
| `--disable-telemetry` | | Disable telemetry data collection |
| `--help` | `-h` | Show help information |
| `--version` | `-V` | Show version information |

---

## 📚 Commands

### 1. `sync-ctl analyze <PROJECT_PATH>`

Analyze a project and display detected components (languages, frameworks, dependencies, architecture).

**Arguments:**
- `<PROJECT_PATH>` — Path to the project directory to analyze

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--json` | `-j` | Output analysis results in JSON format |
| `--detailed` | `-d` | Show detailed analysis information (legacy format) |
| `--display <FORMAT>` | | Display format: `matrix` (default), `detailed`, `summary` |
| `--only <ASPECTS>` | | Only analyze specific aspects (comma-separated: languages, frameworks, dependencies) |
| `--color-scheme <SCHEME>` | | Color scheme: `auto` (default), `dark`, `light` |

**Display Mode Comparison:**

#### Matrix View (Default) 🆕
- **Best for**: Quick overview, comparing multiple projects
- **Features**: Modern dashboard with box-drawing characters, side-by-side project comparison, key metrics
- **Docker Info**: Overview with service counts and orchestration patterns

#### Detailed View
- **Best for**: In-depth analysis, debugging, comprehensive reports
- **Features**: Full Docker analysis, complete technology breakdown, all metadata
- **Docker Info**: Complete Docker infrastructure analysis including:
  - Dockerfile analysis with base images, ports, stages
  - Docker Compose services with dependencies and networking
  - Orchestration patterns and service discovery
  - Port mappings and volume configurations

#### Summary View
- **Best for**: CI/CD pipelines, quick status checks
- **Features**: Brief overview with essential information only

**Examples:**

```bash
# Basic analysis with modern matrix view
sync-ctl analyze .

# Detailed view with full Docker analysis
sync-ctl analyze . --display detailed
# Or use the legacy flag
sync-ctl analyze . -d

# Summary view for CI/CD pipelines
sync-ctl analyze . --display summary

# JSON output for scripting
sync-ctl analyze . --json

# Only show languages and frameworks
sync-ctl analyze . --only languages,frameworks

# Use light terminal theme colors
sync-ctl analyze . --color-scheme light

# Analyze specific project path
sync-ctl analyze /path/to/project
```

**Supported Technologies:**
- **Languages**: Rust, JavaScript/TypeScript, Python, Go, Java
- **Frameworks**: 260+ including React, Vue, Angular, Next.js, Express, Django, Flask, FastAPI, Spring Boot, and more
- **Databases**: PostgreSQL, MySQL, MongoDB, Redis, and more
- **Tools**: Docker, Kubernetes, Terraform, and more

---

### 2. `sync-ctl generate <PROJECT_PATH>`

Generate Infrastructure as Code files (Dockerfile, docker-compose.yml, Terraform config).

**Arguments:**
- `<PROJECT_PATH>` — Path to the project directory

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--output <OUTPUT_DIR>` | `-o` | Output directory for generated files |
| `--dockerfile` | | Generate Dockerfile |
| `--compose` | | Generate Docker Compose file |
| `--terraform` | | Generate Terraform configuration |
| `--all` | | Generate all supported IaC files (default if no specific flag) |
| `--dry-run` | | Preview output without creating files |
| `--force` | | Overwrite existing files |

**Examples:**

```bash
# Generate all IaC files
sync-ctl generate .
sync-ctl generate . --all

# Only generate Dockerfile
sync-ctl generate . --dockerfile

# Generate Dockerfile and Compose
sync-ctl generate . --dockerfile --compose

# Preview without creating files
sync-ctl generate . --dry-run

# Custom output directory
sync-ctl generate . --output ./infrastructure/

# Generate and overwrite existing files
sync-ctl generate . --all --force
```

**Status:** 🚧 In Development
- Basic generation implemented
- Enhanced monorepo generation with per-project IaC files coming soon

---

### 3. `sync-ctl validate <PATH>`

Validate existing IaC files against best practices.

**Arguments:**
- `<PATH>` — Path to directory containing IaC files

**Options:**

| Flag | Description |
|------|-------------|
| `--types <TYPES>` | Types of files to validate (comma-separated) |
| `--fix` | Automatically fix issues where possible |

**Examples:**

```bash
# Validate all IaC files
sync-ctl validate .

# Validate specific types
sync-ctl validate . --types dockerfile,compose

# Auto-fix issues
sync-ctl validate . --fix
```

**Status:** ⚠️ Not yet implemented

---

### 4. `sync-ctl support`

Show supported languages and frameworks.

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--languages` | | Show only languages |
| `--frameworks` | | Show only frameworks |
| `--detailed` | `-d` | Show detailed information |

**Examples:**

```bash
# Show all supported tech
sync-ctl support

# Only show languages
sync-ctl support --languages

# Only show frameworks
sync-ctl support --frameworks

# Detailed information
sync-ctl support --detailed
```

**Supported Technologies:**

<details>
<summary><strong>260+ technologies across 5 ecosystems</strong></summary>

**JavaScript/TypeScript** — React, Vue, Angular, Next.js, Express, Nest.js, Fastify, and 40+ more

**Python** — Django, Flask, FastAPI, Celery, NumPy, TensorFlow, PyTorch, and 70+ more

**Go** — Gin, Echo, Fiber, gRPC, Kubernetes client, and 20+ more

**Rust** — Actix-web, Axum, Rocket, Tokio, SeaORM, and 20+ more

**Java/Kotlin** — Spring Boot, Micronaut, Quarkus, Hibernate, and 90+ more

</details>

---

### 5. `sync-ctl dependencies <PROJECT_PATH>`

Analyze project dependencies in detail.

**Arguments:**
- `<PROJECT_PATH>` — Path to the project directory

**Options:**

| Flag | Description |
|------|-------------|
| `--licenses` | Show license information for dependencies |
| `--vulnerabilities` | Check for known vulnerabilities |
| `--prod-only` | Show only production dependencies |
| `--dev-only` | Show only development dependencies |
| `--format <FORMAT>` | Output format: `table` (default) or `json` |

**Examples:**

```bash
# Show all dependencies
sync-ctl dependencies .

# Analyze with licenses
sync-ctl dependencies . --licenses

# Check for vulnerabilities
sync-ctl dependencies . --vulnerabilities

# Show only production dependencies with licenses
sync-ctl dependencies . --prod-only --licenses

# Development dependencies only
sync-ctl dependencies . --dev-only

# JSON output
sync-ctl dependencies . --format json
sync-ctl dependencies . --vulnerabilities --format json > deps.json
```

**Supported Package Managers:**
- npm, yarn, pnpm, bun (JavaScript)
- pip, pipenv, poetry (Python)
- cargo (Rust)
- go.mod (Go)
- maven, gradle (Java)

---

### 6. `sync-ctl vulnerabilities [PATH]`

Check dependencies for known security vulnerabilities (CVEs).

**Arguments:**
- `[PATH]` — Path to scan (default: current directory `.`)

**Options:**

| Flag | Description |
|------|-------------|
| `--severity <LEVEL>` | Show only vulnerabilities with severity >= threshold (`low`, `medium`, `high`, `critical`) |
| `--format <FORMAT>` | Output format: `table` (default) or `json` |
| `--output <FILE>` | Export report to file |

**Examples:**

```bash
# Scan current directory
sync-ctl vulnerabilities
sync-ctl vulnerabilities .

# Only show high and critical vulnerabilities
sync-ctl vulnerabilities . --severity high
sync-ctl vulnerabilities . --severity critical

# Only show critical vulnerabilities
sync-ctl vulnerabilities . --severity critical

# Export report to file (table format)
sync-ctl vulnerabilities . --output report.txt

# Export JSON report
sync-ctl vulnerabilities . --output report.json --format json

# Scan specific path with severity filter
sync-ctl vulnerabilities /path/to/project --severity medium
```

**Supported Package Managers:**
- npm, yarn, pnpm, bun (JavaScript)
- pip (Python)
- cargo (Rust)
- go mod (Go)
- maven, gradle (Java)

**Exit Behavior:**
- Exits with code `1` if critical or high severity vulnerabilities are found
- Use this in CI/CD to fail builds on security issues

---

### 7. `sync-ctl security [PROJECT_PATH]`

Perform comprehensive security analysis (secrets detection, code patterns, vulnerabilities).

**Arguments:**
- `[PROJECT_PATH]` — Path to analyze (default: current directory `.`)

**Options:**

| Flag | Description |
|------|-------------|
| `--mode <MODE>` | Scan mode: `lightning`, `fast`, `balanced`, `thorough` (default), `paranoid` |
| `--include-low` | Include low severity findings |
| `--no-secrets` | Skip secrets detection |
| `--no-code-patterns` | Skip code pattern analysis |
| `--format <FORMAT>` | Output format: `table` (default) or `json` |
| `--output <FILE>` | Export report to file |
| `--fail-on-findings` | Exit with error code on security findings |

**Security Scan Modes:**

| Mode | Speed | Coverage | Use Case |
|------|-------|----------|----------|
| `lightning` | 🚀 Fastest | Critical files only | Pre-commit hooks, CI checks |
| `fast` | ⚡ Very Fast | Smart sampling | Development workflow |
| `balanced` | 🎯 Optimized | Good coverage | Regular security checks |
| `thorough` | 🔍 Complete | Comprehensive | Security audits (default) |
| `paranoid` | 🕵️ Maximum | Everything + low severity | Compliance, releases |

**Examples:**

```bash
# Comprehensive security scan (default: thorough mode)
sync-ctl security
sync-ctl security .

# Lightning-fast scan for pre-commit hooks
sync-ctl security --mode lightning
sync-ctl security . --mode lightning

# Fast scan for development
sync-ctl security . --mode fast

# Balanced scan (recommended for regular checks)
sync-ctl security . --mode balanced

# Paranoid scan with low severity findings
sync-ctl security . --mode paranoid
sync-ctl security . --mode paranoid --include-low

# Skip specific checks
sync-ctl security . --no-secrets
sync-ctl security . --no-code-patterns
sync-ctl security . --no-secrets --no-code-patterns

# Export security report
sync-ctl security . --output security-report.txt
sync-ctl security . --output security-report.json --format json

# Fail CI/CD on security findings
sync-ctl security . --fail-on-findings
sync-ctl security . --mode fast --fail-on-findings --format json
```

**What It Detects:**
- **Secrets**: API keys, tokens, passwords, credentials
- **Code Patterns**: SQL injection, XSS, insecure crypto, hardcoded secrets
- **Vulnerabilities**: Known CVEs in dependencies
- **Security Best Practices**: File permissions, configuration issues

**Performance:**
- Powered by Turbo Engine (10-100x faster than traditional scanners)
- Uses parallel processing and smart caching
- Blazing-fast secret detection powered by Rust

---

### 8. `sync-ctl tools <SUBCOMMAND>`

Manage vulnerability scanning tools (install, check status, verify).

#### 8a. `sync-ctl tools status`

Check which vulnerability scanning tools are installed.

**Options:**

| Flag | Description |
|------|-------------|
| `--format <FORMAT>` | Output format: `table` (default) or `json` |
| `--languages <LANGS>` | Check tools for specific languages only (comma-separated) |

**Examples:**

```bash
# Check all tools
sync-ctl tools status

# Check JavaScript tools only
sync-ctl tools status --languages javascript

# Check multiple languages
sync-ctl tools status --languages rust,python,javascript

# JSON output
sync-ctl tools status --format json
```

#### 8b. `sync-ctl tools install`

Install missing vulnerability scanning tools.

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--languages <LANGS>` | | Install tools for specific languages only (comma-separated) |
| `--include-owasp` | | Also install OWASP Dependency Check (large download) |
| `--dry-run` | | Show what would be installed without installing |
| `--yes` | `-y` | Skip confirmation prompts |

**Examples:**

```bash
# Install all missing tools (interactive)
sync-ctl tools install

# Install JavaScript tools only
sync-ctl tools install --languages javascript

# Install multiple language tools
sync-ctl tools install --languages rust,python

# Include OWASP Dependency Check (large download)
sync-ctl tools install --include-owasp

# Dry run to see what would be installed
sync-ctl tools install --dry-run

# Install without prompts
sync-ctl tools install --yes
sync-ctl tools install -y
```

#### 8c. `sync-ctl tools verify`

Verify that installed tools are working correctly.

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--languages <LANGS>` | | Test tools for specific languages only |
| `--detailed` | `-d` | Show detailed verification output |

**Examples:**

```bash
# Verify all tools
sync-ctl tools verify

# Verify specific language tools
sync-ctl tools verify --languages javascript

# Verify with detailed output
sync-ctl tools verify --detailed
sync-ctl tools verify -d
```

#### 8d. `sync-ctl tools guide`

Show tool installation guides for manual setup.

**Options:**

| Flag | Description |
|------|-------------|
| `--languages <LANGS>` | Show guide for specific languages only |
| `--platform <PLATFORM>` | Show platform-specific instructions |

**Examples:**

```bash
# Show all installation guides
sync-ctl tools guide

# Show Python tools guide
sync-ctl tools guide --languages python

# Platform-specific guide
sync-ctl tools guide --platform macos
sync-ctl tools guide --platform linux
sync-ctl tools guide --platform windows
```

**Supported Tools:**
- **JavaScript**: npm audit, yarn audit, pnpm audit, retire.js
- **Python**: pip-audit, safety
- **Rust**: cargo-audit
- **Go**: nancy
- **Java**: OWASP Dependency Check

---

### 9. `sync-ctl chat [PROJECT_PATH]`

Start an interactive AI chat session to analyze, generate, and understand your project.

**Arguments:**
- `[PROJECT_PATH]` — Path to the project directory (default: current directory `.`)

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--provider <PROVIDER>` | | LLM provider: `openai`, `anthropic`, `bedrock`, `ollama`, `auto` (default) |
| `--model <MODEL>` | | Model to use (e.g., `gpt-4o`, `claude-3-5-sonnet-latest`, `llama3.2`) |
| `--query <QUERY>` | | Run a single query instead of interactive mode |
| `--resume <SESSION>` | `-r` | Resume a previous session (`latest`, session number, or UUID) |
| `--list-sessions` | | List available sessions for this project and exit |

**Examples:**

```bash
# Start interactive chat (auto-detect provider)
sync-ctl chat
sync-ctl chat .

# Use specific provider
sync-ctl chat --provider openai
sync-ctl chat --provider anthropic
sync-ctl chat --provider bedrock
sync-ctl chat --provider ollama

# Use specific model
sync-ctl chat --model gpt-4o
sync-ctl chat --model claude-3-5-sonnet-latest
sync-ctl chat --model llama3.2

# Run a single query
sync-ctl chat --query "Generate a Dockerfile for this project"
sync-ctl chat --query "What security issues are in my code?"
sync-ctl chat --query "Optimize my Docker Compose file"

# Resume latest session
sync-ctl chat --resume latest
sync-ctl chat -r latest

# Resume specific session (by number)
sync-ctl chat --resume 3
sync-ctl chat -r 3

# Resume by UUID
sync-ctl chat --resume 8f7a9b2c

# List all sessions
sync-ctl chat --list-sessions

# Analyze specific project path
sync-ctl chat /path/to/project
```

**Chat Commands** (inside interactive session):

| Command | Description |
|---------|-------------|
| `/model` | Switch AI model (GPT-4, Claude, etc.) |
| `/provider` | Switch between OpenAI and Anthropic |
| `/clear` | Clear conversation history |
| `/help` | Show available commands |

**Keyboard Shortcuts:**

| Shortcut | Action |
|----------|--------|
| `Ctrl+J` | Insert newline (multi-line input) |
| `Shift+Enter` | Insert newline |
| `@filename` | Add file to context |
| `Ctrl+C` | Cancel / Exit |

**Supported Providers:**

| Provider | Models | API Key Required |
|----------|--------|------------------|
| **OpenAI** | GPT-4o, GPT-4, GPT-3.5 | Yes (`OPENAI_API_KEY`) |
| **Anthropic** | Claude 3 (Sonnet, Opus, Haiku) | Yes (`ANTHROPIC_API_KEY`) |
| **Bedrock** | Claude via AWS | Yes (AWS credentials) |
| **Ollama** | Llama 3.2, Mistral, etc. | No (local) |

**Setup:**

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."

# Ollama (no key needed, install Ollama first)
# Download from https://ollama.ai
```

**What the Agent Can Do:**
- Generate Dockerfiles, Docker Compose, Kubernetes manifests, Terraform configs
- Analyze your codebase and detect 260+ technologies
- Security scanning and vulnerability detection
- Create CI/CD pipelines (GitHub Actions, GitLab CI)
- Answer questions about your project structure
- Suggest optimizations and best practices

---

### 10. `sync-ctl auth <SUBCOMMAND>`

Authenticate with the Syncable platform.

#### 10a. `sync-ctl auth login`

Log in to Syncable (opens browser for authentication).

**Options:**

| Flag | Description |
|------|-------------|
| `--no-browser` | Don't open browser automatically |

**Examples:**

```bash
# Login (opens browser)
sync-ctl auth login

# Login without auto-opening browser
sync-ctl auth login --no-browser
```

#### 10b. `sync-ctl auth logout`

Log out and clear stored credentials.

**Examples:**

```bash
sync-ctl auth logout
```

#### 10c. `sync-ctl auth status`

Show current authentication status.

**Examples:**

```bash
sync-ctl auth status
```

#### 10d. `sync-ctl auth token`

Print current access token (for scripting).

**Options:**

| Flag | Description |
|------|-------------|
| `--raw` | Print raw token without formatting |

**Examples:**

```bash
# Show formatted token
sync-ctl auth token

# Raw token for scripting
sync-ctl auth token --raw

# Use in API calls
curl -H "Authorization: Bearer $(sync-ctl auth token --raw)" https://api.syncable.dev/
```

---

## ⚙️ Configuration

### Configuration File (`.syncable.toml`)

Place in your project root or `~/.config/syncable/config.toml`:

```toml
[agent]
default_provider = "anthropic"
default_model = "claude-sonnet-4-20250514"

[security]
default_mode = "thorough"
fail_on_high_severity = true

[analysis]
ignore_patterns = ["node_modules", "target", "dist", ".git"]

[telemetry]
enabled = true
```

**Configuration Options:**

#### `[agent]` Section
- `default_provider` — Default LLM provider (`openai`, `anthropic`, `bedrock`, `ollama`)
- `default_model` — Default model name (e.g., `gpt-4o`, `claude-3-5-sonnet-latest`)

#### `[security]` Section
- `default_mode` — Default security scan mode (`lightning`, `fast`, `balanced`, `thorough`, `paranoid`)
- `fail_on_high_severity` — Exit with error on high severity findings (boolean)

#### `[analysis]` Section
- `ignore_patterns` — Patterns to ignore during analysis (array of strings)

#### `[telemetry]` Section
- `enabled` — Enable/disable telemetry (boolean)

---

## 🎯 Common Workflows

### Complete Project Analysis Workflow

```bash
# 1. Quick overview
sync-ctl analyze .

# 2. Detailed analysis with Docker
sync-ctl analyze . --display detailed

# 3. Security scan
sync-ctl security .

# 4. Vulnerability check
sync-ctl vulnerabilities . --severity medium

# 5. Generate IaC
sync-ctl generate . --all
```

### CI/CD Integration

```bash
# Quick check for CI/CD
sync-ctl analyze . --display summary

# Security scan that fails on findings
sync-ctl security . --mode fast --fail-on-findings

# Vulnerability scan with threshold (fails on critical/high)
sync-ctl vulnerabilities . --severity high

# JSON reports for processing
sync-ctl dependencies . --vulnerabilities --format json > deps.json
sync-ctl security . --format json --output security.json
```

### Monorepo Analysis

```bash
# Analyze entire monorepo
sync-ctl analyze .

# Matrix view shows all projects side-by-side
sync-ctl analyze . --display matrix

# Individual project analysis
cd frontend && sync-ctl analyze . --display detailed
cd ../backend && sync-ctl analyze . --display detailed

# Security scan across entire monorepo
sync-ctl security . --mode thorough
```

### Security Audit

```bash
# Comprehensive security analysis
sync-ctl security . --mode paranoid --include-low

# Check vulnerabilities
sync-ctl vulnerabilities . --severity low

# Export reports
sync-ctl security . --output security-report.json --format json
sync-ctl vulnerabilities . --output vuln-report.json --format json

# Fail on findings (for compliance)
sync-ctl security . --fail-on-findings
```

### Development Workflow

```bash
# Quick analysis before commit
sync-ctl analyze . --display summary

# Fast security scan
sync-ctl security . --mode lightning

# Generate Dockerfile for new service
sync-ctl generate . --dockerfile

# Ask AI for help
sync-ctl chat --query "How do I optimize this Dockerfile?"
```

### Pre-commit Hook

```bash
# .git/hooks/pre-commit
#!/bin/bash
sync-ctl security . --mode lightning --fail-on-findings
```

---

## 🔌 VS Code Integration

Install the **Syncable IDE Companion** extension for enhanced AI chat experience:

```bash
code --install-extension syncable.syncable-ide-companion
```

**Features:**
- **Native diff views** — Review file changes side-by-side in VS Code
- **One-click accept/reject** — Accept with `Cmd+S` or reject changes easily
- **Auto-detection** — Works automatically when running `sync-ctl chat` in VS Code's terminal

**Without the extension:**
- The agent still works but shows diffs in the terminal instead

---

## 🌍 Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key for chat (required for OpenAI provider) |
| `ANTHROPIC_API_KEY` | Anthropic API key for chat (required for Anthropic provider) |
| `SYNC_CTL_DEBUG` | Enable debug logging (set to any value) |
| `CARGO_PKG_VERSION` | CLI version (auto-set by Cargo) |

**Setup:**

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."

# Debug mode
export SYNC_CTL_DEBUG=1
```

---

## 🚦 Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error occurred or security findings detected |

**Exit Code Behavior:**

- **`vulnerabilities` command**: Exits with `1` if critical or high severity vulnerabilities are found
- **`security` command**: Exits with `1` when `--fail-on-findings` is used and findings are detected
- **All commands**: Exits with `1` on any error

---

## 💡 Pro Tips

1. **For Development**: Use `--display detailed` to see complete Docker analysis
2. **For CI/CD**: Use `--display summary` for quick checks
3. **For Security**: Run `sync-ctl security . --fail-on-findings` in CI/CD
4. **For Performance**: Use `--mode lightning` for fastest security scans
5. **For Debugging**: Use `--verbose` for detailed logs
6. **For Automation**: Use `--json` output with other tools
7. **For Teams**: Share vulnerability reports with `--output` option
8. **For Updates**: Use `--clear-update-cache` to force update checks
9. **For Monorepos**: Use matrix view to see all projects at once
10. **For AI Help**: Use `sync-ctl chat` for interactive assistance

---

## 🚀 Implementation Status

### ✅ Fully Implemented
- **analyze** — Project analysis with multiple display modes
- **security** — Turbo security engine with 5 scan modes
- **vulnerabilities** — Dependency vulnerability scanning
- **dependencies** — Comprehensive dependency analysis
- **support** — Technology support information
- **tools** — Vulnerability tool management
- **chat** — AI assistant with multiple providers
- **auth** — Platform authentication

### 🚧 In Development
- **generate** — IaC file generation (basic implementation done)
- Enhanced monorepo generation with per-project IaC files
- Advanced compliance framework checking

### 🔮 Coming Soon
- **validate** — IaC validation and best practices checking
- **Cloud Integration** — Deploy directly to cloud platforms
- **Monitoring Setup** — Automated monitoring configuration
- **Performance Analysis** — Resource optimization recommendations
- **Interactive Mode** — Guided setup and configuration wizard

---

## 📖 Getting Help

```bash
# Get help with any command
sync-ctl --help                     # Show all available commands
sync-ctl analyze --help            # Show analyze command options
sync-ctl generate --help           # Show generation options
sync-ctl security --help           # Show security scanning options
sync-ctl vulnerabilities --help    # Show vulnerability check options
sync-ctl dependencies --help       # Show dependency analysis options
sync-ctl tools --help              # Show tool management options
sync-ctl chat --help               # Show AI chat options
sync-ctl auth --help               # Show authentication options
```

---

## 📚 Additional Resources

- **GitHub**: [github.com/syncable-dev/syncable-cli](https://github.com/syncable-dev/syncable-cli)
- **Website**: [syncable.dev](https://syncable.dev)
- **Issues**: [github.com/syncable-dev/syncable-cli/issues](https://github.com/syncable-dev/syncable-cli/issues)
- **Contributing**: [CONTRIBUTING.md](../CONTRIBUTING.md)
- **Changelog**: [CHANGELOG.md](../CHANGELOG.md)

---

**Built with 🦀 Rust** — Your AI-Powered DevOps Engineer in the Terminal
