# Agent Output Pipeline Design

## Problem

When external AI agents (Claude Code, Cursor, Windsurf, Codex, Gemini) run `sync-ctl` commands via the installed skills, raw command output goes directly to stdout. A full project scan can produce hundreds of thousands of tokens, which:

1. Overflows the agent's context window
2. Burns through the user's token budget
3. Degrades agent reasoning quality (buried in noise)

The syncable-cli already has a production-grade RAG pipeline for its **internal** agent (`output_store.rs`, `compression.rs`, `retrieve.rs`, `truncation.rs`). But this pipeline is only wired to the internal agent tools — it's invisible to external agents running `sync-ctl` as a shell command.

## Solution

Expose the existing RAG pipeline through two new CLI features:

1. **`--agent` flag** on all scan commands — routes output through compression + disk storage, returns a ~15KB compressed summary to stdout
2. **`retrieve` subcommand** — lets agents query stored full data by ref_id with filtering

External agents get the exact same compressed format (`CompressedOutput`) that the internal syncable agent already uses. No new compression logic. No new storage format. Just CLI plumbing to expose what exists.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Output format for external agents | Same `CompressedOutput` as internal agent | Reuse existing compression, no format duplication |
| `--agent` behavior on stdout | Replaces stdout with compressed summary | Whole point is agents never see full blast |
| Retrieval interface | Top-level `retrieve` subcommand | Separate concern from scan commands; matches internal `RetrieveOutputTool` |
| Opt-in vs default | New `--agent` flag (opt-in) | `--json` stays untouched for human scripts; `--agent` implies `--json` |

## Subsystem 1: CLI Changes (Rust)

### `--agent` Flag

**Added to:** `analyze`, `security`, `vulnerabilities`, `dependencies`, `validate`, `optimize`

**Behavior when `--agent` is passed:**

1. Run the command normally, produce full output internally
2. Route through existing `compress_tool_output()` (for scan/lint commands) or `compress_analysis_output()` (for analyze)
3. Store full output to disk via `output_store::store_output()` at `/tmp/syncable-cli/outputs/{ref_id}.json` (1hr TTL)
4. Register in session via `output_store::register_session_ref()` (no-op for cross-process retrieval — kept for code path reuse with internal agent; `latest` resolution uses disk scan instead)
5. Print `CompressedOutput` JSON to stdout

**`--agent` implies `--json`** — no need to pass both.

**Existing flags unchanged:** `--json`, `--format`, `--display` all work exactly as before.

**`--agent` + `--output` interaction:** When both are passed, the compressed summary goes to stdout and the full uncompressed output is written to the `--output` file. This lets agents get the summary for reasoning while preserving full data in a file for human review.

**Strict JSON output:** `--agent` mode produces valid JSON only — no appended plaintext (the `format_session_refs_for_agent()` footer used by the internal agent is skipped). The `retrieval_hint` field inside the JSON uses CLI syntax (`sync-ctl retrieve '<ref_id>' --query '...'`) instead of the internal tool call format.

**TTL cleanup:** At the start of `--agent` processing, call `cleanup_old_outputs()` to remove expired files from `/tmp/syncable-cli/outputs/`. This prevents unbounded disk growth since external CLI invocations are short-lived (unlike the internal agent which runs cleanup periodically).

**Files to modify:**

- `src/cli.rs` — Add `agent: bool` field to `Analyze`, `Security`, `Vulnerabilities`, `Dependencies`, `Validate`, `Optimize` command structs
- `src/main.rs` — In each command handler, check `agent` flag; if true, route output through compression pipeline instead of direct printing
- `src/agent/tools/output_store.rs` — Fix `find_issues_array()` to include `"failures"` and `"diagnostics"` fields (currently missing, causing retrieval to return empty for kubelint/hadolint/dclint/helmlint outputs)
- `src/agent/tools/compression.rs` — When called from `--agent` mode, skip `format_session_refs_for_agent()` plaintext footer so output is strict JSON; add CLI-syntax `retrieval_hint` (e.g., `sync-ctl retrieve '<ref_id>' --query '...'` instead of internal `retrieve_output(...)` tool call format)

### `CompressedOutput` Format (already defined in `compression.rs`)

For scan/lint commands:
```json
{
  "tool": "security",
  "status": "CRITICAL_ISSUES_FOUND",
  "summary": { "total": 47, "critical": 3, "high": 12, "medium": 20, "low": 8, "info": 4 },
  "critical_issues": [ "...full details for all critical..." ],
  "high_issues": [ "...first 10 high issues..." ],
  "patterns": [ { "code": "hardcoded-secret", "count": 5, "severity": "High", "message": "...", "affected_files": ["..."] } ],
  "full_data_ref": "security_a1b2c3d4",
  "retrieval_hint": "Use `sync-ctl retrieve 'security_a1b2c3d4' --query 'severity:critical'` for full details"
}
```

For analyze:
```json
{
  "tool": "analyze_project",
  "summary": { "project_count": 3, "languages": ["Rust", "TypeScript"], "frameworks": ["Axum", "React"] },
  "full_data_ref": "analyze_e5f6g7h8",
  "retrieval_hint": "Use `sync-ctl retrieve 'analyze_e5f6g7h8' --query 'section:frameworks'` for details"
}
```

### `retrieve` Subcommand

**Definition in `src/cli.rs`:**
```
Retrieve {
    ref_id: Option<String>,     // e.g., "security_a1b2c3d4" or "latest"
    query: Option<String>,      // e.g., "severity:critical", "file:path"
    list: bool,                 // --list: show all stored outputs
}
```

**Behavior:**

- `sync-ctl retrieve --list` — Calls `output_store::list_outputs()`, prints table of ref_id / tool / age / size
- `sync-ctl retrieve <ref_id>` — Calls `output_store::retrieve_output(ref_id)`, prints full stored JSON
- `sync-ctl retrieve <ref_id> --query "<filter>"` — Calls retrieval with filtering from `retrieve.rs`
- `sync-ctl retrieve latest --query "<filter>"` — Resolves `latest` by scanning `/tmp/syncable-cli/outputs/*.json` files, sorting by embedded timestamp, and using the most recent. This works across separate CLI invocations (no in-memory state needed).

**Query filters (already implemented in `retrieve.rs`):**

| Filter | Example | Works with |
|--------|---------|-----------|
| `severity:<level>` | `severity:critical` | security, vulnerabilities, validate, optimize |
| `file:<path>` | `file:deployment.yaml` | All scan commands |
| `code:<id>` | `code:DL3008` | validate, security |
| `container:<name>` | `container:nginx` | optimize |
| `section:<name>` | `section:frameworks` | analyze |
| `language:<name>` | `language:Go` | analyze |
| `framework:<name>` | `framework:React` | analyze |
| `project:<name>` | `project:api-gateway` | analyze |
| `compact:true` | `compact:true` | analyze |

**Output:** Always JSON. If filtered result > 50KB, prints warning suggesting more specific query.

**`retrieve --list` format:** Always JSON (array of objects with `ref_id`, `tool`, `timestamp`, `age`, `size_bytes` fields). External agents consume this programmatically.

**Error handling:** If `ref_id` is not found (expired or invalid), return JSON error object `{ "error": "not_found", "message": "..." , "available": [...] }` with non-zero exit code. The `available` array lists valid ref_ids so the agent can self-correct.

**Handler in `src/main.rs`:** Thin wrapper calling existing `output_store` and `retrieve` module functions. The logic already exists — this just exposes it through CLI args.

**TTL cleanup:** At the start of `retrieve` subcommand execution, call `cleanup_old_outputs()` to remove expired files.

**`latest` resolution:** Implemented by scanning all `*.json` files in `/tmp/syncable-cli/outputs/`, reading the `timestamp` field from each, and selecting the most recent. This is disk-based and works across separate process invocations — no dependency on the in-memory `SESSION_REGISTRY`.

## Subsystem 2: Skill Rewrites (Markdown)

### Command Skills (7 files)

Each command skill changes:

1. **Command invocations** switch from `--json` to `--agent`:
   - Before: `sync-ctl security <PATH> --mode paranoid --format json`
   - After: `sync-ctl security <PATH> --mode paranoid --agent`

2. **New "Reading Results" section** added to each skill, teaching the agent:
   - The output is a compressed summary — act on it directly for triage and decisions
   - All critical issues are always included in full
   - To drill into specifics: `sync-ctl retrieve <ref_id> --query "<filter>"`
   - Skill-specific query filters documented (each skill lists its applicable filters)

3. **Raw JSON parsing instructions removed** — no more "parse the findings array" or "count the issues manually"

### Workflow Skills (4 files)

Same changes as command skills, plus:

1. **Cross-step retrieval** — Workflows teach agents to reuse `ref_id` from earlier steps. Example in project-assessment: "The analyze step returned `full_data_ref`. Use this ref_id with `sync-ctl retrieve` if you need details later — don't re-run analyze."

2. **`sync-ctl retrieve --list`** — Workflows teach agents to check what data is already stored before running redundant commands.

### Skill-Specific Query Filter Documentation

| Skill | Filters to Document |
|-------|-------------------|
| `syncable-analyze` | `section:summary`, `section:frameworks`, `section:languages`, `language:<name>`, `framework:<name>`, `project:<name>`, `compact:true` |
| `syncable-security` | `severity:<level>`, `file:<path>`, `code:<id>` |
| `syncable-vulnerabilities` | `severity:<level>`, `file:<path>` |
| `syncable-dependencies` | `severity:<level>`, `file:<path>` (new wiring needed — dependency audit findings follow the same issues/findings array pattern as other scan commands, but `compress_tool_output()` is not yet called from the dependencies code path) |
| `syncable-validate` | `severity:<level>`, `file:<path>`, `code:<id>` |
| `syncable-optimize` | `severity:<level>`, `container:<name>` |
| `syncable-platform` | N/A (action commands — auth/deploy are imperative, not scan/analysis, so `--agent` compression is not applicable) |

### Files Modified

- `skills/commands/syncable-analyze.md`
- `skills/commands/syncable-security.md`
- `skills/commands/syncable-vulnerabilities.md`
- `skills/commands/syncable-dependencies.md`
- `skills/commands/syncable-validate.md`
- `skills/commands/syncable-optimize.md`
- `skills/commands/syncable-platform.md` (no `--agent` flag — these are action commands, not scans. Skill unchanged except a note that other skills use `--agent`)
- `skills/workflows/syncable-project-assessment.md`
- `skills/workflows/syncable-security-audit.md`
- `skills/workflows/syncable-iac-pipeline.md`
- `skills/workflows/syncable-deploy-pipeline.md`

## Implementation Order

**Subsystem 1 first** (CLI must support `--agent` and `retrieve` before skills can reference them), **then Subsystem 2**.

## Not In Scope

- Changing compression algorithms or thresholds (already tuned at 15KB target)
- Changing storage location or TTL (1hr at `/tmp/syncable-cli/outputs/`)
- Changes to the internal agent's RAG pipeline (stays identical)
- Changes to the npx installer (skills are content-only changes, format is unchanged)
- New agent detection or format transformer logic
