# Syncable CLI Skills NPX Installer Design

## Context

The Syncable CLI (`sync-ctl`) ships with 11 skills (7 command + 4 workflow) that teach AI coding agents how to use the CLI toolbox. Users need a way to:

1. Install `sync-ctl` (and its Rust toolchain dependency) if missing
2. Detect which AI coding agents they have installed
3. Install skills in the correct format for each agent
4. Keep skills updated

This installer lives inside the `syncable-cli` repo at `installer/` and is published to npm as `syncable-cli-skills`.

## Package Details

- **npm name:** `syncable-cli-skills`
- **Usage:** `npx syncable-cli-skills`
- **Location in repo:** `installer/`
- **Tech stack:** TypeScript, commander, inquirer, ora, chalk
- **Source:** open source (part of syncable-cli repo)

## Directory Structure

```
installer/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts              # CLI entrypoint (commander setup)
│   ├── commands/
│   │   ├── install.ts        # install command (default)
│   │   ├── uninstall.ts      # remove skills from agents
│   │   ├── update.ts         # update skills (uninstall + install)
│   │   └── status.ts         # show what's installed where
│   ├── prerequisites/
│   │   ├── check.ts          # check rustup, cargo, sync-ctl
│   │   ├── install-rustup.ts # install Rust toolchain
│   │   └── install-cli.ts    # cargo install syncable-cli
│   ├── agents/
│   │   ├── types.ts          # AgentConfig interface
│   │   ├── detect.ts         # detect all installed agents
│   │   ├── claude.ts         # Claude Code config
│   │   ├── cursor.ts         # Cursor config
│   │   ├── windsurf.ts       # Windsurf config
│   │   ├── codex.ts          # Codex config
│   │   └── gemini.ts         # Gemini CLI config
│   ├── transformers/
│   │   ├── claude.ts         # no-op (native format)
│   │   ├── codex.ts          # wrap in SKILL.md directory
│   │   ├── cursor.ts         # convert to .mdc format
│   │   ├── windsurf.ts       # convert to windsurf rule format
│   │   └── gemini.ts         # concatenate into GEMINI.md section
│   ├── skills.ts             # load bundled skill files, parse frontmatter
│   └── utils.ts              # shell exec, spinner, platform helpers
├── skills/                   # bundled copy of ../skills/ (build artifact)
└── dist/                     # compiled JS output
```

## CLI Interface

```
npx syncable-cli-skills [command] [options]

Commands:
  install     Install sync-ctl and skills (default)
  uninstall   Remove skills from agents
  update      Update skills to latest version
  status      Show what's installed and where

Options:
  --skip-cli          Skip sync-ctl installation check
  --dry-run           Show what would be done without doing it
  --agents <list>     Comma-separated: claude,cursor,windsurf,codex,gemini
  --global-only       Only install global skills (Claude, Codex)
  --project-only      Only install project-level rules (Cursor, Windsurf, Gemini)
  -y, --yes           Skip confirmations (auto-yes)
  --verbose           Show detailed output for debugging
  -h, --help          Show help
  -v, --version       Show version
```

Running with no arguments defaults to `install`.

## Interactive Install Flow

```
$ npx syncable-cli-skills

  Syncable CLI Skills Installer
  ─────────────────────────────

  Checking prerequisites...

  ✓ Node.js v20.11.0
  ✗ sync-ctl not found
  ✗ cargo not found

  sync-ctl (Syncable CLI) is required but not installed.
  This requires Rust's cargo package manager.

  ? Install Rust toolchain via rustup? (Y/n) Y

  ◐ Installing rustup...
  ✓ Rust toolchain installed (cargo 1.79.0)

  ? Install syncable-cli via cargo? (Y/n) Y

  ◐ Running: cargo install syncable-cli
  ✓ sync-ctl installed (v0.36.0)

  Detecting AI coding agents...

  ✓ Claude Code detected (~/.claude/)
  ✓ Cursor detected (~/.cursor/)
  ✗ Windsurf not detected
  ✓ Codex detected (~/.codex/)
  ✗ Gemini CLI not detected

  ? Which agents should receive Syncable skills?
    ◉ Claude Code — global install (~/.claude/skills/syncable/)
    ◉ Cursor — project install (.cursor/rules/syncable-*.mdc)
    ◉ Codex — global install (~/.codex/skills/syncable-*/)
    ──────────────
    ◯ Windsurf (not detected — install anyway?)
    ◯ Gemini CLI (not detected — install anyway?)

  ◐ Installing skills for Claude Code...
  ✓ 11 skills installed to ~/.claude/skills/syncable/

  ◐ Installing skills for Cursor...
  ✓ 11 skills installed to .cursor/rules/

  ◐ Installing skills for Codex...
  ✓ 11 skills installed to ~/.codex/skills/

  ─────────────────────────────
  ✓ Setup complete!

  Installed:
    • sync-ctl v0.36.0
    • 7 command skills + 4 workflow skills
    • Agents: Claude Code, Cursor, Codex

  Try it: Open Claude Code and say "assess this project"
```

## Agent Detection & Installation

### Agent Categories

**Global install (skills persist across all projects):**

| Agent | Detection | Skill Path | Format |
|-------|-----------|-----------|--------|
| Claude Code | `~/.claude/` exists | `~/.claude/skills/syncable/commands/*.md`, `~/.claude/skills/syncable/workflows/*.md` | Native — our format matches |
| Codex | `~/.codex/` exists | `~/.codex/skills/syncable-<name>/SKILL.md` | Each skill is a directory with SKILL.md |

**Project install (skills are per-repo, installed into current working directory):**

| Agent | Detection | Skill Path | Format |
|-------|-----------|-----------|--------|
| Cursor | `~/.cursor/` exists | `.cursor/rules/syncable-<name>.mdc` | `.mdc` with `description`/`globs`/`alwaysApply` frontmatter |
| Windsurf | `~/.codeium/windsurf/` exists | `.windsurf/rules/syncable-<name>.md` | `.md` with `trigger`/`description` frontmatter |
| Gemini CLI | `~/.gemini/` exists | `GEMINI.md` in project root (append) | Concatenated markdown block with markers |

### Platform Paths

- macOS/Linux: `~/` expands to `os.homedir()`
- Windows: `os.homedir()` returns `C:\Users\<name>`
- All paths use `path.join()` for cross-platform safety

## Skill Format Transformers

### Claude Code (no transform)

Skills copy as-is. The `commands/` and `workflows/` directory structure is preserved under `~/.claude/skills/syncable/`.

### Codex

Each skill becomes a directory under `~/.codex/skills/`:

```
~/.codex/skills/syncable-analyze/
└── SKILL.md
```

Frontmatter stays the same (`name`, `description`). Body stays the same.

### Cursor

Each skill becomes a `.mdc` file. Frontmatter is transformed:

**From (our format):**
```yaml
---
name: syncable-analyze
description: Use when analyzing...
---
```

**To (.mdc format):**
```yaml
---
description: "Syncable CLI: Use when analyzing..."
globs:
alwaysApply: true
---
```

The `name` field is dropped (filename is the identifier). `globs` is empty (skills apply globally). `alwaysApply: true` ensures the agent always has access to the skill context.

### Windsurf

Each skill becomes a `.md` file in `.windsurf/rules/`. Frontmatter is transformed:

**To:**
```yaml
---
trigger: always
description: "Syncable CLI: Use when analyzing..."
---
```

### Gemini CLI

All skills are concatenated into a single block and appended to `GEMINI.md`:

```markdown
<!-- SYNCABLE-CLI-SKILLS-START -->
## Syncable CLI Skills

The following skills describe how to use the Syncable CLI (sync-ctl) toolbox.

### syncable-analyze
[full skill content]

### syncable-security
[full skill content]

...
<!-- SYNCABLE-CLI-SKILLS-END -->
```

The HTML comment markers allow `uninstall` and `update` to cleanly find and remove/replace the section without touching user content.

If `GEMINI.md` doesn't exist, create it. If it exists, append (preserving existing content).

## Prerequisite Installation

### Rustup / Cargo

Detection: check if `cargo` is on PATH or at `$HOME/.cargo/bin/cargo`.

If missing, install via:

**macOS/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

**Windows (fallback chain):**
1. Try `winget install Rustlang.Rustup` (requires no admin on modern Windows)
2. If winget unavailable, download and run `rustup-init.exe` from https://rustup.rs
3. If both fail, show manual install instructions and skip

**PATH propagation after install:** Node's `child_process.exec` spawns a new shell each time, so `source ~/.cargo/env` has no effect. After rustup completes, the installer must prepend `$HOME/.cargo/bin` to `process.env.PATH` in the Node process so subsequent `exec` calls find `cargo`.

```typescript
process.env.PATH = `${path.join(os.homedir(), '.cargo', 'bin')}${path.delimiter}${process.env.PATH}`;
```

### sync-ctl

Detection: `sync-ctl --version` (or check `$HOME/.cargo/bin/sync-ctl`).

If missing (but cargo exists):
```bash
cargo install syncable-cli
```

If already installed, parse version from `sync-ctl --version` output. Only offer `cargo install syncable-cli --force` if the installed version is older than the minimum required version. This avoids unnecessary multi-minute recompiles.

The npm package should declare a `MIN_SYNC_CTL_VERSION` constant that matches the skills it bundles.

## Commands

### install (default)

1. Check prerequisites (Node.js, cargo, sync-ctl)
2. Offer to install missing prerequisites interactively
3. Detect installed agents
4. Present agent checklist (detected pre-selected, undetected available)
5. For each selected agent: transform and install skills
6. Print summary

### uninstall

1. Detect where skills are installed
2. Confirm removal
3. For global agents: remove `~/.claude/skills/syncable/`, `~/.codex/skills/syncable-*/`
4. For project agents: remove `.cursor/rules/syncable-*.mdc`, `.windsurf/rules/syncable-*.md`
5. For Gemini: remove content between `SYNCABLE-CLI-SKILLS-START` and `SYNCABLE-CLI-SKILLS-END` markers in `GEMINI.md`

### update

Uninstall then install. Uses bundled skills from the npm package (which reflect the version at publish time).

### status

Show a table:
```
  Agent         Status      Location
  ───────────── ─────────── ──────────────────────────
  Claude Code   ✓ installed ~/.claude/skills/syncable/ (11 skills)
  Cursor        ✓ installed .cursor/rules/ (11 .mdc files)
  Windsurf      ✗ not installed
  Codex         ✓ installed ~/.codex/skills/ (11 skills)
  Gemini CLI    ✗ not installed

  sync-ctl      ✓ v0.36.0
  cargo         ✓ v1.79.0
```

## Build & Publish

### Build script (package.json)

```json
{
  "scripts": {
    "prebuild": "node scripts/copy-skills.js",
    "build": "tsc",
    "prepublishOnly": "npm run build"
  }
}
```

The `prebuild` step runs a Node script (`scripts/copy-skills.js`) that copies `../skills/` into `installer/skills/` using `fs-extra`. This is cross-platform safe (no reliance on Unix `cp`). The `installer/skills/` directory must be in `.gitignore` to avoid duplicating the canonical source.

### Module system: ESM

The package uses `"type": "module"` since `chalk@5`, `ora@8`, and `inquirer@9` are ESM-only. TypeScript compiles to ESM output (`"module": "NodeNext"` in tsconfig). Requires Node.js >= 18.

### package.json key fields

```json
{
  "name": "syncable-cli-skills",
  "version": "0.1.0",
  "type": "module",
  "description": "Install Syncable CLI skills for AI coding agents",
  "bin": {
    "syncable-cli-skills": "./dist/index.js"
  },
  "files": [
    "dist/",
    "skills/"
  ],
  "engines": {
    "node": ">=18.0.0"
  },
  "dependencies": {
    "commander": "^12.0.0",
    "inquirer": "^9.0.0",
    "ora": "^8.0.0",
    "chalk": "^5.0.0",
    "fs-extra": "^11.0.0"
  },
  "devDependencies": {
    "@types/fs-extra": "^11.0.0",
    "typescript": "^5.0.0"
  }
}
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| No internet (can't install rustup/cargo) | Show manual install instructions, skip to agent detection |
| cargo install fails | Show error, suggest `cargo install syncable-cli` manually, continue with skills |
| Agent directory not writable | Warn and skip that agent |
| Skill file already exists | Overwrite (with confirmation unless `--yes`) |
| GEMINI.md has existing Syncable section | Replace between markers |
| Windows without curl | Use `winget` for rustup, fall back to manual instructions |
| Node.js too old | Check `>=18.0.0` at startup, exit with clear message |

## Design Decisions

1. **Skills bundled in npm package** — no network fetch at install time. Skills version matches the npm package version.
2. **Interactive by default** — every destructive or install action requires confirmation. `--yes` flag for CI/scripting.
3. **Per-project for Cursor/Windsurf/Gemini** — these agents don't have global skill directories, so skills must be installed per-project. The installer makes this clear.
4. **Gemini uses comment markers** — allows clean uninstall/update without destroying user content in GEMINI.md.
5. **Transformers are pure functions** — each takes a parsed skill (frontmatter + body) and returns the target format string. Easy to test, easy to add new agents.
6. **Codex gets directory-per-skill** — matches Codex's native SKILL.md convention.
7. **Cursor uses alwaysApply: true** — skills should always be available, not scoped to specific file globs.
8. **Uninstall only removes skills** — `sync-ctl`, `cargo`, and `rustup` are intentionally left installed. They are general-purpose tools the user may rely on beyond Syncable.
9. **Dynamic skill count** — the installer counts bundled skills at runtime rather than hardcoding "11 skills". If skills are added or removed, output reflects the actual count.
10. **No telemetry** — the installer does not phone home, collect analytics, or send any data. It runs entirely locally.
11. **`--agents` takes precedence over `--global-only`/`--project-only`** — if `--agents` is specified, it overrides the global/project filters. The flags are for convenience when `--agents` is not used.
12. **Agent detection uses both directory and PATH** — Codex is detected via `~/.codex/` OR `codex` on PATH. Gemini CLI via `~/.gemini/` OR `gemini` on PATH. This avoids false positives from other tools that may use the same directories.
13. **ESM module system** — the package uses `"type": "module"` because `chalk@5`, `ora@8`, and `inquirer@9` are ESM-only. Requires Node.js >= 18.
14. **`installer/skills/` is gitignored** — this directory is a build artifact copied from `../skills/` at build time. It must be in `.gitignore`.
