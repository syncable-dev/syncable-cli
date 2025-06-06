# 🚀 Syncable IaC CLI

> AI-powered Infrastructure-as-Code generator that analyzes your codebase and automatically creates optimized Docker, Docker Compose, and Terraform configurations.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
![Crates.io Downloads](https://img.shields.io/crates/d/syncable-cli)

## 🎯 **260+ Technologies Supported**
**The most comprehensive project analyzer supporting 5 major languages and their complete ecosystems:**
- ☕ **Java/JVM**: 98 technologies (13 Spring components + enterprise stack)
- 🐍 **Python**: 76 technologies (Django, FastAPI, ML/Data Science)  
- 🟨 **JavaScript/TypeScript**: 46 technologies (React, Next.js, Node.js)
- 🐹 **Go**: 21 technologies (cloud-native & microservices)
- 🦀 **Rust**: 20 technologies (high-performance web & systems)

## 🌟 Help Other Developers Discover This Tool

**If this tool saves you time, please consider giving it a ⭐ on GitHub!** 

Stars help other developers find Syncable CLI, and the more builders who discover it early, the better we can make it for everyone. Every star helps us reach developers who could benefit from automated infrastructure analysis and generation.

[⭐ **Star on GitHub**](https://github.com/syncable-dev/syncable-cli)


## ✨ Features

### 🔍 Comprehensive Project Analysis
- **Language Detection**: Automatically detects JavaScript/TypeScript, Python, Rust, Go, Java/Kotlin with precise version detection
- **Framework Recognition**: Identifies **260+ technologies** across all major ecosystems including complete Spring, Django, React, and Express families
- **Dependency Analysis**: Parses all package managers (npm/yarn/pnpm, pip/poetry, cargo, go mod, maven/gradle) and extracts version constraints
- **Vulnerability Scanning**: Integrates with security databases for each language ecosystem
- **Security Analysis**: Basic secret detection and environment variable security checks
- **Context Extraction**: Discovers entry points, ports, environment variables, and build scripts

### 🎯 Current Capabilities (Phase 1 Complete ✅)
- ✅ Multi-language project analysis
- ✅ Framework and library detection with confidence scoring
- ✅ Comprehensive dependency parsing
- ✅ Security vulnerability checking
- ✅ **Basic security analysis with secret detection**
- ✅ Project context analysis (ports, env vars, build scripts)
- ✅ Project type classification

### 🚧 Coming Soon (Phase 2+)
- 🤖 AI-powered Dockerfile generation
- 🐳 Intelligent Docker Compose creation
- ☁️ Cloud-ready Terraform configurations
- 🔒 **Advanced security analysis** (infrastructure, framework-specific, compliance)
- 📊 Performance optimization suggestions

### 🐳 Docker Infrastructure Analysis
**NEW**: Comprehensive Docker infrastructure analysis and understanding:

- **Dockerfile Analysis**: 
  - Supports all Dockerfile variants (`Dockerfile`, `dockerfile.dev`, `dockerfile.prod`, etc.)
  - Extracts base images, exposed ports, environment variables, and build stages
  - Detects multi-stage builds and complexity metrics
  - Environment-specific configuration detection

- **Docker Compose Analysis**:
  - Supports all compose file variants (`docker-compose.yml`, `docker-compose.dev.yaml`, etc.)
  - Service dependency mapping and network topology analysis
  - Port mapping analysis (external/internal, host/container)
  - Volume mount analysis and data persistence patterns

- **Service Discovery & Networking**:
  - Internal DNS and service communication patterns
  - Custom network analysis and service isolation
  - Load balancer detection (nginx, traefik, haproxy, kong)
  - API gateway identification and ingress patterns

- **Orchestration Pattern Detection**:
  - Single Container applications
  - Docker Compose multi-service setups
  - Microservices architecture patterns
  - Event-driven architecture (with message queues)
  - Service mesh detection (Istio, Linkerd, Envoy)

- **Monorepo Docker Support**:
  - Analyzes Docker configurations across multiple projects
  - Maps services to their respective project contexts
  - Handles compose files at repository root with project-specific Dockerfiles

## 📦 Installation

### ⚡ Quick Install

The fastest way to get started:

```bash
cargo install syncable-cli
```

Or see below for building from source.

### From Source (Recommended)

```bash
# Prerequisites: Rust 1.70+ and Git

# Clone the repository
git clone https://github.com/syncable-dev/syncable-cli.git
cd syncable-cli

# Build and install
cargo install --path .

# Verify installation
sync-ctl --version
```

### Pre-built Binaries

Coming soon! Check the [releases page](https://github.com/syncable-dev/syncable-cli/releases).

## 🚀 Quick Start

### Analyze a Project

```bash
# Analyze current directory
sync-ctl analyze

# Analyze specific project
sync-ctl analyze /path/to/your/project

# Get JSON output
sync-ctl analyze --json > analysis.json

# Use different display modes (NEW!)
sync-ctl analyze --display matrix    # Modern dashboard view (default)
sync-ctl analyze --display summary   # Brief summary only
sync-ctl analyze --display detailed  # Legacy verbose output
sync-ctl analyze -d                   # Shorthand for detailed
```

### 📊 Display Modes (NEW!)

The analyze command now offers multiple display formats:

- **Matrix View** (default): A modern, compact dashboard with side-by-side project comparison
- **Summary View**: Brief overview perfect for CI/CD pipelines
- **Detailed View**: Traditional verbose output with all project details
- **JSON**: Machine-readable format for integration with other tools

See the [Display Modes Documentation](docs/cli-display-modes.md) for visual examples and more details.

### Check for Vulnerabilities

```bash
# Run vulnerability scan
sync-ctl vulnerabilities /path/to/project

# Check only high severity and above
sync-ctl vulnerabilities --severity high

# Export vulnerability report
sync-ctl vulnerabilities --format json --output vuln-report.json
```

### Security Analysis

```bash
# Basic security analysis with secret detection
sync-ctl security /path/to/project

# Include low severity findings
sync-ctl security --include-low

# Skip specific analysis types
sync-ctl security --no-secrets --no-code-patterns

# Generate security report
sync-ctl security --format json --output security-report.json

# Fail CI/CD pipeline on security findings
sync-ctl security --fail-on-findings
```

**Current Security Features:**
- ✅ Secret detection (API keys, tokens, passwords)
- ✅ Environment variable security analysis
- ✅ Basic code pattern analysis (limited rules)
- ✅ Security scoring and risk assessment
- 🚧 Infrastructure security analysis (coming soon)
- 🚧 Framework-specific security checks (coming soon)
- 🚧 Compliance framework validation (coming soon)

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
### Example: Security Analysis

```bash
$ sync-ctl security ./my-project

🛡️  Finalizing analysis... [00:00:01] ▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰ 100/100 100%

🛡️  Security Analysis Results
============================================================

📊 SECURITY SUMMARY
✅ Security Score: 100.0/100

🔍 ANALYSIS SCOPE
✅ Secret Detection         (5 files analyzed)
✅ Environment Variables    (3 variables checked)
ℹ️  Code Security Patterns   (no applicable files found)
🚧 Infrastructure Security  (coming soon)
🚧 Compliance Frameworks    (coming soon)

🎯 FINDINGS BY CATEGORY
🔐 Secret Detection: 0 findings
🔒 Code Security: 0 findings
🏗️ Infrastructure: 0 findings
📋 Compliance: 0 findings

💡 RECOMMENDATIONS
• Enable dependency vulnerability scanning in CI/CD
• Consider implementing rate limiting for API endpoints
• Review environment variable security practices
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

## 🧪 Comprehensive Technology Support (260+ Technologies)

### 📊 Coverage by Language
- **☕ Java/JVM**: **98 technologies** - The most comprehensive JVM ecosystem coverage
- **🐍 Python**: **76 technologies** - Complete Python web, data, and ML stack
- **🟨 JavaScript/TypeScript**: **46 technologies** - Full-stack web development ecosystem  
- **🐹 Go**: **21 technologies** - Modern cloud-native and microservices tools
- **🦀 Rust**: **20 technologies** - High-performance systems and web frameworks

### 🌟 Major Ecosystem Coverage

#### ☕ **Java/JVM Ecosystem** (98 technologies)
**Spring Family** (13 technologies):
- Spring Boot, Spring Framework, Spring Security, Spring Data
- Spring Cloud (Gateway, Config, Netflix), Spring WebFlux, Spring MVC
- Spring Batch, Spring Integration, Spring AOP, and more

**Enterprise & Microservices**: Quarkus, Micronaut, Dropwizard, Jakarta EE  
**Database & ORM**: Hibernate, MyBatis, JPA, JDBI, MongoDB Driver, Redis Jedis  
**Message Brokers**: Apache Kafka, RabbitMQ, ActiveMQ, Apache Pulsar  
**Search & Big Data**: Elasticsearch, Apache Solr, Apache Spark, Apache Flink  
**Security**: Apache Shiro, Keycloak, Bouncy Castle, JWT, OAuth2  
**Build Tools**: Maven, Gradle, Ant  
**Testing**: JUnit, TestNG, Mockito, Selenium, Cucumber, Testcontainers  
**Web Servers**: Tomcat, Jetty, Undertow, Netty  

#### 🐍 **Python Ecosystem** (76 technologies)  
**Web Frameworks**: Django, Flask, FastAPI, Pyramid, CherryPy, Tornado, Falcon  
**Django Family**: Django REST Framework, Django ORM, Django-allauth  
**Data & ML**: NumPy, Pandas, Scikit-learn, TensorFlow, PyTorch, Keras  
**Database & ORM**: SQLAlchemy, Alembic, psycopg2, PyMongo, Redis-py  
**Async & Messaging**: Celery, asyncio, aiohttp, Dramatiq  
**Scientific**: Matplotlib, Seaborn, Jupyter, SciPy  
**WSGI/ASGI Servers**: Gunicorn, Uvicorn, Hypercorn, Daphne, Waitress  
**Testing**: pytest, unittest, nose2, behave, Robot Framework  

#### 🟨 **JavaScript/TypeScript Ecosystem** (46 technologies)
**Meta-Frameworks**: Next.js, Nuxt.js, SvelteKit, Astro, SolidStart, Tanstack Start  
**Frontend**: React, Vue.js, Angular, Svelte, SolidJS  
**Mobile**: React Native, Expo  
**Backend**: Express.js, Nest.js, Fastify, Hono, Elysia  
**Database/ORM**: Prisma, Drizzle ORM, TypeORM, Mongoose, Sequelize  
**Build Tools**: Vite, Webpack, Rollup, Parcel  
**Runtimes**: Node.js, Bun, Deno, Cloudflare Workers, Vercel Edge  
**Testing**: Jest, Vitest, Cypress, Playwright  

#### 🐹 **Go Ecosystem** (21 technologies)
**Web Frameworks**: Gin, Echo, Fiber, Chi, Gorilla Mux, Beego  
**Microservices**: gRPC, go-kit, go-micro  
**Database**: GORM, sqlx, pgx  
**Cloud Native**: Kubernetes client, Docker, Consul  
**Testing**: Testify, Ginkgo, GoConvey  

#### 🦀 **Rust Ecosystem** (20 technologies)  
**Web Frameworks**: Actix-web, Axum, Rocket, Warp, Tide  
**Async Runtimes**: Tokio, async-std  
**Database/ORM**: SeaORM, Diesel, SQLx  
**Serialization**: Serde  
**Testing**: Built-in test framework, criterion (benchmarking)

### 📦 **Package Manager Support**
- **JavaScript**: npm, yarn, pnpm, bun
- **Python**: pip, poetry, pipenv, conda, pdm  
- **Java**: Maven, Gradle
- **Rust**: Cargo
- **Go**: go mod
- **PHP**: Composer
- **Ruby**: Bundler

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
- [x] **Basic Security Analysis** (secret detection, env vars)
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
