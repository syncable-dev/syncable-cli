# 🚀 Syncable IaC CLI

> Automatically generate optimized Docker, Kubernetes, and cloud infrastructure configurations by analyzing your codebase.
> Automatically generate optimized Docker, Kubernetes, and cloud infrastructure configurations by analyzing your codebase.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io Downloads](https://img.shields.io/crates/d/syncable-cli)](https://crates.io/crates/syncable-cli)

**Syncable IaC CLI** analyzes your project and automatically generates production-ready infrastructure configurations. Supporting **260+ technologies** across 5 major language ecosystems, it understands your stack and creates optimized IaC files tailored to your specific needs.

## ⚡ Quick Start
[![Crates.io Downloads](https://img.shields.io/crates/d/syncable-cli)](https://crates.io/crates/syncable-cli)

**Syncable IaC CLI** analyzes your project and automatically generates production-ready infrastructure configurations. Supporting **260+ technologies** across 5 major language ecosystems, it understands your stack and creates optimized IaC files tailored to your specific needs.

## ⚡ Quick Start

```bash
# Install
# Install
cargo install syncable-cli

# Analyze any project

# Analyze any project
sync-ctl analyze /path/to/your/project

# Check for vulnerabilities
sync-ctl vulnerabilities

# Run security analysis
sync-ctl security

# Force update check (clears cache)
sync-ctl --clear-update-cache analyze .
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

### 🛡️ Security & Compliance
- **Vulnerability scanning** - Integrated security checks for all dependencies
- **Secret detection** - Finds exposed API keys and credentials
- **Security scoring** - Get actionable security recommendations
- **Compliance checks** - SOC2, GDPR, HIPAA support (coming soon)

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

## 🛠️ Installation

### Via Cargo (Recommended)
```bash
cargo install syncable-cli
```

### From Source
```bash
git clone https://github.com/syncable-dev/syncable-cli.git
cd syncable-cli
cargo install --path .
```

## 📖 Usage Guide

### Basic Commands
cargo install syncable-cli
```

### From Source
```bash
git clone https://github.com/syncable-dev/syncable-cli.git
cd syncable-cli
cargo install --path .
```

## 📖 Usage Guide

### Basic Commands

```bash
# Analyze with different display formats
sync-ctl analyze                    # Matrix view (default)
sync-ctl analyze --display detailed  # Detailed view
sync-ctl analyze --json             # JSON output

# Security & vulnerability checks
sync-ctl security                   # Comprehensive security analysis
sync-ctl vulnerabilities            # Dependency vulnerability scan

# Dependency analysis
sync-ctl dependencies --licenses    # Show license information
sync-ctl dependencies --vulnerabilities  # Check for known CVEs
```

### Display Modes

Choose the output format that works best for you:

- **Matrix** (default) - Compact dashboard view
- **Detailed** - Comprehensive vertical layout  
- **Summary** - Brief overview for CI/CD
- **JSON** - Machine-readable format

### Advanced Configuration
# Analyze with different display formats
sync-ctl analyze                    # Matrix view (default)
sync-ctl analyze --display detailed  # Detailed view
sync-ctl analyze --json             # JSON output

# Security & vulnerability checks
sync-ctl security                   # Comprehensive security analysis
sync-ctl vulnerabilities            # Dependency vulnerability scan

# Dependency analysis
sync-ctl dependencies --licenses    # Show license information
sync-ctl dependencies --vulnerabilities  # Check for known CVEs
```

### Display Modes

Choose the output format that works best for you:

- **Matrix** (default) - Compact dashboard view
- **Detailed** - Comprehensive vertical layout  
- **Summary** - Brief overview for CI/CD
- **JSON** - Machine-readable format

### Advanced Configuration

Create `.syncable.toml` in your project root:
Create `.syncable.toml` in your project root:

```toml
[analysis]
include_dev_dependencies = true
ignore_patterns = ["vendor", "node_modules", "target"]

[security]
fail_on_high_severity = true
check_secrets = true
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
- npm, yarn, pnpm, bun (JavaScript)
- pip, poetry, pipenv, conda (Python)
- Maven, Gradle (Java)
- Cargo (Rust)
- Go modules (Go)

</details>

## 🚀 Roadmap

### ✅ Phase 1: Analysis Engine (Complete)
- Project analysis and technology detection
- Vulnerability scanning
- Basic security analysis

### 🔄 Phase 2: AI-Powered Generation (In Progress)
- Smart Dockerfile generation
- Intelligent Docker Compose creation
- Cloud-optimized configurations

### 📅 Future Phases
- Kubernetes manifests & Helm charts
- Terraform modules for AWS/GCP/Azure
- CI/CD pipeline generation
- Real-time monitoring setup

[security]
fail_on_high_severity = true
check_secrets = true
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
- npm, yarn, pnpm, bun (JavaScript)
- pip, poetry, pipenv, conda (Python)
- Maven, Gradle (Java)
- Cargo (Rust)
- Go modules (Go)

</details>

## 🚀 Roadmap

### ✅ Phase 1: Analysis Engine (Complete)
- Project analysis and technology detection
- Vulnerability scanning
- Basic security analysis

### 🔄 Phase 2: AI-Powered Generation (In Progress)
- Smart Dockerfile generation
- Intelligent Docker Compose creation
- Cloud-optimized configurations

### 📅 Future Phases
- Kubernetes manifests & Helm charts
- Terraform modules for AWS/GCP/Azure
- CI/CD pipeline generation
- Real-time monitoring setup

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Run tests
cargo test

# Check code quality
cargo clippy
# Check code quality
cargo clippy

# Format code
cargo fmt
```
```

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.
MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

Built with Rust 🦀 and powered by the open-source community.
Built with Rust 🦀 and powered by the open-source community.

---

**Need help?** Check our [documentation](https://github.com/syncable-dev/syncable-cli/wiki) or [open an issue](https://github.com/syncable-dev/syncable-cli/issues).

[![Star on GitHub](https://img.shields.io/github/stars/syncable-dev/syncable-cli?style=social)](https://github.com/syncable-dev/syncable-cli)
**Need help?** Check our [documentation](https://github.com/syncable-dev/syncable-cli/wiki) or [open an issue](https://github.com/syncable-dev/syncable-cli/issues).

[![Star on GitHub](https://img.shields.io/github/stars/syncable-dev/syncable-cli?style=social)](https://github.com/syncable-dev/syncable-cli)
