<p align="center">
  <img src="logo.png" alt="Syncable" width="120" />
</p>

<h1 align="center">Syncable CLI</h1>

<p align="center">
  <strong>Your AI-Powered DevOps Engineer in the Terminal</strong>
</p>

<p align="center">
  <a href="https://crates.io/crates/syncable-cli"><img src="https://img.shields.io/crates/v/syncable-cli?style=flat-square&color=blue" alt="Crates.io"></a>
  <a href="https://crates.io/crates/syncable-cli"><img src="https://img.shields.io/crates/d/syncable-cli?style=flat-square" alt="Downloads"></a>
  <a href="https://www.gnu.org/licenses/gpl-3.0"><img src="https://img.shields.io/badge/License-GPL%20v3-blue.svg?style=flat-square" alt="License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Built%20with-Rust-orange?style=flat-square" alt="Rust"></a>
</p>

<p align="center">
  <a href="#-quick-start">Quick Start</a> •
  <a href="#-ai-agent">AI Agent</a> •
  <a href="#-features">Features</a> •
  <a href="#-installation">Installation</a> •
  <a href="https://syncable.dev">Syncable Platform →</a>
</p>

---

> **🚀 Ready to deploy?** Take your infrastructure to production with [Syncable Platform](https://syncable.dev) — seamless cloud deployments, monitoring, and team collaboration built on top of this CLI.

---

## What is Syncable CLI?

**Stop copy-pasting Dockerfiles from Stack Overflow.** Syncable CLI is an AI-powered assistant that understands your codebase and generates production-ready infrastructure — Dockerfiles, Kubernetes manifests, Terraform configs, and CI/CD pipelines — tailored specifically to your project.

```bash
$ sync-ctl chat
🤖 Syncable Agent powered by Claude

You: Create a production Dockerfile for this project

Agent: I've analyzed your Express.js + TypeScript project. Here's an optimized
multi-stage Dockerfile with:
  ✓ Non-root user for security
  ✓ Layer caching for faster builds
  ✓ Health checks configured
  ✓ Production dependencies only

[Creates Dockerfile with VS Code diff view]

You: Now add Redis caching and create a docker-compose

Agent: I'll add Redis to your stack and create a compose file...
```

<p align="center">
  <img src="syncable-cli-demo.gif" alt="Syncable CLI Demo" width="800" />
</p>

## ⚡ Quick Start

```bash
# Install
cargo install syncable-cli

# Start the AI Agent
sync-ctl chat

# Or run a quick analysis
sync-ctl analyze .
```

That's it. The agent analyzes your codebase, understands your stack, and helps you build infrastructure that actually works.

## 🤖 AI Agent

The Syncable Agent is like having a senior DevOps engineer available 24/7. It can:

### Generate Infrastructure
- **Dockerfiles** — Optimized multi-stage builds for any language
- **Docker Compose** — Full local development environments
- **Kubernetes** — Deployments, services, ingress, and more
- **Terraform** — Cloud infrastructure as code
- **CI/CD** — GitHub Actions, GitLab CI, Jenkins pipelines

### Understand Your Code
- Detects **260+ technologies** across JavaScript, Python, Go, Rust, and Java
- Identifies architecture patterns (monolith, microservices, serverless)
- Maps service dependencies and port configurations
- Reads your existing configs and improves them

### 🔌 VS Code Integration (Recommended)

For the best experience, install the **Syncable IDE Companion** extension:

```bash
code --install-extension syncable.syncable-ide-companion
```

This enables:
- **Native diff views** — Review file changes side-by-side in VS Code
- **One-click accept/reject** — Accept with `Cmd+S` or reject changes easily
- **Auto-detection** — Works automatically when running `sync-ctl chat` in VS Code's terminal

> Without the extension, the agent still works but shows diffs in the terminal instead.

### Chat Commands
| Command | Description |
|---------|-------------|
| `/model` | Switch AI model (GPT-4, Claude, etc.) |
| `/provider` | Switch between OpenAI and Anthropic |
| `/clear` | Clear conversation history |
| `/help` | Show available commands |

### Keyboard Shortcuts
| Shortcut | Action |
|----------|--------|
| `Ctrl+J` | Insert newline (multi-line input) |
| `Shift+Enter` | Insert newline |
| `@filename` | Add file to context |
| `Ctrl+C` | Cancel / Exit |

## 🔍 Features

### Project Analysis
```bash
sync-ctl analyze .
```
Get a complete breakdown of your project — languages, frameworks, databases, ports, and architecture patterns.

### Security Scanning
```bash
sync-ctl security
```
Blazing-fast secret detection powered by Rust. Finds API keys, tokens, and credentials in seconds, not minutes.

| Mode | Speed | Use Case |
|------|-------|----------|
| `--mode lightning` | 🚀 Fastest | Pre-commit hooks |
| `--mode fast` | ⚡ Fast | Development |
| `--mode thorough` | 🔍 Complete | Security audits |
| `--mode paranoid` | 🕵️ Maximum | Compliance |

### Vulnerability Checking
```bash
sync-ctl vulnerabilities
```
Scan your dependencies for known CVEs across npm, pip, cargo, and more.

## 📦 Installation

### Cargo (Recommended)
```bash
cargo install syncable-cli
```

### Linux/macOS
```bash
curl -sSL https://install.syncable.dev | sh
```

### Windows
```powershell
iwr -useb https://raw.githubusercontent.com/syncable-dev/syncable-cli/main/install.ps1 | iex
```

### From Source
```bash
git clone https://github.com/syncable-dev/syncable-cli.git
cd syncable-cli
cargo install --path .
```

## 🔧 Configuration

### AI Provider Setup
```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Project Config (`.syncable.toml`)
```toml
[agent]
default_provider = "anthropic"
default_model = "claude-sonnet-4-20250514"

[security]
default_mode = "thorough"
fail_on_high_severity = true

[analysis]
ignore_patterns = ["node_modules", "target", "dist"]
```

## 🌟 Supported Technologies

<details>
<summary><strong>260+ technologies across 5 ecosystems</strong></summary>

**JavaScript/TypeScript** — React, Vue, Angular, Next.js, Express, Nest.js, Fastify, and 40+ more

**Python** — Django, Flask, FastAPI, Celery, NumPy, TensorFlow, PyTorch, and 70+ more

**Go** — Gin, Echo, Fiber, gRPC, Kubernetes client, and 20+ more

**Rust** — Actix-web, Axum, Rocket, Tokio, SeaORM, and 20+ more

**Java/Kotlin** — Spring Boot, Micronaut, Quarkus, Hibernate, and 90+ more

</details>

## 🚀 What's Next?

This CLI is the foundation of the **Syncable Platform** — a complete DevOps solution that takes you from code to production:

- **One-click deployments** to AWS, GCP, or Azure
- **Team collaboration** with shared environments
- **Monitoring & logs** built-in
- **Cost optimization** recommendations

**[Get started at syncable.dev →](https://syncable.dev)**

## 🤝 Contributing

We love contributions! Whether it's bug fixes, new features, or documentation improvements.

```bash
# Clone and build
git clone https://github.com/syncable-dev/syncable-cli.git
cd syncable-cli
cargo build

# Run tests
cargo test

# Check code quality
cargo clippy && cargo fmt
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## 📄 License

This project is licensed under the **GNU General Public License v3.0** (GPL-3.0).

See [LICENSE](LICENSE) for the full license text.

### Third-Party Attributions

The Dockerfile linting functionality (`src/analyzer/hadolint/`) is a Rust translation
of [Hadolint](https://github.com/hadolint/hadolint), originally written in Haskell by
Lukas Martinelli and contributors. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)
for full attribution details.

---

<p align="center">
  <strong>Built with 🦀 Rust</strong>
  <br>
  <a href="https://github.com/syncable-dev/syncable-cli">GitHub</a> •
  <a href="https://syncable.dev">Website</a> •
  <a href="https://github.com/syncable-dev/syncable-cli/issues">Issues</a>
</p>

<p align="center">
  <a href="https://github.com/syncable-dev/syncable-cli"><img src="https://img.shields.io/github/stars/syncable-dev/syncable-cli?style=social" alt="GitHub stars"></a>
</p>
