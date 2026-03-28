<p align="center">
  <img src="https://raw.githubusercontent.com/syncable-dev/syncable-cli/main/logo.png" alt="Syncable" width="320" />
</p>

<h1 align="center">syncable-cli-skills</h1>

<p align="center">
  <strong>Teach your AI coding agent how to use the Syncable CLI toolbox.</strong>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/syncable-cli-skills"><img src="https://img.shields.io/npm/v/syncable-cli-skills.svg" alt="npm version" /></a>
  <a href="https://www.npmjs.com/package/syncable-cli-skills"><img src="https://img.shields.io/npm/dm/syncable-cli-skills.svg" alt="npm downloads" /></a>
  <a href="https://github.com/syncable-dev/syncable-cli"><img src="https://img.shields.io/github/license/syncable-dev/syncable-cli.svg" alt="license" /></a>
  <img src="https://img.shields.io/node/v/syncable-cli-skills.svg" alt="node version" />
</p>

---

One command installs **11 skills** (7 command + 4 workflow) that give AI coding agents full access to Syncable's security scanning, vulnerability detection, IaC validation, Kubernetes optimization, and deployment pipeline - all through the `sync-ctl` CLI.

## Supported Agents

| Agent | Install Type | Format |
|-------|-------------|--------|
| **Claude Code** | Global (`~/.claude/skills/`) | Native markdown |
| **Codex** | Global (`~/.codex/skills/`) | `SKILL.md` directories |
| **Cursor** | Per-project (`.cursor/rules/`) | `.mdc` with `alwaysApply` |
| **Windsurf** | Per-project (`.windsurf/rules/`) | `.md` with `trigger: always` |
| **Gemini CLI** | Per-project (`GEMINI.md`) | Concatenated markdown block |

## Quick Start

```bash
npx syncable-cli-skills
```

That's it. The installer will:

1. Check if `sync-ctl` is installed (and offer to install it via `cargo` if not)
2. Detect which AI coding agents you have
3. Let you pick which agents should receive skills
4. Install skills in each agent's native format

Then open your agent and say: **"assess this project"**

## What Gets Installed

### Command Skills

Atomic wrappers around individual `sync-ctl` commands. Each skill teaches the agent when and how to invoke a specific tool:

| Skill | What it does |
|-------|-------------|
| `syncable-analyze` | Detect tech stack, languages, frameworks, dependencies |
| `syncable-security` | Scan for secrets, hardcoded credentials, insecure patterns |
| `syncable-vulnerabilities` | Check dependencies for known CVEs |
| `syncable-dependencies` | Audit licenses, find outdated or deprecated packages |
| `syncable-validate` | Lint Dockerfiles, Compose files, Terraform, K8s manifests |
| `syncable-optimize` | Analyze Kubernetes resource requests, limits, cost efficiency |
| `syncable-platform` | Authenticate, select projects/environments, deploy |

### Workflow Skills

Multi-step orchestrations that chain command skills together with decision logic:

| Skill | What it does |
|-------|-------------|
| `syncable-project-assessment` | Full health check: stack analysis + security + vulnerabilities + dependencies |
| `syncable-security-audit` | Deep pre-deployment security review with paranoid mode scanning |
| `syncable-iac-pipeline` | Validate all IaC files + Kubernetes optimization (conditional) |
| `syncable-deploy-pipeline` | End-to-end deploy: auth, analyze, security gate, deploy + monitor |

## CLI Reference

```
npx syncable-cli-skills [command] [options]
```

### Commands

| Command | Description |
|---------|-------------|
| `install` | Install sync-ctl and skills *(default)* |
| `uninstall` | Remove skills from agents |
| `update` | Update skills to the latest version |
| `status` | Show what's installed and where |

### Options

| Option | Description |
|--------|-------------|
| `--skip-cli` | Skip the sync-ctl installation check |
| `--dry-run` | Preview what would happen without making changes |
| `--agents <list>` | Comma-separated list: `claude,cursor,windsurf,codex,gemini` |
| `--global-only` | Only install to global agents (Claude Code, Codex) |
| `--project-only` | Only install to project-level agents (Cursor, Windsurf, Gemini) |
| `-y, --yes` | Skip all confirmation prompts |
| `--verbose` | Show detailed output for debugging |

### Examples

```bash
# Interactive install (default)
npx syncable-cli-skills

# Install for specific agents only
npx syncable-cli-skills install --agents claude,cursor

# Non-interactive CI install
npx syncable-cli-skills install --yes --global-only

# Preview without making changes
npx syncable-cli-skills install --dry-run

# Check current installation status
npx syncable-cli-skills status

# Update to latest skills
npx syncable-cli-skills update

# Remove all skills
npx syncable-cli-skills uninstall --yes
```

## Prerequisites

- **Node.js >= 18** (required to run the installer)
- **Rust / Cargo** (required for `sync-ctl` - the installer can set this up for you)
- **sync-ctl** (the Syncable CLI - the installer can install this via `cargo install syncable-cli`)

If `cargo` or `sync-ctl` are missing, the installer will walk you through setting them up interactively.

## How It Works

Skills are markdown files with YAML frontmatter that describe **when** to use a tool and **how** to invoke it. They don't execute code directly - they teach the AI agent's reasoning layer about the capabilities available through `sync-ctl`.

When an agent encounters a task like "check this project for vulnerabilities," the skill gives it the exact command, flags, output interpretation logic, and error handling to do the job correctly.

Each agent has its own format for loading skills:

- **Claude Code** reads `.md` files from `~/.claude/skills/`
- **Codex** reads `SKILL.md` from directories in `~/.codex/skills/`
- **Cursor** reads `.mdc` files with special frontmatter from `.cursor/rules/`
- **Windsurf** reads `.md` files with `trigger: always` from `.windsurf/rules/`
- **Gemini CLI** reads from a `GEMINI.md` file in the project root

The installer transforms skills into each format automatically.

## Updating

Skills are bundled in the npm package. When new skills or improvements are published:

```bash
npx syncable-cli-skills update
```

This removes the old skills and installs the latest version.

## Uninstalling

```bash
npx syncable-cli-skills uninstall
```

This removes only the Syncable skills. It does **not** uninstall `sync-ctl`, `cargo`, or `rustup` - those are general-purpose tools you may rely on.

## Privacy

This installer runs entirely locally. It does not collect analytics, send telemetry, or phone home. The only network requests it makes are to install `rustup` or `sync-ctl` if you explicitly approve those steps.

## License

GPL-3.0 - See [LICENSE](https://github.com/syncable-dev/syncable-cli/blob/main/LICENSE) for details.

## Links

- [Syncable CLI](https://github.com/syncable-dev/syncable-cli) - The CLI toolbox these skills teach agents to use
- [crates.io](https://crates.io/crates/syncable-cli) - `cargo install syncable-cli`
