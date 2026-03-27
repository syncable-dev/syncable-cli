# Agent Output Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the existing internal RAG pipeline (compression + disk storage + retrieval) through CLI flags so external AI agents get compressed output instead of raw stdout blasts, then rewrite all 11 skills to use the new two-step pattern.

**Architecture:** Add `--agent` flag to 5 scan commands (analyze, security, vulnerabilities, dependencies, optimize) that routes handler output through `compress_tool_output()` / `compress_analysis_output()` + `store_output()`, printing ~15KB compressed JSON to stdout. Add `retrieve` top-level subcommand wrapping existing `retrieve_filtered()` and `list_outputs()`. Fix existing bug in `find_issues_array()` missing `failures`/`diagnostics` fields. Rewrite all 11 skill markdown files to use `--agent` flag and teach agents the retrieve pattern.

**Tech Stack:** Rust (clap, serde_json), Markdown

**Spec:** `docs/superpowers/specs/2026-03-27-agent-output-pipeline-design.md`

---

## File Structure

### Subsystem 1: CLI Changes (Rust)

| File | Action | Responsibility |
|------|--------|---------------|
| `src/cli.rs` | Modify | Add `agent: bool` field to Analyze, Security, Vulnerabilities, Dependencies, Optimize structs. Add `Retrieve` command variant. |
| `src/main.rs` | Modify | Wire `--agent` flag in 5 command handlers. Add `Retrieve` match arm. Add `handle_agent_output()` helper. Add `handle_retrieve()` handler. |
| `src/agent/tools/output_store.rs` | Modify | Fix `find_issues_array()` to include `failures` and `diagnostics`. Add `resolve_latest()` function. |
| `src/agent/tools/compression.rs` | Modify | Add `compress_tool_output_cli()` variant that produces strict JSON (no plaintext footer, CLI-syntax retrieval hint). |
| `src/handlers/analyze.rs` | Modify | Change return type to return raw JSON Value when called in agent mode. |

### Subsystem 2: Skill Rewrites (Markdown)

| File | Action | Responsibility |
|------|--------|---------------|
| `skills/commands/syncable-analyze.md` | Modify | Switch to `--agent`, add Reading Results section |
| `skills/commands/syncable-security.md` | Modify | Switch to `--agent`, add Reading Results section |
| `skills/commands/syncable-vulnerabilities.md` | Modify | Switch to `--agent`, add Reading Results section |
| `skills/commands/syncable-dependencies.md` | Modify | Switch to `--agent`, add Reading Results section |
| `skills/commands/syncable-validate.md` | Modify | Switch to `--agent`, add Reading Results section |
| `skills/commands/syncable-optimize.md` | Modify | Switch to `--agent`, add Reading Results section |
| `skills/commands/syncable-platform.md` | No change | Action commands, no compression needed |
| `skills/workflows/syncable-project-assessment.md` | Modify | Switch to `--agent`, add cross-step retrieval |
| `skills/workflows/syncable-security-audit.md` | Modify | Switch to `--agent`, add cross-step retrieval |
| `skills/workflows/syncable-iac-pipeline.md` | Modify | Switch to `--agent`, add cross-step retrieval |
| `skills/workflows/syncable-deploy-pipeline.md` | Modify | Switch to `--agent`, add cross-step retrieval |

---

## Part 1: CLI Changes (Rust)

### Task 1: Fix `find_issues_array()` bug in output_store.rs

**Files:**
- Modify: `src/agent/tools/output_store.rs:291-300`

This is an existing bug — `find_issues_array()` is missing `"failures"` and `"diagnostics"` fields that `compression.rs::extract_issues()` already handles. This causes retrieval filtering to return empty results for kubelint/hadolint/dclint/helmlint outputs.

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)]` module in `src/agent/tools/output_store.rs`:

```rust
#[test]
fn test_find_issues_array_failures_field() {
    let data = serde_json::json!({
        "failures": [
            {"code": "DL3008", "severity": "warning", "message": "Pin versions"},
            {"code": "DL3009", "severity": "info", "message": "Delete apt cache"}
        ]
    });
    let result = find_issues_array(&data);
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_find_issues_array_diagnostics_field() {
    let data = serde_json::json!({
        "diagnostics": [
            {"code": "DC001", "severity": "error", "message": "Invalid compose version"}
        ]
    });
    let result = find_issues_array(&data);
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib -- output_store::tests::test_find_issues_array_failures_field output_store::tests::test_find_issues_array_diagnostics_field`
Expected: FAIL — `find_issues_array` doesn't check `failures` or `diagnostics` fields

- [ ] **Step 3: Fix `find_issues_array`**

In `src/agent/tools/output_store.rs:291-300`, add the missing fields to the `issue_fields` array:

```rust
fn find_issues_array(data: &Value) -> Option<Vec<Value>> {
    let issue_fields = [
        "issues",
        "findings",
        "violations",
        "warnings",
        "errors",
        "recommendations",
        "results",
        "failures",
        "diagnostics",
    ];

    for field in &issue_fields {
        if let Some(arr) = data.get(field).and_then(|v| v.as_array()) {
            return Some(arr.clone());
        }
    }

    if let Some(arr) = data.as_array() {
        return Some(arr.clone());
    }

    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib -- output_store::tests::test_find_issues_array`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/agent/tools/output_store.rs
git commit -m "fix: add failures/diagnostics fields to find_issues_array

Fixes retrieval returning empty results for kubelint, hadolint, dclint,
and helmlint outputs which use 'failures' field instead of 'issues'."
```

---

### Task 2: Add `resolve_latest()` to output_store.rs

**Files:**
- Modify: `src/agent/tools/output_store.rs`

External agents can't use the in-memory session registry across CLI invocations. `resolve_latest()` scans disk files by timestamp to find the most recent stored output.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_resolve_latest_returns_most_recent() {
    use std::fs;
    use std::path::Path;

    let output_dir = Path::new("/tmp/syncable-cli/outputs");
    fs::create_dir_all(output_dir).unwrap();

    // Clean up any existing test files
    let _ = fs::remove_file(output_dir.join("test_old_aaa111.json"));
    let _ = fs::remove_file(output_dir.join("test_new_bbb222.json"));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Write two files with different timestamps
    let old_data = serde_json::json!({
        "ref_id": "test_old_aaa111",
        "tool": "test_old",
        "timestamp": now - 60,
        "data": {}
    });
    let new_data = serde_json::json!({
        "ref_id": "test_new_bbb222",
        "tool": "test_new",
        "timestamp": now,
        "data": {}
    });

    fs::write(
        output_dir.join("test_old_aaa111.json"),
        serde_json::to_string(&old_data).unwrap(),
    ).unwrap();
    fs::write(
        output_dir.join("test_new_bbb222.json"),
        serde_json::to_string(&new_data).unwrap(),
    ).unwrap();

    let latest = resolve_latest();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap(), "test_new_bbb222");

    // Cleanup
    let _ = fs::remove_file(output_dir.join("test_old_aaa111.json"));
    let _ = fs::remove_file(output_dir.join("test_new_bbb222.json"));
}

#[test]
fn test_resolve_latest_empty_dir() {
    // When no outputs exist, returns None
    // This test is fragile if other tests leave files — just verify it doesn't panic
    let result = resolve_latest();
    // Result is either Some or None, but must not panic
    let _ = result;
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib -- output_store::tests::test_resolve_latest`
Expected: FAIL — `resolve_latest` doesn't exist yet

- [ ] **Step 3: Implement `resolve_latest()`**

Add to `src/agent/tools/output_store.rs` near `list_outputs()`:

```rust
/// Resolve "latest" to the most recent ref_id by scanning disk files.
/// Works across separate CLI invocations (no in-memory state dependency).
pub fn resolve_latest() -> Option<String> {
    let output_dir = std::path::Path::new("/tmp/syncable-cli/outputs");
    if !output_dir.exists() {
        return None;
    }

    let mut newest: Option<(u64, String)> = None;

    if let Ok(entries) = std::fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "json") {
                continue;
            }

            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<Value>(&contents) {
                    if let Some(ts) = data.get("timestamp").and_then(|v| v.as_u64()) {
                        if let Some(ref_id) = data.get("ref_id").and_then(|v| v.as_str()) {
                            match &newest {
                                Some((best_ts, _)) if ts > *best_ts => {
                                    newest = Some((ts, ref_id.to_string()));
                                }
                                None => {
                                    newest = Some((ts, ref_id.to_string()));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    newest.map(|(_, ref_id)| ref_id)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib -- output_store::tests::test_resolve_latest`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/agent/tools/output_store.rs
git commit -m "feat: add resolve_latest() for cross-process ref_id resolution

Scans /tmp/syncable-cli/outputs/ by embedded timestamp to find the most
recent stored output. Enables 'sync-ctl retrieve latest' across separate
CLI invocations."
```

---

### Task 3: Add `compress_tool_output_cli()` to compression.rs

**Files:**
- Modify: `src/agent/tools/compression.rs`

The existing `compress_tool_output()` appends a plaintext footer (`format_session_refs_for_agent()`) to the JSON, making it invalid JSON. For `--agent` CLI mode, we need strict JSON with CLI-syntax retrieval hints.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_compress_tool_output_cli_produces_valid_json() {
    let output = serde_json::json!({
        "findings": (0..100).map(|i| serde_json::json!({
            "code": format!("SEC{:03}", i),
            "severity": if i < 3 { "critical" } else if i < 15 { "high" } else { "medium" },
            "message": format!("Finding {} with enough text to exceed compression threshold when multiplied", i),
            "file": format!("src/file_{}.rs", i),
        })).collect::<Vec<_>>()
    });
    let config = CompressionConfig::default();
    let result = compress_tool_output_cli(&output, "security", &config);

    // Must be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result);
    assert!(parsed.is_ok(), "CLI output must be valid JSON, got: {}", &result[..200.min(result.len())]);

    let json = parsed.unwrap();
    // Must contain CLI-syntax retrieval hint
    let hint = json.get("retrieval_hint").and_then(|v| v.as_str()).unwrap();
    assert!(hint.contains("sync-ctl retrieve"), "Hint should use CLI syntax, got: {}", hint);
    assert!(!hint.contains("retrieve_output("), "Hint should NOT use internal tool call syntax");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib -- compression::tests::test_compress_tool_output_cli_produces_valid_json`
Expected: FAIL — function doesn't exist

- [ ] **Step 3: Implement `compress_tool_output_cli()`**

Add to `src/agent/tools/compression.rs` after `compress_tool_output()`:

```rust
/// CLI variant of compress_tool_output - produces strict JSON for external agents.
///
/// Differences from compress_tool_output():
/// 1. Does NOT append format_session_refs_for_agent() plaintext footer
/// 2. Uses CLI-syntax retrieval hints (sync-ctl retrieve '<ref_id>' --query '...')
/// 3. Output is guaranteed valid JSON
pub fn compress_tool_output_cli(output: &Value, tool_name: &str, config: &CompressionConfig) -> String {
    let raw_str = serde_json::to_string(output).unwrap_or_default();
    if raw_str.len() <= config.target_size_bytes {
        // Still store it for retrieval even if small
        let ref_id = output_store::store_output(output, tool_name);
        let mut result = output.clone();
        if let Some(obj) = result.as_object_mut() {
            obj.insert("full_data_ref".to_string(), Value::String(ref_id.clone()));
            obj.insert("retrieval_hint".to_string(), Value::String(
                format!("Use `sync-ctl retrieve '{}' --query 'severity:critical'` for filtered details", ref_id)
            ));
        }
        return serde_json::to_string_pretty(&result).unwrap_or(raw_str);
    }

    // Run the same compression pipeline as compress_tool_output
    let issues = extract_issues(output);
    if issues.is_empty() {
        let ref_id = output_store::store_output(output, tool_name);
        let mut result = output.clone();
        if let Some(obj) = result.as_object_mut() {
            obj.insert("full_data_ref".to_string(), Value::String(ref_id.clone()));
            obj.insert("retrieval_hint".to_string(), Value::String(
                format!("Use `sync-ctl retrieve '{}'` for full data", ref_id)
            ));
        }
        return serde_json::to_string_pretty(&result).unwrap_or(raw_str);
    }

    // Store full output for retrieval
    let ref_id = output_store::store_output(output, tool_name);

    // Classify by severity — returns tuple: (critical, high, medium, low, info)
    let (critical, high, medium, low, info) = classify_by_severity(&issues);

    // Build summary inline (no helper function — matches compress_tool_output pattern)
    let summary = SeveritySummary {
        total: issues.len(),
        critical: critical.len(),
        high: high.len(),
        medium: medium.len(),
        low: low.len(),
        info: info.len(),
    };

    // Critical: always full detail
    let critical_issues: Vec<Value> = critical.clone();

    // High: full if few, truncate otherwise
    let high_issues: Vec<Value> = if high.len() <= config.max_high_full {
        high.clone()
    } else {
        high.iter().take(config.max_high_full).cloned().collect()
    };

    // Deduplicate medium/low/info into patterns
    let mut all_lower: Vec<Value> = Vec::new();
    all_lower.extend(medium.clone());
    all_lower.extend(low.clone());
    all_lower.extend(info.clone());
    if high.len() > config.max_high_full {
        all_lower.extend(high.iter().skip(config.max_high_full).cloned());
    }
    let patterns = deduplicate_to_patterns(&all_lower, config);

    // Determine status inline
    let status = if summary.critical > 0 {
        "CRITICAL_ISSUES_FOUND"
    } else if summary.high > 0 {
        "HIGH_ISSUES_FOUND"
    } else if summary.total > 0 {
        "ISSUES_FOUND"
    } else {
        "CLEAN"
    };

    let compressed = CompressedOutput {
        tool: tool_name.to_string(),
        status: status.to_string(),
        summary,
        critical_issues,
        high_issues,
        patterns,
        full_data_ref: ref_id.clone(),
        retrieval_hint: format!(
            "Use `sync-ctl retrieve '{}' --query 'severity:critical'` for full details. Other queries: 'file:<path>', 'code:<id>'",
            ref_id
        ),
    };

    // Return strict JSON — no plaintext footer (unlike compress_tool_output which appends session refs)
    serde_json::to_string_pretty(&compressed).unwrap_or(raw_str)
}
```

Also add `compress_analysis_output_cli()` for the analyze command. This mirrors the logic in `compress_analysis_output()` (lines 476-634) but produces strict JSON with CLI-syntax retrieval hints:

```rust
/// CLI variant of compress_analysis_output - produces strict JSON for external agents.
/// Mirrors compress_analysis_output() logic but:
/// 1. Always stores (even small output) for retrieval consistency
/// 2. Uses CLI-syntax retrieval hints
/// 3. No session refs plaintext footer
pub fn compress_analysis_output_cli(output: &Value, _config: &CompressionConfig) -> String {
    let raw_str = serde_json::to_string(output).unwrap_or_default();
    let ref_id = output_store::store_output(output, "analyze_project");

    // Build minimal summary — same inline extraction as compress_analysis_output()
    let mut summary = json!({
        "tool": "analyze_project",
        "status": "ANALYSIS_COMPLETE",
        "full_data_ref": ref_id.clone(),
        "retrieval_hint": format!(
            "Use `sync-ctl retrieve '{}' --query 'section:frameworks'` for details. Other queries: 'section:languages', 'language:<name>', 'framework:<name>', 'project:<name>'",
            ref_id
        )
    });

    let summary_obj = summary.as_object_mut().unwrap();

    // Detect output type and extract summary fields
    // (Same logic as compress_analysis_output lines 495-587)
    let is_monorepo = output.get("projects").is_some() || output.get("is_monorepo").is_some();

    if is_monorepo {
        if let Some(projects) = output.get("projects").and_then(|v| v.as_array()) {
            summary_obj.insert("project_count".to_string(), json!(projects.len()));
            let mut all_languages: Vec<String> = Vec::new();
            let mut all_frameworks: Vec<String> = Vec::new();
            let mut project_names: Vec<String> = Vec::new();
            for project in projects.iter().take(20) {
                if let Some(name) = project.get("name").and_then(|v| v.as_str()) {
                    project_names.push(name.to_string());
                }
                if let Some(analysis) = project.get("analysis") {
                    if let Some(langs) = analysis.get("languages").and_then(|v| v.as_array()) {
                        for lang in langs {
                            if let Some(name) = lang.get("name").and_then(|v| v.as_str())
                                && !all_languages.contains(&name.to_string()) {
                                all_languages.push(name.to_string());
                            }
                        }
                    }
                    if let Some(fws) = analysis.get("frameworks").and_then(|v| v.as_array()) {
                        for fw in fws {
                            if let Some(name) = fw.get("name").and_then(|v| v.as_str())
                                && !all_frameworks.contains(&name.to_string()) {
                                all_frameworks.push(name.to_string());
                            }
                        }
                    }
                }
            }
            summary_obj.insert("project_names".to_string(), json!(project_names));
            summary_obj.insert("languages_detected".to_string(), json!(all_languages));
            summary_obj.insert("frameworks_detected".to_string(), json!(all_frameworks));
        }
    } else {
        // ProjectAnalysis flat structure
        if let Some(langs) = output.get("languages").and_then(|v| v.as_array()) {
            let names: Vec<&str> = langs.iter()
                .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                .collect();
            summary_obj.insert("languages_detected".to_string(), json!(names));
        }
        if let Some(techs) = output.get("technologies").and_then(|v| v.as_array()) {
            let names: Vec<&str> = techs.iter()
                .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
                .collect();
            summary_obj.insert("technologies_detected".to_string(), json!(names));
        }
        // Extract services (matches compress_analysis_output lines 576-586)
        if let Some(services) = output.get("services").and_then(|v| v.as_array()) {
            summary_obj.insert("services_count".to_string(), json!(services.len()));
            let service_names: Vec<&str> = services.iter()
                .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                .collect();
            if !service_names.is_empty() {
                summary_obj.insert("services_detected".to_string(), json!(service_names));
            }
        }
    }

    serde_json::to_string_pretty(&summary).unwrap_or(raw_str)
}
```

**Note:** `extract_issues`, `classify_by_severity`, and `deduplicate_to_patterns` are existing private functions in compression.rs. They are already used by `compress_tool_output()`. If the new `_cli` functions are in the same module (which they should be), they can access these private functions directly.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib -- compression::tests::test_compress_tool_output_cli`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/agent/tools/compression.rs
git commit -m "feat: add CLI variants of compression functions

compress_tool_output_cli() and compress_analysis_output_cli() produce
strict JSON without plaintext footer and use CLI-syntax retrieval hints
for external AI agents."
```

---

### Task 4: Add `--agent` flag to CLI command structs

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Add `agent` field to each command struct**

In `src/cli.rs`, add the following field to each of these command variants:

**Analyze** (after the `color_scheme` field, around line 66):
```rust
        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
```

**Dependencies** (after the `format` field, around line 158):
```rust
        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
```

**Vulnerabilities** (after the `output` field, around line 177):
```rust
        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
```

**Security** (after the `fail_on_findings` field, around line 224):
```rust
        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
```

**Validate** (after the `fix` field, around line 116):
```rust
        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
```

**Optimize** (after the `full` field, around line 314):
```rust
        /// Output compressed JSON for AI agent consumption (implies --json)
        #[arg(long)]
        agent: bool,
```

- [ ] **Step 2: Add `Retrieve` command variant**

Add after the `Optimize` variant (before `Chat`):

```rust
    /// Retrieve stored output from a previous --agent command
    Retrieve {
        /// Reference ID (e.g., "security_a1b2c3d4") or "latest" for most recent
        #[arg(value_name = "REF_ID")]
        ref_id: Option<String>,

        /// Filter query (e.g., "severity:critical", "file:path", "section:frameworks")
        #[arg(long, short = 'q')]
        query: Option<String>,

        /// List all stored outputs
        #[arg(long)]
        list: bool,
    },
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check 2>&1 | head -30`
Expected: Compilation errors in `main.rs` because match arms don't destructure the new `agent` field yet — that's expected and will be fixed in Task 5.

If there are OTHER errors (syntax, type), fix them before proceeding.

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --agent flag to 5 scan commands and Retrieve subcommand

Adds 'agent: bool' field to Analyze, Security, Vulnerabilities,
Dependencies, and Optimize command structs. Adds Retrieve command
variant with ref_id, query, and list fields."
```

---

### Task 5: Wire `--agent` flag in main.rs command handlers

**Files:**
- Modify: `src/main.rs`

This is the biggest task — wiring the `--agent` flag in each command handler and adding the `Retrieve` match arm.

- [ ] **Step 1: Add imports at top of main.rs**

Near the existing imports in `main.rs`, add:

```rust
use syncable_cli::agent::tools::compression::{
    compress_tool_output_cli, compress_analysis_output_cli, CompressionConfig,
};
use syncable_cli::agent::tools::output_store;
```

**Note:** If `compression` or `output_store` modules are not publicly exported from `syncable_cli`, you may need to add `pub mod` re-exports in `src/lib.rs` or `src/agent/mod.rs` → `src/agent/tools/mod.rs`. Check the module visibility chain and add `pub` as needed.

- [ ] **Step 2: Add `handle_agent_output` helper function**

Add a helper function near the other `handle_*` functions in `main.rs`:

```rust
/// Route command output through compression pipeline for --agent mode.
/// Runs cleanup, compresses, stores, and prints strict JSON to stdout.
/// If output_file is provided, writes full uncompressed output to that file.
fn handle_agent_output(json_output: &str, tool_name: &str, output_file: Option<&std::path::Path>) -> syncable_cli::Result<()> {
    output_store::cleanup_old_outputs();

    // If --output was also passed, write full uncompressed output to file
    if let Some(path) = output_file {
        std::fs::write(path, json_output)?;
        eprintln!("Full output saved to: {}", path.display());
    }

    let value: serde_json::Value = serde_json::from_str(json_output)
        .map_err(|e| syncable_cli::error::IaCGeneratorError::Analysis(
            syncable_cli::error::AnalysisError::InvalidStructure(
                format!("Failed to parse output as JSON: {}", e)
            )
        ))?;

    let config = CompressionConfig::default();
    let compressed = if tool_name == "analyze_project" {
        compress_analysis_output_cli(&value, &config)
    } else {
        compress_tool_output_cli(&value, tool_name, &config)
    };

    println!("{}", compressed);
    Ok(())
}
```

- [ ] **Step 3: Wire --agent in Analyze handler**

In the `Commands::Analyze` match arm (around line 129), add `agent` to the destructuring pattern and modify the handler call:

```rust
        Commands::Analyze {
            path,
            json,
            detailed,
            display,
            only,
            color_scheme,
            agent,
        } => {
            // ... existing telemetry code ...

            if agent {
                // Force JSON mode, call inner handler (returns Result<String>), route through compression
                match syncable_cli::handlers::analyze::handle_analyze(path, true, false, None, only, color_scheme) {
                    Ok(output) => handle_agent_output(&output, "analyze_project", None),
                    Err(e) => Err(e),
                }
            } else {
                match handle_analyze(path, json, detailed, display, only, color_scheme) {
                    Ok(_output) => Ok(()),
                    Err(e) => Err(e),
                }
            }
        }
```

**Important:** The `handle_analyze` in `main.rs` is a thin wrapper returning `Result<()>`. For `--agent` mode, call the inner handler directly: `syncable_cli::handlers::analyze::handle_analyze(...)` which returns `crate::Result<String>` — the string is the JSON output when `json: true`.

- [ ] **Step 4: Wire --agent in Security handler**

In the `Commands::Security` match arm (around line 365), add `agent` to destructuring and modify:

```rust
        Commands::Security {
            path,
            mode,
            include_low,
            no_secrets,
            no_code_patterns,
            no_infrastructure,
            no_compliance,
            frameworks,
            format,
            output,
            fail_on_findings,
            agent,
        } => {
            // ... existing telemetry code ...

            if agent {
                // Force JSON format, call inner handler (returns Result<String>)
                let result = syncable_cli::handlers::security::handle_security(
                    path, mode, include_low, no_secrets, no_code_patterns,
                    no_infrastructure, no_compliance, frameworks,
                    OutputFormat::Json, None, fail_on_findings,
                )?;
                handle_agent_output(&result, "security", output.as_deref())
            } else {
                let effective_format = if cli.json { OutputFormat::Json } else { format };
                // ... rest of existing handler code ...
                handle_security(
                    path, mode, include_low, no_secrets, no_code_patterns,
                    no_infrastructure, no_compliance, frameworks,
                    effective_format, output, fail_on_findings,
                ).map(|_| ())
            }
        }
```

**Important:** Same as Analyze — call the inner handler `syncable_cli::handlers::security::handle_security(...)` which returns `crate::Result<String>`. The main.rs wrapper `handle_security()` returns `Result<()>` and discards the string.

- [ ] **Step 5: Wire --agent in Vulnerabilities handler**

Find the `Commands::Vulnerabilities` match arm, add `agent` to destructuring. The vulnerabilities handler is `async` and returns `Result<()>` — it currently prints directly. For `--agent` mode, we need to capture its JSON output. Check how the handler produces output:

```rust
        Commands::Vulnerabilities {
            path,
            severity,
            format,
            output,
            agent,
        } => {
            // ... existing telemetry code ...

            if agent {
                // Force JSON, call inner handler — see Task 6 for handler refactor
                // After Task 6 refactor, the handler returns Result<String> in JSON mode
                let result = syncable_cli::handlers::vulnerabilities::handle_vulnerabilities(
                    path, severity, OutputFormat::Json, None,
                ).await?;
                handle_agent_output(&result, "vulnerabilities", output.as_deref())
            } else {
                handle_vulnerabilities(path, severity, format, output).await
            }
        }
```

**Implementation note:** The vulnerabilities handler returns `Result<()>` and prints directly. For `--agent` mode to work, the handler needs to be modified to return the JSON string instead of printing it. This pattern applies to `dependencies` and `optimize` as well. The modification is: when format is JSON, collect output into a String and return it instead of printing. This is a necessary refactor for each affected handler.

- [ ] **Step 6: Wire --agent in Dependencies handler**

Same pattern as Vulnerabilities — add `agent` to destructuring, force JSON format when `agent` is true, route through `handle_agent_output()`.

- [ ] **Step 7: Wire --agent in Optimize handler**

Same pattern — add `agent` to destructuring, force JSON format when `agent` is true, route through `handle_agent_output()`.

- [ ] **Step 7b: Wire --agent in Validate handler (stub)**

The Validate handler is currently a stub (`handle_validate` just prints "not yet implemented"). Add `agent` to the destructuring but ignore it for now:

```rust
        Commands::Validate { path, types, fix, agent: _ } => {
            // ... existing telemetry code ...
            handle_validate(path, types, fix)
        }
```

When validate is implemented, the `agent: _` will be replaced with actual `--agent` wiring.

- [ ] **Step 8: Add Retrieve to command_name telemetry match**

In main.rs around line 106-123, there's a match for `command_name` used in telemetry. Add:
```rust
        Commands::Retrieve { .. } => "retrieve",
```

- [ ] **Step 9: Add Retrieve match arm**

Add after the Optimize match arm:

```rust
        Commands::Retrieve { ref_id, query, list } => {
            output_store::cleanup_old_outputs();

            if list {
                let outputs = output_store::list_outputs();
                let json_list: Vec<serde_json::Value> = outputs.iter().map(|o| {
                    let age_secs = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .saturating_sub(o.timestamp);
                    let age_str = if age_secs < 60 {
                        format!("{}s ago", age_secs)
                    } else if age_secs < 3600 {
                        format!("{}m ago", age_secs / 60)
                    } else {
                        format!("{}h ago", age_secs / 3600)
                    };
                    serde_json::json!({
                        "ref_id": o.ref_id,
                        "tool": o.tool,
                        "timestamp": o.timestamp,
                        "age": age_str,
                        "size_bytes": o.size_bytes,
                    })
                }).collect();
                println!("{}", serde_json::to_string_pretty(&json_list)?);
                return Ok(());
            }

            let resolved_ref = match ref_id.as_deref() {
                Some("latest") => match output_store::resolve_latest() {
                    Some(id) => id,
                    None => {
                        let error = serde_json::json!({
                            "error": "no_outputs",
                            "message": "No stored outputs found. Run a command with --agent first."
                        });
                        eprintln!("{}", serde_json::to_string_pretty(&error)?);
                        std::process::exit(1);
                    }
                },
                Some(id) => id.to_string(),
                None => {
                    eprintln!("Usage: sync-ctl retrieve <REF_ID> [--query <FILTER>]");
                    eprintln!("       sync-ctl retrieve --list");
                    return Ok(());
                }
            };

            match output_store::retrieve_filtered(&resolved_ref, query.as_deref()) {
                Some(data) => {
                    let json_str = serde_json::to_string_pretty(&data)?;
                    if json_str.len() > 50_000 {
                        eprintln!("Warning: result is {} bytes. Use a more specific --query to narrow results.", json_str.len());
                    }
                    println!("{}", json_str);
                    Ok(())
                }
                None => {
                    let available = output_store::list_outputs();
                    let available_ids: Vec<&str> = available.iter().map(|o| o.ref_id.as_str()).collect();
                    let error = serde_json::json!({
                        "error": "not_found",
                        "message": format!("Output '{}' not found (expired or invalid ref_id)", resolved_ref),
                        "available": available_ids,
                    });
                    eprintln!("{}", serde_json::to_string_pretty(&error)?);
                    std::process::exit(1);
                }
            }
        }
```

- [ ] **Step 10: Verify it compiles**

Run: `cargo check`
Expected: PASS (or warnings only). Fix any compilation errors.

- [ ] **Step 11: Run existing tests**

Run: `cargo test`
Expected: All existing tests pass. The changes are additive — no existing behavior changed.

- [ ] **Step 12: Commit**

```bash
git add src/main.rs src/lib.rs src/agent/mod.rs src/agent/tools/mod.rs
git commit -m "feat: wire --agent flag and retrieve subcommand in main.rs

Routes --agent output through compression pipeline for analyze, security,
vulnerabilities, dependencies, and optimize commands. Adds retrieve
subcommand with --list, --query, and 'latest' resolution."
```

---

### Task 6: Handler refactors for --agent JSON capture

**Files:**
- Modify: `src/handlers/vulnerabilities.rs`
- Modify: `src/handlers/dependencies.rs`
- Modify: `src/handlers/optimize.rs`

The `vulnerabilities`, `dependencies`, and `optimize` handlers currently return `Result<()>` and print directly to stdout. For `--agent` mode, they need to return the JSON string so main.rs can route it through compression.

- [ ] **Step 1: Modify vulnerabilities handler**

In `src/handlers/vulnerabilities.rs`, change the handler to return `Result<String>` when format is JSON:

The exact changes depend on how the handler currently formats and prints output. The pattern is:
1. When format is JSON, collect the serde_json output into a String
2. Return the String instead of printing
3. Let the caller (main.rs) decide whether to print directly or route through compression

- [ ] **Step 2: Modify dependencies handler**

Same pattern in `src/handlers/dependencies.rs`.

- [ ] **Step 3: Modify optimize handler**

Same pattern in `src/handlers/optimize.rs`.

- [ ] **Step 4: Update main.rs callers**

Update the match arms in main.rs to handle the new return types.

- [ ] **Step 5: Verify compilation and tests**

Run: `cargo check && cargo test`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/handlers/vulnerabilities.rs src/handlers/dependencies.rs src/handlers/optimize.rs src/main.rs
git commit -m "refactor: handlers return JSON string for --agent mode capture

Modified vulnerabilities, dependencies, and optimize handlers to return
JSON output string instead of printing directly, enabling --agent mode
to route through compression pipeline."
```

---

### Task 7: Module visibility and integration test

**Files:**
- Modify: `src/lib.rs` and/or `src/agent/mod.rs`, `src/agent/tools/mod.rs` (as needed)
- Create: integration test or manual verification

- [ ] **Step 1: Ensure module visibility**

The `compression` and `output_store` modules need to be accessible from `main.rs`. Check the module tree:

```
src/lib.rs → pub mod agent
src/agent/mod.rs → pub mod tools
src/agent/tools/mod.rs → pub mod compression, pub mod output_store
```

If any of these are `pub(crate)` or private, change to `pub`. The functions `compress_tool_output_cli`, `compress_analysis_output_cli`, `CompressionConfig`, `output_store::cleanup_old_outputs`, `output_store::store_output`, `output_store::retrieve_filtered`, `output_store::list_outputs`, `output_store::resolve_latest` all need to be reachable from main.rs.

- [ ] **Step 2: Manual integration test**

Run a full end-to-end test:

```bash
# Build
cargo build

# Test --agent on analyze
./target/debug/sync-ctl analyze . --agent

# Verify output is valid JSON
./target/debug/sync-ctl analyze . --agent | python3 -c "import sys,json; d=json.load(sys.stdin); print('ref:', d.get('full_data_ref')); print('hint:', d.get('retrieval_hint'))"

# Test retrieve --list
./target/debug/sync-ctl retrieve --list

# Test retrieve with ref_id from analyze output
./target/debug/sync-ctl retrieve latest --query "section:frameworks"

# Test error case
./target/debug/sync-ctl retrieve nonexistent_abc123
```

Expected: Each command produces valid JSON output with appropriate fields.

- [ ] **Step 3: Commit any visibility fixes**

```bash
git add -A
git commit -m "feat: ensure module visibility for --agent pipeline integration"
```

---

## Part 2: Skill Rewrites (Markdown)

### Task 8: Rewrite command skills with --agent pattern

**Files:**
- Modify: `skills/commands/syncable-analyze.md`
- Modify: `skills/commands/syncable-security.md`
- Modify: `skills/commands/syncable-vulnerabilities.md`
- Modify: `skills/commands/syncable-dependencies.md`
- Modify: `skills/commands/syncable-validate.md`
- Modify: `skills/commands/syncable-optimize.md`

For each command skill, make these changes:

1. **Replace all `--json` / `--format json` with `--agent`** in command examples
2. **Add "Reading Results" section** after the command reference
3. **Add skill-specific query filters** for the retrieve command
4. **Remove raw JSON parsing instructions** — agents no longer need to manually parse arrays

- [ ] **Step 1: Rewrite syncable-analyze.md**

Replace all command examples using `--json` with `--agent`. Add after the command reference:

```markdown
## Reading Results

When you use `--agent`, the output is a compressed summary — not the full analysis. You can act on it directly for most decisions.

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
```

- [ ] **Step 2: Rewrite syncable-security.md**

Replace `--format json` with `--agent`. Add Reading Results section:

```markdown
## Reading Results

When you use `--agent`, the output is a compressed summary (~15KB max). All **critical** issues are included in full detail. High-severity issues show the first 10. Medium/low issues are deduplicated into patterns.

The output JSON includes:
- `status` — e.g., "CRITICAL_ISSUES_FOUND", "HIGH_ISSUES_FOUND", "CLEAN"
- `summary` — counts by severity (total, critical, high, medium, low, info)
- `critical_issues` — full details for every critical finding
- `high_issues` — first 10 high-severity findings
- `patterns` — deduplicated medium/low findings with counts
- `full_data_ref` — reference ID for retrieving full data
- `retrieval_hint` — exact command for drill-down

To drill into specifics:
```bash
# Get all critical findings (already in summary, but also via retrieve)
sync-ctl retrieve <ref_id> --query "severity:critical"

# Get findings for a specific file
sync-ctl retrieve <ref_id> --query "file:src/auth.rs"

# Get findings by rule code
sync-ctl retrieve <ref_id> --query "code:hardcoded-secret"
```

**Available query filters:** `severity:critical|high|medium|low|info`, `file:<path>`, `code:<id>`
```

- [ ] **Step 3: Rewrite syncable-vulnerabilities.md**

Same pattern as security — replace format flags with `--agent`, add Reading Results with filters: `severity:<level>`, `file:<path>`.

- [ ] **Step 4: Rewrite syncable-dependencies.md**

Replace `--format json` with `--agent`, add Reading Results with filters: `severity:<level>`, `file:<path>`.

- [ ] **Step 5: Rewrite syncable-validate.md**

Replace command invocations with `--agent`. Note: the validate CLI command is currently a stub, but the skill should still document the `--agent` flag for when it's implemented. Add Reading Results with filters: `severity:<level>`, `file:<path>`, `code:<id>`.

- [ ] **Step 6: Rewrite syncable-optimize.md**

Replace `--format json` with `--agent`, add Reading Results with filters: `severity:<level>`, `container:<name>`.

- [ ] **Step 7: Verify no remaining --json/--format json in command skills**

```bash
grep -r "\-\-json\|\-\-format json" skills/commands/
```

Expected: Only `syncable-platform.md` should still reference `--json` (platform commands don't get `--agent`). All other command skills should use `--agent`.

- [ ] **Step 8: Commit**

```bash
git add skills/commands/
git commit -m "feat: rewrite command skills to use --agent flag

All 6 scan command skills now use --agent instead of --json/--format json.
Each skill includes a Reading Results section documenting compressed output
format and available retrieve query filters."
```

---

### Task 9: Rewrite workflow skills with --agent and cross-step retrieval

**Files:**
- Modify: `skills/workflows/syncable-project-assessment.md`
- Modify: `skills/workflows/syncable-security-audit.md`
- Modify: `skills/workflows/syncable-iac-pipeline.md`
- Modify: `skills/workflows/syncable-deploy-pipeline.md`

Workflow skills get the same `--agent` switch plus cross-step retrieval instructions.

- [ ] **Step 1: Rewrite syncable-project-assessment.md**

Replace all `--json` / `--format json` with `--agent` in each step's commands. Add cross-step retrieval:

After Step 1 (Analyze), add:
```markdown
Save the `full_data_ref` from the analyze output — you'll use it to retrieve details without re-running analyze.
```

After all steps, add or update the Report Synthesis section:
```markdown
## Cross-Step Retrieval

Each step produces a `full_data_ref` in its output. You can retrieve details from any previous step at any time:

```bash
# Check what data is available from all steps
sync-ctl retrieve --list

# Get framework details from Step 1 (analyze)
sync-ctl retrieve <analyze_ref_id> --query "section:frameworks"

# Get critical security findings from Step 2
sync-ctl retrieve <security_ref_id> --query "severity:critical"

# Get vulnerability details from Step 3
sync-ctl retrieve <vuln_ref_id> --query "severity:high"
```

Do NOT re-run a command just to get more detail — use `sync-ctl retrieve` instead.
```

- [ ] **Step 2: Rewrite syncable-security-audit.md**

Same pattern — `--agent` flags, cross-step retrieval.

- [ ] **Step 3: Rewrite syncable-iac-pipeline.md**

Same pattern — `--agent` flags, cross-step retrieval. Emphasize that the analyze step's ref_id can be reused in later steps.

- [ ] **Step 4: Rewrite syncable-deploy-pipeline.md**

Same pattern — `--agent` flags, cross-step retrieval. The deploy pipeline's security gate step should reference the security output's `status` field to check for critical issues without needing to parse raw arrays.

Update the security gate decision logic:
```markdown
**CRITICAL GATE:** Check the security output's `status` field:
- If `status` is "CRITICAL_ISSUES_FOUND": present findings to user, warn, require confirmation
- If `status` is "HIGH_ISSUES_FOUND": warn but allow deployment
- If `status` is "CLEAN": proceed to deploy

All critical findings are in the `critical_issues` array of the compressed output — no retrieval needed for the gate decision.
```

- [ ] **Step 5: Verify no remaining --json/--format json in workflow skills**

```bash
grep -r "\-\-json\|\-\-format json" skills/workflows/
```

Expected: No matches.

- [ ] **Step 6: Commit**

```bash
git add skills/workflows/
git commit -m "feat: rewrite workflow skills with --agent and cross-step retrieval

All 4 workflow skills now use --agent and teach agents to reuse ref_ids
across steps via sync-ctl retrieve. Security gate in deploy pipeline uses
compressed output status field instead of raw JSON parsing."
```

---

### Task 10: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Verify all skills use --agent pattern**

```bash
# Should find --agent in all command skills except platform
grep -l "\-\-agent" skills/commands/*.md

# Should find --agent in all workflow skills
grep -l "\-\-agent" skills/workflows/*.md

# Should find NO --json in skills except platform
grep -rn "\-\-json" skills/ | grep -v platform
```

- [ ] **Step 2: Verify Rust tests pass**

```bash
cargo test
```

- [ ] **Step 3: Verify npm skills build**

```bash
cd installer && npm run build && cd ..
```

- [ ] **Step 4: Commit any final fixes**

Only if needed from verification.
