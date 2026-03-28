---
name: syncable-validate
description: Lint and validate Dockerfiles, Docker Compose files, Kubernetes manifests, Helm charts, and Terraform configs using the Syncable CLI sync-ctl tool
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
sync-ctl validate <PATH> --agent
```

### Validate specific types only

```bash
sync-ctl validate <PATH> --types dockerfile --agent
sync-ctl validate <PATH> --types dockerfile,compose --agent
sync-ctl validate <PATH> --types terraform --agent
```

### Auto-fix issues where possible

```bash
sync-ctl validate <PATH> --fix
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output for agent consumption (always use when processing results) |
| `--types <comma-separated>` | Filter to specific IaC types: `dockerfile`, `compose`, `terraform` |
| `--fix` | Automatically fix issues where possible |

**Note:** The `--agent` flag is available for structured output. When `--agent` is not used, output is terminal text with structured lint violations.

## Output Interpretation

Output contains lint violations, each with:

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

## Reading Results

When you use `--agent`, the output is a compressed summary. All **error**-severity violations are included in full detail. Warnings are deduplicated into patterns.

The output JSON includes:
- `status` — e.g., "ERRORS_FOUND", "WARNINGS_ONLY", "CLEAN"
- `summary` — counts by severity
- `errors` — full details for every error-severity finding
- `patterns` — deduplicated warning/info findings with counts
- `full_data_ref` — reference ID for retrieving full data
- `retrieval_hint` — exact command for drill-down

To drill into specifics:
```bash
# Get all error-severity findings
sync-ctl retrieve <ref_id> --query "severity:high"

# Get findings for a specific file
sync-ctl retrieve <ref_id> --query "file:Dockerfile"

# Get findings by rule code
sync-ctl retrieve <ref_id> --query "code:DL3006"
```

**Available query filters:** `severity:<level>`, `file:<path>`, `code:<id>`

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No IaC files found` | Directory has no Dockerfiles, Compose, or Terraform files | Run `sync-ctl analyze <PATH> --agent` to verify what IaC exists |
| `Unknown type` | Invalid `--types` value | Valid types are: `dockerfile`, `compose`, `terraform` |

## Examples

**Lint all IaC in current directory:**
```bash
sync-ctl validate . --agent
```

**Lint only Dockerfiles:**
```bash
sync-ctl validate . --types dockerfile --agent
```

**Auto-fix Docker Compose issues:**
```bash
sync-ctl validate . --types compose --fix
```
