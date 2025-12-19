# Syncable CLI - Development Roadmap

> **AI-powered DevOps in your terminal** — Analyze codebases, generate infrastructure, deploy with confidence.

---

## ✅ Completed Features

### 🔍 Analysis Engine
- [x] **Language Detection** — 5 language ecosystems with version detection
- [x] **Framework Detection** — 260+ frameworks across JavaScript, Python, Go, Rust, Java/Kotlin
- [x] **Dependency Analysis** — Parse package.json, Cargo.toml, requirements.txt, go.mod, pom.xml
- [x] **Architecture Detection** — Monolith, microservices, serverless patterns
- [x] **Port & Service Discovery** — Automatic port detection and service mapping

### 🛡️ Security & Vulnerability Scanning
- [x] **Turbo Security Engine** — Rust-powered, 10-100x faster than traditional scanners
- [x] **Secret Detection** — 260+ patterns (API keys, tokens, certificates, credentials)
- [x] **5 Scan Modes** — Lightning, Fast, Balanced, Thorough, Paranoid
- [x] **Smart Filtering** — Context-aware false positive elimination
- [x] **Git Status Analysis** — Risk assessment based on tracked/ignored status
- [x] **Vulnerability Database Integration**:
  - [x] npm audit (JavaScript)
  - [x] pip-audit (Python)
  - [x] cargo-audit (Rust)
  - [x] govulncheck (Go)
  - [x] OWASP/Grype (Java)

### 🤖 AI Agent
- [x] **Multi-Provider Support**:
  - [x] OpenAI (GPT-5, GPT-4, GPT-4o, GPT-3.5)
  - [x] Anthropic (Claude Sonnet, Claude Opus)
- [x] **Interactive Chat** — Natural language DevOps assistance
- [x] **Project-Aware Context** — Analyzes codebase before generating
- [x] **File Context** — `@filename` to include files in conversation
- [x] **Slash Commands** — `/model`, `/provider`, `/clear`, `/help`

### 🐳 Infrastructure Generation
- [x] **Dockerfile Generation** — Optimized multi-stage builds
- [x] **Docker Compose** — Full service stacks with dependencies
- [x] **Context-Aware** — Generates based on actual project analysis

### 💻 Developer Experience
- [x] **VS Code Integration** — Native diff views for file changes
- [x] **IDE Companion Extension** — Seamless file editing workflow
- [x] **Multi-line Input** — Ctrl+J, Shift+Enter for newlines
- [x] **Smart Keyboard Shortcuts**:
  - [x] `@` file picker for context
  - [x] `/` slash commands
  - [x] Ctrl+Shift+Backspace (delete to line start)
  - [x] Ctrl+W / Alt+Backspace (delete word)
  - [x] Ctrl+U (clear input)
- [x] **Bracketed Paste** — Proper multi-line paste handling

---

## 🚧 In Progress

### 🏗️ Terraform Generation
- [ ] AWS ECS/Fargate configurations
- [ ] Google Cloud Run setups
- [ ] Azure Container Instances
- [ ] Resource tagging and IAM roles

### ☸️ Kubernetes Support
- [ ] Deployment manifests
- [ ] Service and Ingress configs
- [ ] Helm chart generation
- [ ] ConfigMaps and Secrets

---

## 📅 Planned Features

### 🔗 CI/CD Pipeline Generation
- [ ] **GitHub Actions** — Build, test, deploy workflows
- [ ] **GitLab CI** — Pipeline configurations
- [ ] **Jenkins** — Jenkinsfile generation

### ☁️ Cloud Platform Integration
- [ ] **AWS** — ECS, Lambda, RDS, S3
- [ ] **Google Cloud** — Cloud Run, GKE, Cloud SQL
- [ ] **Azure** — Container Instances, AKS, Azure DB

### 📊 Monitoring & Observability
- [ ] Prometheus configuration
- [ ] Grafana dashboard templates
- [ ] OpenTelemetry setup
- [ ] Log aggregation (ELK, Fluentd)

### 🎨 Interactive Features
- [ ] Configuration wizard
- [ ] Dependency graph visualization
- [ ] Architecture diagram generation
- [ ] Watch mode with auto-regeneration

### 🔒 Advanced Security
- [ ] Security header configuration
- [ ] Network policy generation
- [ ] SOC 2 / GDPR / HIPAA templates
- [ ] Runtime security monitoring setup

---

## 🌟 Future Vision

### 🚀 Syncable Platform Integration
The CLI is the foundation for the **Syncable Platform** at [syncable.dev](https://syncable.dev):
- One-click cloud deployments
- Team collaboration & shared environments
- Cost optimization recommendations
- Integrated monitoring & logs

### 🧠 AI Enhancements
- Local LLM support (Ollama) for offline generation
- Custom model fine-tuning for IaC
- Predictive resource scaling
- Automated performance optimization

### 🔮 Emerging Technologies
- WebAssembly (WASM) deployments
- Edge computing configurations
- Serverless container platforms

---

## 📈 Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Framework Detection | 260+ | ✅ 260+ |
| Secret Patterns | 260+ | ✅ 260+ |
| Analysis Speed (1000 files) | <5s | ✅ ~2s |
| Security Scan Speed | 10-100x faster | ✅ Achieved |
| AI Providers | 2+ | ✅ 2 (OpenAI, Anthropic) |

---

*This roadmap is updated as we ship features. Star the repo to stay updated!*

[![GitHub stars](https://img.shields.io/github/stars/syncable-dev/syncable-cli?style=social)](https://github.com/syncable-dev/syncable-cli)
