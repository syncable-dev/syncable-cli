# Syncable CLI Skills Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build 7 command skills and 4 workflow skills that teach AI agents when and how to use the Syncable CLI (`sync-ctl`) as a toolbox.

**Architecture:** Layered skill model — command skills are atomic wrappers around individual CLI commands, workflow skills orchestrate command skills for multi-step patterns. All skills are markdown files following the Claude Code skill format with frontmatter, structured sections, and concrete examples.

**Tech Stack:** Markdown (Claude Code skill format), Bash (CLI invocations in examples)

**Spec:** `docs/superpowers/specs/2026-03-27-syncable-cli-skills-design.md`

---

## File Structure

```
skills/
├── commands/
│   ├── syncable-analyze.md        # Project stack analysis
│   ├── syncable-security.md       # Secret/code pattern scanning
│   ├── syncable-vulnerabilities.md # CVE dependency scanning
│   ├── syncable-dependencies.md   # License/dependency audit
│   ├── syncable-validate.md       # IaC linting (hadolint/dclint/terraform)
│   ├── syncable-optimize.md       # K8s optimization & cost analysis
│   └── syncable-platform.md       # Auth/project/org/env/deploy
│
└── workflows/
    ├── syncable-project-assessment.md  # Full health check
    ├── syncable-security-audit.md      # Deep security review
    ├── syncable-iac-pipeline.md        # IaC validation pipeline
    └── syncable-deploy-pipeline.md     # End-to-end deploy
```

Each file is a self-contained Claude Code skill (markdown with YAML frontmatter). No code, no tests — these are pure prompt/documentation files.

---

### Task 1: Create directory structure

**Files:**
- Create: `skills/commands/` (directory)
- Create: `skills/workflows/` (directory)

- [ ] **Step 1: Create the skills directories**

```bash
mkdir -p skills/commands skills/workflows
```

- [ ] **Step 2: Commit**

```bash
git add skills/
git commit -m "chore: scaffold skills directory structure"
```

---

### Task 2: Write `syncable-analyze` command skill

**Files:**
- Create: `skills/commands/syncable-analyze.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-analyze
description: Use when analyzing a project's tech stack, detecting languages, frameworks, runtimes, or dependencies using Syncable CLI. Trigger on: "what stack is this", "analyze this project", "detect frameworks", "what languages does this use".
---

## Purpose

Analyze a project directory to detect its tech stack: programming languages, frameworks, runtimes, package managers, dependencies, Docker presence, and monorepo structure. This is the foundation skill — most workflows start here to understand what they're working with.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Basic analysis (JSON output for agent consumption)

```bash
sync-ctl analyze <PATH> --json
```

### Human-readable matrix view

```bash
sync-ctl analyze <PATH> --display matrix
```

### Filtered analysis (only specific aspects)

```bash
sync-ctl analyze <PATH> --json --only languages,frameworks
sync-ctl analyze <PATH> --json --only dependencies
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--json` | Machine-readable JSON output (always use when processing results) |
| `--detailed` | Show detailed analysis (legacy vertical format) |
| `--display {matrix\|detailed\|summary}` | Display format for human-readable output |
| `--only <filters>` | Comma-separated: `languages`, `frameworks`, `dependencies` |

## Output Interpretation

The JSON output contains:

- **languages** — detected programming languages with file counts and percentages
- **frameworks** — detected frameworks with versions where available
- **dependencies** — package managers found and dependency counts
- **runtimes** — detected runtime versions (Node.js, Python, Go, Rust, Java)
- **docker** — whether Dockerfiles or Docker Compose files exist
- **monorepo** — whether the project is a monorepo and its structure

When reporting to the user, prioritize: primary language, main framework, runtime version, and whether Docker/K8s infrastructure exists.

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No such file or directory` | Invalid path | Ask user to verify the project path |
| Empty output | No recognizable project files | Tell user the directory may not contain a supported project. Run `sync-ctl support` to show supported technologies |
| Timeout | Very large monorepo | Try `--only languages` for a faster partial scan |

## Examples

**Analyze current directory:**
```bash
sync-ctl analyze . --json
```

**Analyze a specific project:**
```bash
sync-ctl analyze /path/to/project --json
```

**Quick language-only check:**
```bash
sync-ctl analyze . --json --only languages
```
```

- [ ] **Step 2: Verify the file is valid markdown with correct frontmatter**

Run: `head -5 skills/commands/syncable-analyze.md`
Expected: YAML frontmatter with `---`, `name:`, `description:`, `---`

- [ ] **Step 3: Commit**

```bash
git add skills/commands/syncable-analyze.md
git commit -m "feat(skills): add syncable-analyze command skill"
```

---

### Task 3: Write `syncable-security` command skill

**Files:**
- Create: `skills/commands/syncable-security.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-security
description: Use when scanning code for secrets, credentials, API keys, or insecure code patterns using Syncable CLI. Trigger on: "scan for secrets", "find leaked credentials", "security scan", "is this code secure", "check for hardcoded passwords".
---

## Purpose

Perform security analysis on a codebase: detect leaked secrets (API keys, tokens, passwords, private keys), identify insecure code patterns, and analyze configuration security. Returns findings with severity levels, file locations, and remediation guidance.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Standard security scan

```bash
sync-ctl security <PATH> --mode balanced --format json
```

### Mode Selection Guide

Always pass `--mode` explicitly. Choose based on context:

| Mode | When to use | Speed |
|------|------------|-------|
| `lightning` | Quick check, only critical files (.env, configs) | Fastest |
| `fast` | Smart sampling, good for large repos during development | Fast |
| `balanced` | **Default choice.** Good coverage with optimizations | Medium |
| `thorough` | Pre-deployment reviews, PR security checks | Slow |
| `paranoid` | Compliance audits, production security reviews | Slowest |

### Key Flags

| Flag | Purpose |
|------|---------|
| `--mode {lightning\|fast\|balanced\|thorough\|paranoid}` | Scan depth (always specify) |
| `--format json` | Machine-readable output (always use when processing results) |
| `--include-low` | Include low-severity findings (off by default) |
| `--no-secrets` | Skip secrets detection (only code patterns) |
| `--no-code-patterns` | Skip code pattern analysis (only secrets) |
| `--fail-on-findings` | Exit with error code if findings exist (for CI) |
| `--output <FILE>` | Write report to file |

## Output Interpretation

The JSON output contains:

- **findings** — array of security issues, each with:
  - `severity` — Critical, High, Medium, Low, Info
  - `category` — secrets, code_pattern, configuration, infrastructure
  - `file` — exact file path
  - `line` — line number
  - `description` — what was found
  - `remediation` — how to fix it
- **summary** — total counts by severity
- **score** — overall security score (0-100)

**Priority for reporting to user:**
1. Critical findings first (leaked secrets, hardcoded credentials)
2. High findings (insecure patterns)
3. Summary with score
4. Remediation steps for top findings

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No such file or directory` | Invalid path | Ask user to verify the project path |
| Very slow scan | Large repo with `thorough`/`paranoid` mode | Suggest trying `balanced` or `fast` mode first |
| No findings | Clean project or scan mode too light | If `lightning`/`fast`, suggest re-running with `balanced` for deeper coverage |

## Examples

**Quick secrets check on current directory:**
```bash
sync-ctl security . --mode balanced --format json
```

**Deep pre-deploy audit:**
```bash
sync-ctl security . --mode paranoid --format json
```

**Secrets-only scan (skip code patterns):**
```bash
sync-ctl security . --mode thorough --no-code-patterns --format json
```

**Save report to file:**
```bash
sync-ctl security . --mode thorough --format json --output security-report.json
```
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/commands/syncable-security.md`

- [ ] **Step 3: Commit**

```bash
git add skills/commands/syncable-security.md
git commit -m "feat(skills): add syncable-security command skill"
```

---

### Task 4: Write `syncable-vulnerabilities` command skill

**Files:**
- Create: `skills/commands/syncable-vulnerabilities.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-vulnerabilities
description: Use when checking project dependencies for known CVEs or security vulnerabilities using Syncable CLI. Trigger on: "check for CVEs", "vulnerable dependencies", "dependency security", "are my packages safe", "npm audit", "cargo audit".
---

## Purpose

Scan project dependencies for known CVEs (Common Vulnerabilities and Exposures) across npm, pip, cargo, go, and java ecosystems. Returns vulnerable packages with severity, affected versions, and available fixes.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory
- Language-specific scanning tools should be installed. If a scan fails with "tool not found", run `sync-ctl tools install` to install missing scanners.

## Commands

### Scan for vulnerabilities

```bash
sync-ctl vulnerabilities <PATH> --format json
```

### Filter by severity

```bash
sync-ctl vulnerabilities <PATH> --severity high --format json
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--format json` | Machine-readable output (always use) |
| `--severity {low\|medium\|high\|critical}` | Only show findings at or above this severity |
| `--output <FILE>` | Write report to file |

## Output Interpretation

The JSON output contains an array of vulnerability findings, each with:

- **package** — affected dependency name
- **version** — installed version
- **severity** — Critical, High, Medium, Low
- **cve** — CVE identifier (e.g., CVE-2024-1234)
- **description** — what the vulnerability is
- **fix_version** — version that resolves it (if available)
- **ecosystem** — npm, pip, cargo, go, java

**Priority for reporting to user:**
1. Critical/High CVEs with available fixes — actionable immediately
2. Critical/High CVEs without fixes — flag as risk
3. Medium/Low — mention count, don't enumerate unless asked

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `tool not found` or scanner missing | Language-specific audit tool not installed | Run `sync-ctl tools install` to install missing scanners, then retry |
| `No dependencies found` | No package manager files detected | Run `sync-ctl analyze <PATH> --json` first to verify the project has dependencies |
| Timeout | Very large dependency tree | Try scanning specific subdirectories in a monorepo |

## Examples

**Scan current project:**
```bash
sync-ctl vulnerabilities . --format json
```

**Only critical and high severity:**
```bash
sync-ctl vulnerabilities . --severity high --format json
```

**Save report:**
```bash
sync-ctl vulnerabilities . --format json --output vuln-report.json
```

**Install missing scanners first:**
```bash
sync-ctl tools install --yes
sync-ctl vulnerabilities . --format json
```
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/commands/syncable-vulnerabilities.md`

- [ ] **Step 3: Commit**

```bash
git add skills/commands/syncable-vulnerabilities.md
git commit -m "feat(skills): add syncable-vulnerabilities command skill"
```

---

### Task 5: Write `syncable-dependencies` command skill

**Files:**
- Create: `skills/commands/syncable-dependencies.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-dependencies
description: Use when auditing project dependencies for licenses, production/dev split, or detailed dependency analysis using Syncable CLI. Trigger on: "license audit", "list dependencies", "dependency analysis", "what licenses am I using", "show me all packages".
---

## Purpose

Analyze project dependencies in detail: list all packages, check license types, separate production from development dependencies, and optionally flag vulnerabilities inline. Use this for license compliance and dependency inventory.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Full dependency analysis with licenses

```bash
sync-ctl dependencies <PATH> --licenses --format json
```

### Production dependencies only

```bash
sync-ctl dependencies <PATH> --licenses --prod-only --format json
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--format json` | Machine-readable output (always use) |
| `--licenses` | Include license information for each dependency |
| `--vulnerabilities` | Quick inline vulnerability check (for thorough CVE scanning, use the standalone `sync-ctl vulnerabilities` command instead) |
| `--prod-only` | Show only production dependencies |
| `--dev-only` | Show only development dependencies |

## Output Interpretation

The JSON output contains:

- **dependencies** — array of packages with name, version, license, and prod/dev classification
- **summary** — total counts, license distribution

**Priority for reporting to user:**
1. License concerns (copyleft in commercial projects, unknown licenses)
2. Dependency counts (prod vs dev)
3. Specific packages only if asked

**When to use `--vulnerabilities` vs standalone `vulnerabilities` command:**
- Use `--vulnerabilities` here for a quick inline check alongside license info
- Use `sync-ctl vulnerabilities` for a dedicated, thorough CVE scan

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No dependencies found` | No package manager files | Verify project path, run `sync-ctl analyze` to check for supported package managers |
| Incomplete results | Some package managers not fully parsed | Note which ecosystems were scanned and which may be missing |

## Examples

**Full audit with licenses:**
```bash
sync-ctl dependencies . --licenses --format json
```

**Production-only for license compliance:**
```bash
sync-ctl dependencies . --licenses --prod-only --format json
```

**Quick vulnerability check alongside deps:**
```bash
sync-ctl dependencies . --licenses --vulnerabilities --format json
```
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/commands/syncable-dependencies.md`

- [ ] **Step 3: Commit**

```bash
git add skills/commands/syncable-dependencies.md
git commit -m "feat(skills): add syncable-dependencies command skill"
```

---

### Task 6: Write `syncable-validate` command skill

**Files:**
- Create: `skills/commands/syncable-validate.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-validate
description: Use when linting or validating Dockerfiles, Docker Compose files, Terraform configs, or Kubernetes manifests using Syncable CLI. Trigger on: "lint Dockerfile", "validate compose", "check terraform", "is my IaC correct", "lint my infrastructure files".
---

## Purpose

Validate Infrastructure-as-Code files against best practices. Covers Dockerfiles (via native hadolint), Docker Compose files (via native dclint), and Terraform configurations. Reports violations with severity, file locations, and auto-fix suggestions.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory
- Project must contain IaC files (Dockerfiles, docker-compose.yml, *.tf files)

## Commands

### Validate all IaC files in a directory

```bash
sync-ctl validate <PATH>
```

### Validate specific types only

```bash
sync-ctl validate <PATH> --types dockerfile
sync-ctl validate <PATH> --types dockerfile,compose
sync-ctl validate <PATH> --types terraform
```

### Auto-fix issues where possible

```bash
sync-ctl validate <PATH> --fix
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--types <comma-separated>` | Filter to specific IaC types: `dockerfile`, `compose`, `terraform` |
| `--fix` | Automatically fix issues where possible |

**Note:** The `validate` command does not support `--format json`. Output is terminal text with structured lint violations. When processing results, parse the text output rather than expecting JSON.

## Output Interpretation

Output contains lint violations (as terminal text, not JSON), each with:

- **file** — path to the IaC file
- **line** — line number of the violation
- **rule** — rule ID (e.g., DL3006 for hadolint, DCL001 for dclint)
- **severity** — Error, Warning, Info
- **message** — description of the issue
- **fix** — suggested fix (if available)

**What gets checked:**

| Type | Linter | Example checks |
|------|--------|---------------|
| Dockerfile | hadolint (native Rust) | Pin versions, avoid `latest` tag, use COPY not ADD, multi-stage best practices |
| Docker Compose | dclint (native Rust) | Service naming, volume declarations, network configuration, 15 configurable rules |
| Terraform | Terraform validator | Syntax validation, provider configuration, resource definitions |

**Priority for reporting to user:**
1. Errors first — these will cause build/deploy failures
2. Warnings — best practice violations
3. Info — suggestions for improvement

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No IaC files found` | Directory has no Dockerfiles, Compose, or Terraform files | Run `sync-ctl analyze <PATH> --json` to verify what IaC exists |
| `Unknown type` | Invalid `--types` value | Valid types are: `dockerfile`, `compose`, `terraform` |

## Examples

**Lint all IaC in current directory:**
```bash
sync-ctl validate .
```

**Lint only Dockerfiles:**
```bash
sync-ctl validate . --types dockerfile
```

**Auto-fix Docker Compose issues:**
```bash
sync-ctl validate . --types compose --fix
```
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/commands/syncable-validate.md`

- [ ] **Step 3: Commit**

```bash
git add skills/commands/syncable-validate.md
git commit -m "feat(skills): add syncable-validate command skill"
```

---

### Task 7: Write `syncable-optimize` command skill

**Files:**
- Create: `skills/commands/syncable-optimize.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-optimize
description: Use when optimizing Kubernetes resource requests/limits, analyzing costs, or detecting configuration drift using Syncable CLI. Trigger on: "optimize k8s", "right-size pods", "k8s cost analysis", "resource recommendations", "over-provisioned containers".
---

## Purpose

Analyze Kubernetes manifests and optionally live cluster metrics to recommend resource right-sizing, estimate costs, and detect configuration drift. Can also run kubelint security checks and helmlint validation with `--full` flag.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- For static analysis: K8s manifest files (YAML) in the project
- For live cluster analysis: valid kubeconfig with cluster access
- For cost estimation: `--cloud-provider` flag

## Commands

### Static manifest analysis

```bash
sync-ctl optimize <PATH> --format json
```

### Live cluster analysis

```bash
sync-ctl optimize <PATH> --cluster --format json
sync-ctl optimize <PATH> --cluster my-context --namespace default --format json
```

### With Prometheus metrics

```bash
sync-ctl optimize <PATH> --cluster --prometheus http://localhost:9090 --period 30d --format json
```

### Full analysis (includes kubelint + helmlint)

```bash
sync-ctl optimize <PATH> --full --format json
```

### Cost estimation

```bash
sync-ctl optimize <PATH> --cluster --cloud-provider aws --region us-east-1 --format json
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--format json` | Machine-readable output (always use) |
| `--cluster [CONTEXT]` | Connect to live K8s cluster (uses current context if no name given) |
| `--prometheus <URL>` | Prometheus URL for historical metrics |
| `--namespace <NS>` | Target namespace (or `*` for all) |
| `--period <DURATION>` | Analysis period for metrics (e.g., `7d`, `30d`) |
| `--full` | Include kubelint security checks + helmlint validation |
| `--cloud-provider {aws\|gcp\|azure\|onprem}` | Cloud provider for cost estimation |
| `--region <REGION>` | Region for pricing (default: `us-east-1`) |
| `--fix` | Generate fix suggestions |
| `--apply` | **DANGEROUS:** Apply fixes to manifest files. Requires `--fix`. Never use without explicit user confirmation. |
| `--dry-run` | Preview changes without applying |
| `--severity <LEVEL>` | Minimum severity to report |
| `--threshold <0-100>` | Minimum waste percentage threshold |

## Output Interpretation

The JSON output contains:

- **recommendations** — array of optimization suggestions with:
  - Resource right-sizing (CPU/memory requests/limits)
  - Confidence score
  - Current vs recommended values
  - Estimated savings
- **costs** — cost attribution per workload (if `--cloud-provider` set)
- **drift** — configuration drift between manifests and running state (if `--cluster` set)
- **security** — kubelint findings (if `--full` set)

**Priority for reporting to user:**
1. High-confidence right-sizing recommendations with cost savings
2. Critical security findings (from `--full`)
3. Drift detection issues
4. Cost breakdown summary

## Safety

- `--fix` only generates suggestions — it does NOT modify files
- `--apply` (requires `--fix`) writes changes to files — always confirm with user first
- `--dry-run` previews what `--apply` would do — use this to show the user before applying
- Recommend: always run `--fix --dry-run` first, show output, then `--fix --apply` only after user approval

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No Kubernetes manifests found` | No YAML with K8s resources | Run `sync-ctl analyze <PATH> --json` to check for K8s presence |
| `Cannot connect to cluster` | Invalid kubeconfig or cluster unreachable | Check `kubectl cluster-info` works, verify context name |
| `Prometheus unreachable` | Wrong URL or Prometheus not running | Verify URL, fall back to static analysis without `--prometheus` |

## Examples

**Quick static analysis:**
```bash
sync-ctl optimize . --format json
```

**Full analysis with live cluster and cost estimation:**
```bash
sync-ctl optimize . --cluster --cloud-provider aws --full --format json
```

**Preview fixes before applying:**
```bash
sync-ctl optimize . --fix --dry-run --format json
```
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/commands/syncable-optimize.md`

- [ ] **Step 3: Commit**

```bash
git add skills/commands/syncable-optimize.md
git commit -m "feat(skills): add syncable-optimize command skill"
```

---

### Task 8: Write `syncable-platform` command skill

**Files:**
- Create: `skills/commands/syncable-platform.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-platform
description: Use when authenticating with Syncable, managing projects/orgs/environments, or deploying services through the Syncable platform. Trigger on: "syncable login", "select project", "deploy to syncable", "list environments", "switch organization".
---

## Purpose

Manage Syncable platform operations: authenticate, select organizations/projects/environments, and trigger deployments. This skill covers the full platform lifecycle from login to deploy.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Internet access for Syncable API
- For deployment: authenticated session with project and environment selected

## Commands

### Authentication

```bash
# Check if authenticated
sync-ctl auth status

# Log in (opens browser)
sync-ctl auth login

# Log in without auto-opening browser
sync-ctl auth login --no-browser

# Log out
sync-ctl auth logout

# Get access token (for scripting)
sync-ctl auth token --raw
```

### Organization Management

```bash
# List organizations
sync-ctl org list

# Select an organization
sync-ctl org select <ORG_ID>
```

### Project Management

```bash
# List projects in current org
sync-ctl project list

# List projects in a specific org
sync-ctl project list --org-id <ORG_ID>

# Select a project
sync-ctl project select <PROJECT_ID>

# Show current context (org + project)
sync-ctl project current

# Show project details
sync-ctl project info
sync-ctl project info <PROJECT_ID>
```

### Environment Management

```bash
# List environments in current project
sync-ctl env list

# Select an environment
sync-ctl env select <ENV_ID>
```

### Deployment

```bash
# Launch interactive deployment wizard
sync-ctl deploy wizard

# Create a new environment
sync-ctl deploy new-env

# Check deployment status
sync-ctl deploy status <TASK_ID>

# Watch deployment status until complete
sync-ctl deploy status <TASK_ID> --watch
```

## Auth-First Flow

**Always check auth status before any platform operation.** Follow this sequence:

1. Run `sync-ctl auth status` — if not authenticated, guide user through `sync-ctl auth login`
2. Run `sync-ctl project current` — if no project selected, list projects and ask user to select
3. For deployment: ensure environment is selected via `sync-ctl env list` + `sync-ctl env select`

## Output Interpretation

- **auth status** — shows whether user is logged in, token expiry
- **org/project/env list** — shows available items with IDs and names
- **project current** — shows currently selected org, project, and environment
- **deploy status** — shows deployment progress, logs, and final status

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `Not authenticated` | No valid session | Run `sync-ctl auth login` |
| `Token expired` | Session timed out | Run `sync-ctl auth login` to re-authenticate |
| `No project selected` | Project context not set | Run `sync-ctl project list` then `sync-ctl project select <ID>` |
| `No environment selected` | Environment not set | Run `sync-ctl env list` then `sync-ctl env select <ID>` |
| `Deployment failed` | Build or infra error | Check `sync-ctl deploy status <TASK_ID>` for error details |

## Examples

**Full login-to-deploy flow:**
```bash
sync-ctl auth login
sync-ctl org list
sync-ctl org select <ORG_ID>
sync-ctl project list
sync-ctl project select <PROJECT_ID>
sync-ctl env list
sync-ctl env select <ENV_ID>
sync-ctl deploy wizard
```

**Check current context:**
```bash
sync-ctl auth status
sync-ctl project current
```

**Monitor a deployment:**
```bash
sync-ctl deploy status <TASK_ID> --watch
```
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/commands/syncable-platform.md`

- [ ] **Step 3: Commit**

```bash
git add skills/commands/syncable-platform.md
git commit -m "feat(skills): add syncable-platform command skill"
```

---

### Task 9: Write `syncable-project-assessment` workflow skill

**Files:**
- Create: `skills/workflows/syncable-project-assessment.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-project-assessment
description: Use when a user wants a comprehensive project health check - combines stack analysis, security scanning, vulnerability checks, and dependency auditing via Syncable CLI. Trigger on: "assess this project", "full health check", "project overview", "what's the state of this codebase", "onboard me to this repo".
---

## Purpose

Run a comprehensive project health check by chaining multiple Syncable CLI commands. Produces a unified report covering tech stack, security posture, vulnerability status, and dependency health.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Workflow Steps

### Step 1: Analyze the project stack

```bash
sync-ctl analyze <PATH> --json
```

Parse the output to understand:
- What languages and frameworks are present
- Whether dependencies exist (needed for steps 3 and 4)
- Whether secrets-capable files exist (affects step 2 mode)

### Step 2: Security scan

```bash
sync-ctl security <PATH> --mode balanced --format json
```

**Decision point:** If step 1 shows no config files, secrets files, or environment files, use `--mode lightning` instead of `--mode balanced` to save time.

### Step 3: Vulnerability scan

```bash
sync-ctl vulnerabilities <PATH> --format json
```

**Decision point:** If step 1 detected no dependencies (no package.json, requirements.txt, Cargo.toml, go.mod, etc.), **skip this step entirely** and note "No dependencies detected" in the report.

### Step 4: Dependency audit

```bash
sync-ctl dependencies <PATH> --licenses --format json
```

**Decision point:** Same as step 3 — skip if no dependencies detected.

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| No dependencies detected in step 1 | Skip steps 3 and 4 |
| No secrets-capable files in step 1 | Use `--mode lightning` in step 2 |
| Vulnerability scanner missing | Run `sync-ctl tools install --yes`, then retry step 3 |

## Report Synthesis

After all steps complete, synthesize a unified report for the user:

1. **Tech Stack** — primary language, frameworks, runtimes
2. **Security Score** — score from security scan, critical/high finding count
3. **Vulnerabilities** — critical/high CVE count, packages with available fixes
4. **Dependencies** — total count, license concerns (copyleft, unknown)
5. **Recommendations** — top 3-5 actionable items prioritized by severity

## Examples

**Assess current directory:**

The agent runs these commands in sequence, skipping steps based on decision points:

```bash
sync-ctl analyze . --json
sync-ctl security . --mode balanced --format json
sync-ctl vulnerabilities . --format json
sync-ctl dependencies . --licenses --format json
```

Then synthesizes the results into a single report for the user.
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/workflows/syncable-project-assessment.md`

- [ ] **Step 3: Commit**

```bash
git add skills/workflows/syncable-project-assessment.md
git commit -m "feat(skills): add syncable-project-assessment workflow skill"
```

---

### Task 10: Write `syncable-security-audit` workflow skill

**Files:**
- Create: `skills/workflows/syncable-security-audit.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-security-audit
description: Use when performing a thorough pre-deployment or compliance security review - combines deep security scan, CVE checks, and IaC validation via Syncable CLI. Trigger on: "security audit", "is this production-ready", "pre-deploy security check", "compliance review", "full security review".
---

## Purpose

Perform a deep, multi-layered security review suitable for pre-deployment gates or compliance audits. Goes deeper than the project assessment by using thorough/paranoid scan modes and including IaC validation.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Workflow Steps

### Step 1: Analyze the project

```bash
sync-ctl analyze <PATH> --json
```

Parse the output to determine:
- What IaC files exist (Dockerfiles, Compose, Terraform, K8s manifests) — needed for step 4
- What dependencies exist — needed for step 3

### Step 2: Deep security scan

Choose mode based on context:

**For PR reviews / pre-merge:**
```bash
sync-ctl security <PATH> --mode thorough --format json
```

**For production deployment / compliance:**
```bash
sync-ctl security <PATH> --mode paranoid --format json
```

### Step 3: Vulnerability scan

```bash
sync-ctl vulnerabilities <PATH> --format json
```

### Step 4: IaC validation

**Decision point:** Only run if step 1 detected Docker, Compose, Terraform, or K8s files.

```bash
sync-ctl validate <PATH>
```

If specific types are known from step 1, filter:
```bash
sync-ctl validate <PATH> --types dockerfile,compose
```

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| PR review context | Use `--mode thorough` in step 2 |
| Pre-deploy / compliance context | Use `--mode paranoid` in step 2 |
| No IaC files detected in step 1 | Skip step 4 |
| No dependencies detected in step 1 | Skip step 3 |

## Report Synthesis

Produce a security audit report:

1. **Security Findings** — all Critical and High findings with file locations and remediation
2. **Vulnerability Status** — CVEs by severity, packages needing updates
3. **IaC Compliance** — lint violations in Dockerfiles, Compose, Terraform
4. **Overall Verdict** — PASS (no critical/high findings), WARN (high findings but no critical), FAIL (critical findings present)
5. **Remediation Priority** — ordered list of actions to resolve findings

**If critical findings exist:** Explicitly warn the user. If this audit is part of a deploy pipeline, recommend blocking deployment until critical issues are resolved.
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/workflows/syncable-security-audit.md`

- [ ] **Step 3: Commit**

```bash
git add skills/workflows/syncable-security-audit.md
git commit -m "feat(skills): add syncable-security-audit workflow skill"
```

---

### Task 11: Write `syncable-iac-pipeline` workflow skill

**Files:**
- Create: `skills/workflows/syncable-iac-pipeline.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-iac-pipeline
description: Use when validating all infrastructure-as-code files in a project - combines IaC linting with Kubernetes optimization and security checks via Syncable CLI. Trigger on: "validate infrastructure", "lint all IaC", "check my k8s and docker files", "infrastructure review".
---

## Purpose

Validate all infrastructure-as-code files in a project by chaining IaC linting with Kubernetes optimization and security checks. Covers Dockerfiles, Docker Compose, Terraform, K8s manifests, and Helm charts.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory
- Project must contain IaC files

## Workflow Steps

### Step 1: Analyze the project

```bash
sync-ctl analyze <PATH> --json
```

Parse the output to determine:
- Which IaC types exist (Dockerfile, Compose, Terraform, K8s manifests)
- Whether K8s manifests are present — needed for step 3

### Step 2: Validate IaC files

```bash
sync-ctl validate <PATH>
```

If step 1 revealed specific types, you can filter:
```bash
sync-ctl validate <PATH> --types dockerfile,compose,terraform
```

### Step 3: Kubernetes optimization (conditional)

**Decision point:** Only run if step 1 detected Kubernetes manifests or Helm charts.

```bash
sync-ctl optimize <PATH> --full --format json
```

The `--full` flag includes kubelint security checks and helmlint validation on top of resource optimization.

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| No K8s manifests or Helm charts in step 1 | Skip step 3 |
| No IaC files at all in step 1 | Abort workflow, tell user no IaC files found |

## Report Synthesis

Produce an IaC validation report:

1. **Dockerfile Issues** — hadolint violations by severity
2. **Docker Compose Issues** — dclint violations
3. **Terraform Issues** — validation errors
4. **Kubernetes Issues** — kubelint security findings and resource optimization recommendations (if step 3 ran)
5. **Actionable Fixes** — which issues can be auto-fixed with `--fix`
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/workflows/syncable-iac-pipeline.md`

- [ ] **Step 3: Commit**

```bash
git add skills/workflows/syncable-iac-pipeline.md
git commit -m "feat(skills): add syncable-iac-pipeline workflow skill"
```

---

### Task 12: Write `syncable-deploy-pipeline` workflow skill

**Files:**
- Create: `skills/workflows/syncable-deploy-pipeline.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: syncable-deploy-pipeline
description: Use when deploying a project through Syncable - orchestrates auth, analysis, security gating, and deployment via Syncable CLI. Trigger on: "deploy this", "push to syncable", "set up deployment", "deploy my project".
---

## Purpose

Orchestrate a full deployment pipeline through the Syncable platform: authenticate, analyze the project, run a security audit as a gate, then deploy. Ensures no deployment happens without authentication and security review.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Internet access for Syncable API
- Agent has access to the project directory

## Workflow Steps

### Step 1: Check authentication and platform context

```bash
sync-ctl auth status
```

**Decision point:** If not authenticated:
```bash
sync-ctl auth login
```

Then verify project/environment context:
```bash
sync-ctl project current
```

**Decision point:** If no project selected:
```bash
sync-ctl org list
# Ask user which org
sync-ctl org select <ORG_ID>
sync-ctl project list
# Ask user which project
sync-ctl project select <PROJECT_ID>
sync-ctl env list
# Ask user which environment
sync-ctl env select <ENV_ID>
```

### Step 2: Analyze the project

```bash
sync-ctl analyze <PATH> --json
```

### Step 3: Pre-deploy security audit

Execute the `syncable-security-audit` workflow inline (all its steps and decision logic). **Note:** Step 2's analyze output is reused here — do not re-run analyze.

1. `sync-ctl security <PATH> --mode paranoid --format json`
2. `sync-ctl vulnerabilities <PATH> --format json`
3. `sync-ctl validate <PATH>` (if IaC files exist per Step 2's analysis)

**CRITICAL GATE:** If the security audit finds **critical** findings:
- Present all critical findings to the user
- Explicitly warn: "Critical security findings detected. Deploying with these issues is not recommended."
- Ask the user whether to proceed or abort
- **Never deploy silently when critical findings exist**

### Step 4: Deploy

```bash
sync-ctl deploy wizard
```

Then monitor:
```bash
sync-ctl deploy status <TASK_ID> --watch
```

## Decision Points Summary

| Condition | Action |
|-----------|--------|
| Not authenticated | Run `sync-ctl auth login` first |
| No project/env selected | Guide user through selection |
| Critical security findings | Warn user, require explicit confirmation to proceed |
| High security findings (no critical) | Warn user but allow deployment |
| Clean security audit | Proceed to deploy |

## Safety

- **Never deploy without the security gate.** Even if the user says "just deploy", run at least a fast security scan.
- **Always confirm with the user before triggering deployment.** Show them what will be deployed, to which environment.
- **Monitor deployment status** after triggering — don't fire-and-forget.
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -5 skills/workflows/syncable-deploy-pipeline.md`

- [ ] **Step 3: Commit**

```bash
git add skills/workflows/syncable-deploy-pipeline.md
git commit -m "feat(skills): add syncable-deploy-pipeline workflow skill"
```

---

### Task 13: Final verification and summary commit

**Files:**
- Verify: all 11 skill files exist in `skills/`

- [ ] **Step 1: Verify all files exist**

```bash
ls -la skills/commands/ skills/workflows/
```

Expected:
- `skills/commands/`: 7 files (analyze, security, vulnerabilities, dependencies, validate, optimize, platform)
- `skills/workflows/`: 4 files (project-assessment, security-audit, iac-pipeline, deploy-pipeline)

- [ ] **Step 2: Verify all frontmatter is valid**

```bash
for f in skills/commands/*.md skills/workflows/*.md; do echo "=== $f ==="; head -4 "$f"; echo; done
```

Expected: Each file starts with `---`, `name: syncable-*`, `description: ...`, `---`

- [ ] **Step 3: Verify git status is clean**

```bash
git log --oneline -15
```

Expected: 13 commits on this branch (1 spec + 1 scaffold + 7 command skills + 4 workflow skills)
