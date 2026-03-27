# Syncable CLI Skills Design

## Context

Syncable CLI (`sync-ctl`) is a Rust-based DevOps toolbox with 260+ language/framework detection, security scanning, vulnerability checking, Dockerfile/Compose/K8s linting (native Rust implementations of hadolint, dclint, kubelint, helmlint), Kubernetes optimization, and Syncable platform integration.

Rather than competing with agent frameworks or building an MCP server, we're pivoting to a **skills-first approach**: the CLI stays a pure toolbox, and we ship Skills that teach any agent (Claude Code, Cursor, etc.) when and how to shell out to `sync-ctl`.

### Why Skills Over MCP

- No server to host, maintain, or pay for
- No auth layer needed
- Full local file access — the CLI reads the repo directly
- No latency shipping file contents over HTTP
- Works offline
- Simpler for users — install CLI + skills, done

## Architecture

### Layered Skill Model

Two layers:

1. **Command skills** — one per CLI command, the atomic building blocks. Each skill is the single source of truth for its command's syntax, flags, output interpretation, and error handling.
2. **Workflow skills** — orchestrate command skills for common multi-step patterns. They define sequencing, decision logic, and short-circuit conditions.

The agent can invoke command skills individually for granular tasks, or invoke workflow skills for end-to-end patterns. Workflow skills reference command skills by name.

### Directory Structure

```
skills/
├── commands/
│   ├── syncable-analyze.md
│   ├── syncable-security.md
│   ├── syncable-vulnerabilities.md
│   ├── syncable-dependencies.md
│   ├── syncable-validate.md
│   ├── syncable-optimize.md
│   └── syncable-platform.md
│
└── workflows/
    ├── syncable-project-assessment.md
    ├── syncable-security-audit.md
    ├── syncable-iac-pipeline.md
    └── syncable-deploy-pipeline.md
```

### Skill File Format

Every skill follows the Claude Code skill format:

```markdown
---
name: syncable-<name>
description: <trigger description>
---

## Purpose
## Prerequisites
## Commands
## Output Interpretation
## Error Handling
## Examples
```

Workflow skills add:

```markdown
## Workflow Steps
## Decision Points
```

## Command Skills (7)

### syncable-analyze

- **Triggers:** analyze project stack, detect languages/frameworks/runtimes/dependencies, "what is this project?"
- **Description:** `Use when analyzing a project's tech stack, detecting languages, frameworks, runtimes, or dependencies using Syncable CLI. Trigger on: "what stack is this", "analyze this project", "detect frameworks".`
- **Command:** `sync-ctl analyze <PATH> --json`
- **Key flags:** `--json`, `--detailed`, `--display {matrix|detailed|summary}`, `--only <filters>`
- **Output:** languages, frameworks, dependencies, runtimes, monorepo structure, Docker presence
- **Role:** Foundation skill — almost every workflow starts here.

### syncable-security

- **Triggers:** secret scanning, credential leaks, code security patterns, "is this project secure?"
- **Description:** `Use when scanning code for secrets, credentials, API keys, or insecure code patterns using Syncable CLI. Trigger on: "scan for secrets", "find leaked credentials", "security scan".`
- **Command:** `sync-ctl security <PATH> --mode <MODE> --format json`
- **Key flags:** `--mode {lightning|fast|balanced|thorough|paranoid}`, `--include-low`, `--no-secrets`, `--no-code-patterns`, `--fail-on-findings`
- **Output:** findings with severity, file locations, remediation steps, security score
- **Mode selection guidance:**
  - `lightning` — quick check, critical files only (.env, configs)
  - `fast` — smart sampling with priority patterns
  - `balanced` — recommended default
  - `thorough` — comprehensive, pre-deploy
  - `paranoid` — compliance/audit scenarios

### syncable-vulnerabilities

- **Triggers:** CVE scanning, dependency vulnerabilities, "are my dependencies safe?"
- **Description:** `Use when checking project dependencies for known CVEs or security vulnerabilities using Syncable CLI. Trigger on: "check for CVEs", "vulnerable dependencies", "dependency security".`
- **Command:** `sync-ctl vulnerabilities <PATH> --format json`
- **Key flags:** `--severity {low|medium|high|critical}`, `--output <FILE>`
- **Output:** CVEs per dependency, severity, affected versions, fix versions
- **Note:** Supports npm, pip, cargo, go, java ecosystems.

### syncable-dependencies

- **Triggers:** license audit, dependency listing, "what dependencies do I have?"
- **Description:** `Use when auditing project dependencies for licenses, production/dev split, or detailed dependency analysis using Syncable CLI. Trigger on: "license audit", "list dependencies", "dependency analysis".`
- **Command:** `sync-ctl dependencies <PATH> --licenses --format json`
- **Key flags:** `--licenses`, `--vulnerabilities`, `--prod-only`, `--dev-only`
- **Output:** dependency tree, license types, prod vs dev split

### syncable-validate

- **Triggers:** lint Dockerfiles, validate Compose, check Terraform, "is my IaC correct?"
- **Description:** `Use when linting or validating Dockerfiles, Docker Compose files, Terraform configs, or Kubernetes manifests using Syncable CLI. Trigger on: "lint Dockerfile", "validate compose", "check terraform".`
- **Command:** `sync-ctl validate <PATH> --types <TYPES>`
- **Key flags:** `--types {comma-separated}`, `--fix`
- **Output:** lint violations, best practice issues, auto-fix suggestions
- **Note:** Covers hadolint (Dockerfiles), dclint (Docker Compose), Terraform validation in one command.

### syncable-optimize

- **Triggers:** Kubernetes optimization, resource right-sizing, cost analysis, drift detection
- **Description:** `Use when optimizing Kubernetes resource requests/limits, analyzing costs, or detecting configuration drift using Syncable CLI. Trigger on: "optimize k8s", "right-size pods", "k8s cost analysis".`
- **Command:** `sync-ctl optimize <PATH>` with various flags
- **Key flags:** `--cluster`, `--prometheus <URL>`, `--namespace`, `--full`, `--cloud-provider {aws|gcp|azure|onprem}`, `--fix`, `--apply` (requires `--fix`), `--dry-run`
- **Output:** recommendations, cost savings estimates, drift detection results
- **Important:** `--fix` generates fix suggestions only. `--apply` (which requires `--fix`) actually writes changes to manifest files. The agent must never use `--apply` without explicit user confirmation.
- **Note:** `--full` adds kubelint security checks + helmlint validation.

### syncable-platform

- **Triggers:** Syncable authentication, project/org/env management, deployment
- **Description:** `Use when authenticating with Syncable, managing projects/orgs/environments, or deploying services through the Syncable platform. Trigger on: "syncable login", "select project", "deploy to syncable".`
- **Commands:**
  - `sync-ctl auth login|logout|status|token`
  - `sync-ctl org list|select <ID>`
  - `sync-ctl project list|select <ID>|current|info`
  - `sync-ctl env list|select <ID>`
  - `sync-ctl deploy wizard|new-env|status <TASK_ID>`
- **Note:** Auth-first flow — skill checks auth status before any platform operation.

## Workflow Skills (4)

### syncable-project-assessment

- **Triggers:** "assess this project", "full health check", "what's the state of this codebase?", onboarding to a new repo
- **Description:** `Use when a user wants a comprehensive project health check — combines stack analysis, security scanning, vulnerability checks, and dependency auditing via Syncable CLI. Trigger on: "assess this project", "full health check", "project overview".`
- **Steps:**
  1. `syncable-analyze` → understand the stack
  2. `syncable-security` (balanced mode) → scan for secrets and code patterns
  3. `syncable-vulnerabilities` → check dependency CVEs
  4. `syncable-dependencies` → license audit
- **Decision logic:**
  - If analyze shows no dependencies → skip vulnerabilities and dependencies steps
  - If no secrets-capable files found → use lightning mode for security
- **Output:** Agent synthesizes a unified project health report.

### syncable-security-audit

- **Triggers:** "run a full security audit", "is this production-ready?", pre-merge security review
- **Description:** `Use when performing a thorough pre-deployment or compliance security review — combines deep security scan, CVE checks, and IaC validation via Syncable CLI. Trigger on: "security audit", "is this production-ready", "pre-deploy security check".`
- **Steps:**
  1. `syncable-analyze` → understand what we're scanning
  2. `syncable-security` (thorough or paranoid mode) → deep secret + pattern scan
  3. `syncable-vulnerabilities` → CVE scan
  4. `syncable-validate` → lint IaC files if any exist
- **Decision logic:**
  - If analyze detects Docker/K8s/Terraform files → include validate step, else skip
  - Severity threshold driven by context: PR review = thorough, pre-deploy = paranoid

### syncable-iac-pipeline

- **Triggers:** "validate my infrastructure", "check my Dockerfiles and K8s manifests", "lint all IaC"
- **Description:** `Use when validating all infrastructure-as-code files in a project — combines IaC linting with Kubernetes optimization and security checks via Syncable CLI. Trigger on: "validate infrastructure", "lint all IaC", "check my k8s and docker files".`
- **Steps:**
  1. `syncable-analyze` → detect which IaC types exist
  2. `syncable-validate` → lint all detected types
  3. `syncable-optimize` (with `--full` if K8s manifests found) → kubelint + helmlint + resource optimization
- **Decision logic:**
  - Only run optimize if K8s manifests detected by analyze

### syncable-deploy-pipeline

- **Triggers:** "deploy this project", "set up deployment", "push to Syncable"
- **Description:** `Use when deploying a project through Syncable — orchestrates auth, analysis, security gating, and deployment via Syncable CLI. Trigger on: "deploy this", "push to syncable", "set up deployment".`
- **Steps:**
  1. `syncable-platform` → check auth status, ensure project/org/env selected
  2. `syncable-analyze` → understand the project
  3. `syncable-security-audit` (workflow) → pre-deploy security gate
  4. `syncable-platform` → trigger deployment
- **Decision logic:**
  - If auth missing → guide through login first
  - If security audit finds critical findings → warn user, require confirmation before proceeding
  - Never deploy silently

## Installation Model

Skills ship inside the `syncable-cli` repo under `skills/`. A future `npx syncable-skills install` command will:

1. Copy skill files from `skills/` into the agent's skill directory (e.g., `~/.claude/skills/` for Claude Code)
2. Verify `sync-ctl` is available on PATH
3. Optionally add a CLAUDE.md entry pointing to the skills

## Global Prerequisites

Every skill assumes:
- `sync-ctl` binary is installed and on PATH
- The agent has shell access (Bash tool or equivalent)
- The agent has access to the project directory being analyzed
- For platform skills: internet access for Syncable API
- All command skills default to `--json` or `--format json` output for machine-readable results the agent can parse

## Design Decisions

1. **No generate skills for now** — `generate iac` and `generate ci` are excluded from skills. They stay in the CLI but aren't surfaced to agents yet.
2. **Platform commands consolidated** — auth/project/org/env/deploy are one skill (`syncable-platform`) because they're always used together in a flow.
3. **JSON-first output** — all command skills instruct the agent to use `--json` or `--format json` so the agent can parse structured output, not terminal tables. Note: `analyze` uses a `--json` boolean flag while other commands use `--format json` via the OutputFormat enum.
4. **Analyze is the foundation** — most workflows start with analyze. The skill explicitly tells the agent to run it first if it hasn't been run in the current session.
5. **Security mode guidance** — the security skill provides explicit guidance on which scan mode to use based on context, rather than leaving the agent to guess.
6. **Global flags** — all commands support `--verbose` (`-v`/`-vv`/`-vvv`), `--quiet`, and `--config <FILE>` at the CLI level. JSON output is command-specific: `analyze` uses a `--json` boolean flag, while `security`, `vulnerabilities`, `dependencies`, and `optimize` use `--format json` via the OutputFormat enum. Each skill must document the correct JSON flag for its command.
7. **Dependencies --vulnerabilities vs standalone vulnerabilities** — `dependencies --vulnerabilities` gives a quick inline check alongside license info; the standalone `vulnerabilities` command is a deeper dedicated CVE scan. Skills should guide agents to use the standalone command for thorough checks and the flag for quick inline context.

## CLI Branch State

All commands and flags referenced in this spec exist on both `main` and `develop` branches. The `develop` branch is ahead of `main` with additional features (agent improvements, framework detection, etc.) but the skills surface area is fully available on `main`. This skills branch can target either branch.

## Excluded Commands

The following CLI commands are intentionally excluded from the skills surface:

- **`generate`** — IaC and CI generation are excluded for now (design decision #1).
- **`support`** — lists supported technologies. Low agent utility; agents can infer support from `analyze` results.
- **`tools`** — manages vulnerability scanning tool installation. Rather than a dedicated skill, `syncable-vulnerabilities` includes error handling guidance that directs agents to run `sync-ctl tools install` when a scanner is missing.
- **`chat` / `agent`** — the AI agent and AG-UI server. Excluded because the entire point of this skills approach is to replace agent-in-agent with direct CLI toolbox usage.

## Workflow Composition

Workflow skills can reference other workflow skills as steps (e.g., `syncable-deploy-pipeline` calls `syncable-security-audit`). When a workflow references another workflow, the inner workflow's full step sequence and decision logic is executed inline — the agent treats it as expanding the inner workflow's steps into the outer workflow at that position.

## Security Mode Default

The CLI defaults `--mode` to `thorough`. Skills should always pass `--mode` explicitly rather than relying on the CLI default, since the skill's recommended mode may differ from the CLI default (e.g., `balanced` for project assessments, `paranoid` for compliance audits).
