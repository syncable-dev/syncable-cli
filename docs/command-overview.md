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

## 🔍 Security & Vulnerability Commands

### 3. Security Analysis

```bash
# Comprehensive security scan
sync-ctl security .

# Include low-severity findings
sync-ctl security . --include-low

# Skip specific checks
sync-ctl security . --no-secrets --no-code-patterns

# Export security report
sync-ctl security . --output security-report.json --format json

# Fail CI/CD on security findings
sync-ctl security . --fail-on-findings
```

### 4. Vulnerability Scanning

```bash
# Scan all dependencies for vulnerabilities
sync-ctl vulnerabilities .

# Filter by severity
sync-ctl vulnerabilities . --severity high

# Export vulnerability report
sync-ctl vulnerabilities . --format json --output vulns.json
```

### 5. Dependency Analysis

```bash
# Analyze dependencies with licenses
sync-ctl dependencies . --licenses

# Include vulnerability checking
sync-ctl dependencies . --vulnerabilities

# Production dependencies only
sync-ctl dependencies . --prod-only

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

# Verify tool functionality
sync-ctl tools verify

# Get installation guide
sync-ctl tools guide
```

## 🏗️ Generation Commands

### 7. IaC Generation

```bash
# Generate all IaC files
sync-ctl generate .

# Generate specific types
sync-ctl generate . --dockerfile --compose
sync-ctl generate . --terraform

# Dry run (preview only)
sync-ctl generate . --dry-run

# Custom output directory
sync-ctl generate . --output ./infrastructure/
```

## 🔄 Validation Commands

### 8. IaC Validation (Coming Soon)

```bash
# Validate generated IaC files
sync-ctl validate .

# Validate specific types
sync-ctl validate . --types dockerfile,compose

# Auto-fix issues
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

## 🔧 Configuration Options

### Global Options
- `--config <file>` - Custom configuration file
- `--verbose` / `-v` - Verbose output
- `--json` - JSON output format

### Analysis Options
- `--display <mode>` - matrix (default), detailed, summary
- `--only <components>` - Analyze specific components only

### Security Options
- `--include-low` - Include low-severity findings
- `--no-secrets` - Skip secret detection
- `--no-code-patterns` - Skip code pattern analysis
- `--frameworks <list>` - Check specific frameworks

### Tool Options
- `--languages <list>` - Target specific languages
- `--dry-run` - Preview installation
- `--yes` - Skip confirmation prompts

## 💡 Pro Tips

1. **For Development**: Use `--display detailed` to see complete Docker analysis
2. **For CI/CD**: Use `--display summary` for quick checks
3. **For Security**: Run `sync-ctl security . --fail-on-findings` in CI/CD
4. **For Debugging**: Use `--verbose` for detailed logs
5. **For Automation**: Use `--json` output with other tools
6. **For Teams**: Share vulnerability reports with `--output` option

## 🚀 What's Coming Next

- **Validation Commands**: Validate generated IaC files
- **Advanced Security**: Infrastructure security scanning
- **Cloud Integration**: Deploy directly to cloud platforms
- **Monitoring Setup**: Automated monitoring configuration
- **Performance Analysis**: Resource optimization recommendations 