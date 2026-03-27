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
