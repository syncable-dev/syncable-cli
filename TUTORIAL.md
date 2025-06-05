# 📚 Syncable CLI Tutorial

Welcome to the Syncable Infrastructure-as-Code CLI! This tool analyzes your codebase and automatically generates Docker, Docker Compose, and Terraform configurations. This tutorial will guide you through installation, basic usage, and advanced features.

## 📋 Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Core Features](#core-features)
- [Usage Examples](#usage-examples)
- [Understanding the Analysis](#understanding-the-analysis)
- [Troubleshooting](#troubleshooting)

## 🚀 Installation

### Option 1: Install from Source (Recommended)

1. **Prerequisites**
   - Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
   - Git

2. **Clone and Build**
   ```bash
   # Clone the repository
   git clone https://github.com/yourusername/syncable-cli.git
   cd syncable-cli

   # Build the project
   cargo build --release

   # The binary will be at ./target/release/iac-gen
   ```

3. **Install Globally**
   ```bash
   # Install to your system
   cargo install --path .

   # Now you can use 'iac-gen' from anywhere
   iac-gen --version
   ```

### Option 2: Download Pre-built Binary (Coming Soon)

```bash
# macOS
curl -L https://github.com/yourusername/syncable-cli/releases/latest/download/iac-gen-macos -o iac-gen
chmod +x iac-gen
sudo mv iac-gen /usr/local/bin/

# Linux
curl -L https://github.com/yourusername/syncable-cli/releases/latest/download/iac-gen-linux -o iac-gen
chmod +x iac-gen
sudo mv iac-gen /usr/local/bin/

# Windows
# Download iac-gen-windows.exe from releases page
```

## 🏁 Quick Start

### Basic Project Analysis

```bash
# Analyze the current directory
iac-gen analyze

# Analyze a specific project
iac-gen analyze /path/to/your/project

# Get JSON output for scripting
iac-gen analyze --json
```

### Vulnerability Scanning

```bash
# Check for vulnerabilities in dependencies
iac-gen vuln-check /path/to/project

# Check with specific severity threshold
iac-gen vuln-check --severity high /path/to/project
```

## 🎯 Core Features

### 1. Language & Framework Detection

The CLI automatically detects:
- **Languages**: JavaScript/TypeScript, Python, Rust, Go, Java/Kotlin
- **Frameworks**: Express, React, Next.js, Django, Flask, FastAPI, Spring Boot, Actix-web, and 70+ more
- **Package Managers**: npm, yarn, pnpm, pip, cargo, go mod, maven, gradle

### 2. Dependency Analysis

- Parses all dependency files (package.json, requirements.txt, Cargo.toml, etc.)
- Identifies production vs development dependencies
- Extracts version constraints
- Detects licenses

### 3. Vulnerability Scanning

- Integrates with security databases for each language
- Provides severity ratings (Critical, High, Medium, Low)
- Suggests remediation steps
- Generates security reports

### 4. Project Context Analysis

- **Entry Points**: Identifies main files and startup commands
- **Ports**: Detects exposed ports from code and configs
- **Environment Variables**: Extracts required and optional env vars
- **Build Scripts**: Finds all build and run commands
- **Project Type**: Determines if it's a web app, API, CLI tool, etc.

## 📖 Usage Examples

### Example 1: Analyzing a Node.js Express App

```bash
$ iac-gen analyze ~/projects/my-express-app

🔍 Analyzing project at: /Users/john/projects/my-express-app
============================================================

📊 PROJECT ANALYSIS RESULTS
============================================================

🎯 Languages Detected:
   - JavaScript (confidence: 95%)
     Version: Node.js 18
     Files: 47 JavaScript files found
     Package Manager: npm

🔧 Frameworks Detected:
   - Express (confidence: 90%)
     Category: Web Framework
   - React (confidence: 85%)
     Category: Frontend Framework

📦 Dependencies:
   Production: 23 packages
   Development: 15 packages
   
🔌 Exposed Ports:
   - Port 3000: Express server (HTTP)
   - Port 9090: Metrics endpoint (HTTP)

🔐 Environment Variables:
   Required:
     - DATABASE_URL
     - SECRET_KEY
   Optional:
     - PORT (default: 3000)
     - NODE_ENV (default: development)
     - LOG_LEVEL (default: info)

🔨 Build Scripts:
   - start: node server.js
   - dev: nodemon server.js
   - build: webpack --mode production
   - test: jest

✅ Analysis complete! Project type: Web Application
```

### Example 2: Vulnerability Check

```bash
$ iac-gen vuln-check ~/projects/my-express-app

🔍 Checking vulnerabilities for: /Users/john/projects/my-express-app
============================================================

⚠️  VULNERABILITY REPORT
============================================================

Language: JavaScript (npm audit)

🔴 Critical (1):
   - Package: lodash
     Version: 4.17.15
     Vulnerability: Prototype Pollution
     Fix: Update to version 4.17.21

🟠 High (2):
   - Package: express
     Version: 4.17.0
     Vulnerability: XSS in Error Handler
     Fix: Update to version 4.17.3

🟡 Medium (3):
   - Package: axios
     Version: 0.21.1
     Vulnerability: SSRF
     Fix: Update to version 0.21.2

✅ Total vulnerabilities: 6
   Critical: 1, High: 2, Medium: 3, Low: 0

Run 'npm update' to fix most issues.
```

### Example 3: Analyzing a Python FastAPI Project

```bash
$ iac-gen analyze ~/projects/fastapi-app --json
```

```json
{
  "project_root": "/Users/john/projects/fastapi-app",
  "languages": [{
    "name": "Python",
    "version": "3.11",
    "confidence": 0.95,
    "package_manager": "pip"
  }],
  "frameworks": [{
    "name": "FastAPI",
    "category": "Web",
    "confidence": 0.92
  }],
  "entry_points": [{
    "file": "main.py",
    "function": "main",
    "command": "uvicorn main:app"
  }],
  "ports": [
    {
      "number": 8000,
      "protocol": "Http",
      "description": "FastAPI server"
    }
  ],
  "environment_variables": [
    {
      "name": "DATABASE_URL",
      "required": true
    },
    {
      "name": "REDIS_URL",
      "required": false,
      "default_value": "redis://localhost:6379"
    }
  ],
  "project_type": "ApiService"
}
```

## 🔍 Understanding the Analysis

### Language Detection
- Analyzes file extensions and content
- Checks for language-specific files (package.json, requirements.txt, etc.)
- Detects version from runtime files or lock files

### Framework Detection
- Pattern matching in dependency files
- Imports and configuration file analysis
- Confidence scoring based on multiple indicators

### Project Context
- **Entry Points**: Scans for main files, console scripts, binary definitions
- **Ports**: Regex matching in code, Dockerfile EXPOSE, docker-compose ports
- **Environment Variables**: Scans for `process.env`, `os.environ`, ENV directives
- **Build Scripts**: Extracts from package.json, Makefile, setup.py, etc.

### Project Type Classification
- **Web Application**: Has web framework + frontend components
- **API Service**: Backend framework without UI
- **CLI Tool**: Has CLI framework or single entry point without ports
- **Library**: No main entry point, meant to be imported
- **Microservice**: Multiple services or complex docker-compose
- **Static Site**: Static site generator detected

## 🛠️ Advanced Usage

### Custom Configuration

Create a `.syncable.toml` file in your project:

```toml
[analysis]
# Include dev dependencies in analysis
include_dev_dependencies = true

# Enable deep analysis (slower but more thorough)
deep_analysis = true

# Custom ignore patterns
ignore_patterns = ["vendor", "temp", "_build"]

# Maximum file size to analyze (bytes)
max_file_size = 2097152  # 2MB

[output]
# Output format preferences
format = "json"  # or "yaml", "toml"
```

### CI/CD Integration

```yaml
# GitHub Actions example
name: IaC Analysis
on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Syncable CLI
        run: |
          curl -L https://github.com/yourusername/syncable-cli/releases/latest/download/iac-gen-linux -o iac-gen
          chmod +x iac-gen
          sudo mv iac-gen /usr/local/bin/
      
      - name: Run Analysis
        run: iac-gen analyze --json > analysis.json
        
      - name: Check Vulnerabilities
        run: |
          iac-gen vuln-check --severity high
          if [ $? -ne 0 ]; then
            echo "High severity vulnerabilities found!"
            exit 1
          fi
```

## 🐛 Troubleshooting

### Common Issues

1. **"Language not detected"**
   - Ensure you have standard project files (package.json, requirements.txt, etc.)
   - Check that files aren't in ignored directories

2. **"No frameworks detected"**
   - Framework detection relies on dependencies being declared
   - Ensure dependencies are in manifest files, not just imported

3. **"Vulnerability check failed"**
   - Some checks require tools to be installed (npm, pip, etc.)
   - Run with `--verbose` for detailed error messages

4. **Performance Issues**
   - Use ignore patterns for large directories
   - Reduce max_file_size in configuration
   - Disable deep_analysis for faster results

### Debug Mode

Run with verbose logging:
```bash
# Show debug information
RUST_LOG=debug iac-gen analyze /path/to/project

# Show only warnings and errors
RUST_LOG=warn iac-gen analyze /path/to/project
```

## 🚧 Upcoming Features (Phase 2)

- **AI-Powered Generation**: Generate optimized Dockerfiles and configs
- **Multi-stage Docker builds**: Intelligent optimization
- **Terraform Generation**: Cloud-ready infrastructure configs
- **Security Hardening**: Automated security best practices
- **Performance Optimization**: Resource usage analysis

## 📞 Getting Help

- **Documentation**: See `/docs` folder in the repository
- **Issues**: Report bugs on GitHub Issues
- **Community**: Join our Discord server
- **Email**: support@syncable.io

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

**Happy analyzing! 🚀** 