---
name: syncable-analyze
description: Analyze a project's tech stack including languages, frameworks, runtimes, package managers, and dependencies using the Syncable CLI sync-ctl tool
---

## Purpose

Analyze a project directory to detect its tech stack: programming languages, frameworks, runtimes, package managers, dependencies, Docker presence, and monorepo structure. This is the foundation skill — most workflows start here to understand what they're working with.

## Prerequisites

- `sync-ctl` binary installed and on PATH
- Agent has access to the project directory

## Commands

### Basic analysis (agent output)

```bash
sync-ctl analyze <PATH> --agent
```

### Human-readable matrix view

```bash
sync-ctl analyze <PATH> --display matrix
```

### Filtered analysis (only specific aspects)

```bash
sync-ctl analyze <PATH> --agent --only languages,frameworks
sync-ctl analyze <PATH> --agent --only dependencies
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--agent` | Compressed output for agent consumption (always use when processing results) |
| `--detailed` | Show detailed analysis (legacy vertical format) |
| `--display {matrix\|detailed\|summary}` | Display format for human-readable output |
| `--only <filters>` | Comma-separated: `languages`, `frameworks`, `dependencies` |

## Output Interpretation

When reporting to the user, prioritize: primary language, main framework, runtime version, and whether Docker/K8s infrastructure exists.

## Reading Results

When you use `--agent`, the output is a compressed summary — not the full analysis. Act on it directly for most decisions.

The output JSON includes:
- `summary` — project count, languages, frameworks detected
- `full_data_ref` — reference ID for retrieving full data
- `retrieval_hint` — exact command to get more details

To drill into specifics:
```bash
# Get framework details
sync-ctl retrieve <ref_id> --query "section:frameworks"

# Get language breakdown
sync-ctl retrieve <ref_id> --query "section:languages"

# Get specific project details (monorepos)
sync-ctl retrieve <ref_id> --query "project:<project-name>"

# Get specific language details
sync-ctl retrieve <ref_id> --query "language:Go"

# Get specific framework details
sync-ctl retrieve <ref_id> --query "framework:React"

# List all stored outputs
sync-ctl retrieve --list
```

**Available query filters:** `section:summary`, `section:frameworks`, `section:languages`, `language:<name>`, `framework:<name>`, `project:<name>`, `compact:true`

## Error Handling

| Error | Cause | Action |
|-------|-------|--------|
| `No such file or directory` | Invalid path | Ask user to verify the project path |
| Empty output | No recognizable project files | Tell user the directory may not contain a supported project. Run `sync-ctl support` to show supported technologies |
| Timeout | Very large monorepo | Try `--only languages` for a faster partial scan |

## Examples

**Analyze current directory:**
```bash
sync-ctl analyze . --agent
```

**Analyze a specific project:**
```bash
sync-ctl analyze /path/to/project --agent
```

**Quick language-only check:**
```bash
sync-ctl analyze . --agent --only languages
```
