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
