# 🚀 Syncable CLI - Complete Command Overview

This document provides a comprehensive overview of all available commands and their different display modes.

## 📊 Analysis Commands

### 1. Basic Project Analysis

```bash
# Modern matrix view (default) - compact dashboard
sync-ctl analyze .

# Detailed view with full Docker analysis 
sync-ctl analyze . --display detailed
# Or use the legacy flag
sync-ctl analyze . -d

# Summary view for CI/CD pipelines
sync-ctl analyze . --display summary

# JSON output for scripts
sync-ctl analyze . --json

# Analyze specific project path
sync-ctl analyze /path/to/project
```

### 2. Display Mode Comparison

#### Matrix View (Default) 🆕
- **Best for**: Quick overview, comparing multiple projects
- **Features**: Modern dashboard with box-drawing characters, side-by-side project comparison, key metrics
- **Docker Info**: Overview with service counts and orchestration patterns
- **Note**: Box alignment improvements in progress for better visual consistency

#### Detailed View 
- **Best for**: In-depth analysis, debugging, comprehensive reports
- **Features**: Full Docker analysis, complete technology breakdown, all metadata
- **Docker Info**: Complete Docker infrastructure analysis including:
  - Dockerfile analysis with base images, ports, stages
  - Docker Compose services with dependencies and networking
  - Orchestration patterns and service discovery
  - Port mappings and volume configurations
- **Usage**: Use this view when you need complete information about your project

#### Summary View
- **Best for**: CI/CD pipelines, quick status checks
- **Features**: Brief overview with essential information only
- **Usage**: Perfect for automated scripts and quick validation

## 🔍 Security & Vulnerability Commands

### 3. Security Analysis (Turbo Engine - 10-100x Faster)

```bash
# Comprehensive security scan (default: thorough mode)
sync-ctl security .

# Different scan modes for speed vs coverage
sync-ctl security . --mode lightning    # Fastest - critical files only
sync-ctl security . --mode fast        # Smart sampling
sync-ctl security . --mode balanced    # Good coverage
sync-ctl security . --mode thorough    # Comprehensive (default)
sync-ctl security . --mode paranoid    # Maximum coverage

# Include low-severity findings
sync-ctl security . --include-low

# Skip specific checks
sync-ctl security . --no-secrets --no-code-patterns

# Export security report
sync-ctl security . --output security-report.json --format json

# Fail CI/CD on security findings
sync-ctl security . --fail-on-findings
```

#### Security Scan Modes

| Mode | Speed | Coverage | Use Case |
|------|-------|----------|----------|
| **Lightning** | 🚀 Fastest | Critical files only | Pre-commit hooks, CI checks |
| **Fast** | ⚡ Very Fast | Smart sampling | Development workflow |
| **Balanced** | 🎯 Optimized | Good coverage | Regular security checks |
| **Thorough** | 🔍 Complete | Comprehensive | Security audits (default) |
| **Paranoid** | 🕵️ Maximum | Everything + low severity | Compliance, releases |

### 4. Vulnerability Scanning

```bash
# Scan all dependencies for vulnerabilities
sync-ctl vulnerabilities .

# Filter by severity
sync-ctl vulnerabilities . --severity high
sync-ctl vulnerabilities . --severity critical

# Export vulnerability report
sync-ctl vulnerabilities . --format json --output vulns.json

# Check specific project path
sync-ctl vulnerabilities /path/to/project
```

### 5. Dependency Analysis

```bash
# Analyze dependencies with licenses
sync-ctl dependencies . --licenses

# Include vulnerability checking
sync-ctl dependencies . --vulnerabilities

# Production dependencies only
sync-ctl dependencies . --prod-only

# Development dependencies only  
sync-ctl dependencies . --dev-only

# JSON output
sync-ctl dependencies . --format json
```

## 🛠️ Tool Management Commands

### 6. Vulnerability Scanning Tools

```bash
# Check tool installation status
sync-ctl tools status

# Install missing tools
sync-ctl tools install

# Install for specific languages
sync-ctl tools install --languages rust,python

# Include OWASP Dependency Check (large download)
sync-ctl tools install --include-owasp

# Verify tool functionality
sync-ctl tools verify

# Get installation guide
sync-ctl tools guide

# Platform-specific guides
sync-ctl tools guide --platform linux
```

## 🏗️ Generation Commands

### 7. IaC Generation

```bash
# Generate all IaC files
sync-ctl generate .
sync-ctl generate . --all

# Generate specific types
sync-ctl generate . --dockerfile --compose
sync-ctl generate . --terraform

# Dry run (preview only)
sync-ctl generate . --dry-run

# Custom output directory
sync-ctl generate . --output ./infrastructure/

# Overwrite existing files
sync-ctl generate . --force
```

## 🔄 Validation Commands (Coming Soon)

### 8. IaC Validation

```bash
# Validate generated IaC files (not yet implemented)
sync-ctl validate .

# Validate specific types (planned)
sync-ctl validate . --types dockerfile,compose

# Auto-fix issues (planned)
sync-ctl validate . --fix
```

## 📋 Information Commands

### 9. Support Information

```bash
# Show supported languages
sync-ctl support --languages

# Show supported frameworks
sync-ctl support --frameworks

# Show all supported technologies
sync-ctl support

# Detailed support information
sync-ctl support --detailed
```

## 🎯 Advanced Usage Examples

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
sync-ctl security . --fail-on-findings

# Vulnerability scan with threshold
sync-ctl vulnerabilities . --severity high

# JSON reports for processing
sync-ctl dependencies . --vulnerabilities --format json > deps.json
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
```

## 🔧 Global Configuration Options

### Global Flags (Available for all commands)
- `--config <file>` - Custom configuration file
- `--verbose` / `-v` - Verbose output (-v info, -vv debug, -vvv trace)
- `--quiet` - Suppress all output except errors
- `--json` - JSON output format where applicable
- `--clear-update-cache` - Force update check

### Command-Specific Options

#### Analysis Options
- `--display <mode>` - matrix (default), detailed, summary
- `--only <components>` - Analyze specific components only
- `--json` - JSON output for the analyze command

#### Security Options
- `--mode <scan-mode>` - lightning, fast, balanced, thorough, paranoid
- `--include-low` - Include low-severity findings
- `--no-secrets` - Skip secret detection
- `--no-code-patterns` - Skip code pattern analysis
- `--fail-on-findings` - Exit with error on security issues

#### Generation Options
- `--output <directory>` - Custom output directory
- `--dry-run` - Preview without creating files
- `--force` - Overwrite existing files
- `--all` - Generate all IaC types

#### Tool Options
- `--languages <list>` - Target specific languages
- `--include-owasp` - Include OWASP Dependency Check
- `--dry-run` - Preview installation
- `--yes` - Skip confirmation prompts

## 💡 Pro Tips

1. **For Development**: Use `--display detailed` to see complete Docker analysis
2. **For CI/CD**: Use `--display summary` for quick checks
3. **For Security**: Run `sync-ctl security . --fail-on-findings` in CI/CD
4. **For Performance**: Use `--mode lightning` for fastest security scans
5. **For Debugging**: Use `--verbose` for detailed logs
6. **For Automation**: Use `--json` output with other tools
7. **For Teams**: Share vulnerability reports with `--output` option
8. **For Updates**: Use `--clear-update-cache` to force update checks

## 🚀 Implementation Status

### ✅ Fully Implemented
- **analyze** - Project analysis with multiple display modes
- **security** - Turbo security engine with 5 scan modes
- **vulnerabilities** - Dependency vulnerability scanning
- **dependencies** - Comprehensive dependency analysis
- **support** - Technology support information
- **tools** - Vulnerability tool management

### 🚧 In Development
- **validate** - IaC validation and best practices checking
- **generate** - IaC file generation (Dockerfile, Compose, Terraform)
- Enhanced monorepo generation with per-project IaC files
- Advanced compliance framework checking

### 🔮 Coming Soon
- **Cloud Integration** - Deploy directly to cloud platforms
- **Monitoring Setup** - Automated monitoring configuration
- **Performance Analysis** - Resource optimization recommendations
- **Interactive Mode** - Guided setup and configuration wizard

## 📖 Getting Help

```bash
# Get help with any command
sync-ctl --help                     # Show all available commands
sync-ctl analyze --help            # Show analyze command options
sync-ctl security --help           # Show security scanning options
sync-ctl vulnerabilities --help    # Show vulnerability check options
sync-ctl generate --help           # Show generation options
sync-ctl tools --help              # Show tool management options
``` 