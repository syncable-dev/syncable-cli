# 🚀 Syncable IaC CLI

> AI-powered Infrastructure-as-Code generator that analyzes your codebase and automatically creates optimized Docker, Docker Compose, and Terraform configurations.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ✨ Features

### 🔍 Comprehensive Project Analysis
- **Language Detection**: Automatically detects JavaScript/TypeScript, Python, Rust, Go, Java/Kotlin
- **Framework Recognition**: Identifies 70+ frameworks including Express, React, Django, FastAPI, Spring Boot
- **Dependency Analysis**: Parses all package managers and extracts version constraints
- **Vulnerability Scanning**: Integrates with security databases for each language ecosystem
- **Context Extraction**: Discovers entry points, ports, environment variables, and build scripts

### 🎯 Current Capabilities (Phase 1 Complete ✅)
- ✅ Multi-language project analysis
- ✅ Framework and library detection with confidence scoring
- ✅ Comprehensive dependency parsing
- ✅ Security vulnerability checking
- ✅ Project context analysis (ports, env vars, build scripts)
- ✅ Project type classification

### 🚧 Coming Soon (Phase 2+)
- 🤖 AI-powered Dockerfile generation
- 🐳 Intelligent Docker Compose creation
- ☁️ Cloud-ready Terraform configurations
- 🔒 Security hardening recommendations
- 📊 Performance optimization suggestions

## 📦 Installation

### From Source (Recommended)

```bash
# Prerequisites: Rust 1.70+ and Git

# Clone the repository
git clone https://github.com/yourusername/syncable-cli.git
cd syncable-cli

# Build and install
cargo install --path .

# Verify installation
sync-ctl --version
```

### Pre-built Binaries

Coming soon! Check the [releases page](https://github.com/yourusername/syncable-cli/releases).

## 🚀 Quick Start

### Analyze a Project

```bash
# Analyze current directory
sync-ctl analyze

# Analyze specific project
sync-ctl analyze /path/to/your/project

# Get JSON output
sync-ctl analyze --json > analysis.json
```

### Check for Vulnerabilities

```bash
# Run vulnerability scan
sync-ctl vuln-check /path/to/project

# Check only high severity and above
sync-ctl vuln-check --severity high
```

## 📖 Usage Examples

### Example: Node.js Express Application

```bash
$ sync-ctl analyze ./my-express-app

🔍 Analyzing project at: ./my-express-app
============================================================

📊 PROJECT ANALYSIS RESULTS
============================================================

🎯 Languages: JavaScript (Node.js 18)
🔧 Frameworks: Express, React
📦 Dependencies: 23 production, 15 development

🔌 Exposed Ports:
   - 3000 (Express server)
   - 9090 (Metrics endpoint)

🔐 Environment Variables:
   Required: DATABASE_URL, SECRET_KEY
   Optional: PORT, NODE_ENV, LOG_LEVEL

🔨 Build Scripts:
   - npm start
   - npm run dev
   - npm test
   - npm run build

✅ Project Type: Web Application
```

### Example: Python FastAPI Service

```bash
$ sync-ctl analyze ./fastapi-service --json
```

```json
{
  "project_type": "ApiService",
  "languages": [{
    "name": "Python",
    "version": "3.11",
    "confidence": 0.95
  }],
  "frameworks": [{
    "name": "FastAPI",
    "category": "Web",
    "confidence": 0.92
  }],
  "ports": [{ "number": 8000, "protocol": "Http" }],
  "environment_variables": [
    { "name": "DATABASE_URL", "required": true },
    { "name": "REDIS_URL", "required": false }
  ]
}
```

## 🛠️ Advanced Configuration

Create a `.syncable.toml` in your project:

```toml
[analysis]
include_dev_dependencies = true
deep_analysis = true
ignore_patterns = ["vendor", "node_modules", "target"]
max_file_size = 2097152  # 2MB

[output]
format = "json"  # or "yaml", "toml"
```

## 📚 Documentation

- [**Tutorial**](TUTORIAL.md) - Comprehensive usage guide
- [**Roadmap**](ROADMAP.md) - Development phases and upcoming features
- [**Architecture**](docs/architecture/README.md) - Technical design decisions
- [**API Reference**](docs/api/README.md) - Library usage documentation

## 🧪 Supported Technologies

### Languages & Runtimes
- JavaScript/TypeScript (Node.js)
- Python (3.7+)
- Rust
- Go
- Java/Kotlin

### Frameworks (70+ supported)
- **JavaScript**: Express, Next.js, React, Vue, Angular, Nest.js
- **Python**: Django, Flask, FastAPI, Pyramid
- **Rust**: Actix-web, Rocket, Axum, Warp
- **Go**: Gin, Echo, Fiber, Chi
- **Java**: Spring Boot, Micronaut, Quarkus

### Package Managers
- npm, yarn, pnpm
- pip, poetry, pipenv
- cargo
- go mod
- maven, gradle

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- analyze ./test-project

# Format code
cargo fmt

# Run linter
cargo clippy
```

## 📊 Project Status

### Phase 1: Core Analysis Engine ✅
- [x] Language Detection
- [x] Framework Detection  
- [x] Dependency Parsing
- [x] Vulnerability Checking
- [x] Project Context Analysis

### Phase 2: AI Integration 🚧
- [ ] AI Provider Integration
- [ ] Smart Dockerfile Generation
- [ ] Intelligent Docker Compose
- [ ] Cloud-Ready Terraform

See [ROADMAP.md](ROADMAP.md) for detailed progress.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) 🦀
- Uses [clap](https://github.com/clap-rs/clap) for CLI parsing
- Integrates with various security databases

---

**Built with ❤️ by the Syncable team** 