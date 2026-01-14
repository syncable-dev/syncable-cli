# Testing Protocol - Syncable CLI Agent Tools

## Overview

This protocol defines test cases for all 28 agent tools. Tests are organized by prerequisites and priority.

**Testing Method:** Interact with the agent (`./target/release/sync-ctl agent`) and use prompts to trigger specific tools.

---

## Test Categories

### Category A: Always Testable (Standalone)
Tools that work without external dependencies. Test these first.

### Category B: Require Project Context
Tools that need a codebase to analyze. Use syncable-cli itself as test project.

### Category C: Require Kubernetes
Tools that need cluster access. Skip if no cluster available.

### Category D: Require Prometheus
Tools that need metrics server. Skip if no Prometheus available.

### Category E: Require External Binaries
Tools that depend on external CLI tools (hadolint, terraform).

---

## Priority Levels

- **P0 (Critical):** Core functionality, must test
- **P1 (High):** Important features, should test
- **P2 (Medium):** Secondary features, test if time permits
- **P3 (Low):** Edge cases, optional

---

## Category A: Always Testable

### 1. `read_file` [P0]
**Category:** File I/O | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Read the file src/main.rs" | Returns file contents |
| Realistic | "Show me lines 1-50 of src/agent/mod.rs" | Returns specified line range |
| Edge | "Read the file /nonexistent/path.txt" | Returns clear error message |

**Test Prompts:**
```
"Read the file Cargo.toml"
"Show me the first 20 lines of src/main.rs"
"Read /tmp/nonexistent_file_12345.txt"
```

---

### 2. `write_file` [P0]
**Category:** File I/O | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Create a file test_output.txt with 'hello world'" | File created with content |
| Realistic | "Write a simple Dockerfile to /tmp/test/Dockerfile" | Creates file with valid content |
| Edge | "Write to /root/protected.txt" | Permission error handled |

**Test Prompts:**
```
"Create a file called /tmp/sync-test.txt containing 'test content'"
"Write a simple README to /tmp/test-readme.md"
```

**Cleanup:** Delete test files after testing.

---

### 3. `write_files` [P1]
**Category:** File I/O | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Create two files: /tmp/a.txt and /tmp/b.txt" | Both files created |
| Realistic | "Create a basic project structure with index.js and package.json in /tmp/testproj" | Multiple files created atomically |
| Edge | "Create files where one path is invalid" | Atomic failure, no partial writes |

**Test Prompts:**
```
"Create these files: /tmp/multi1.txt with 'file 1' and /tmp/multi2.txt with 'file 2'"
```

---

### 4. `list_directory` [P0]
**Category:** File I/O | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "List the contents of src/" | Returns file/directory listing |
| Realistic | "List all .rs files in src/ recursively" | Returns filtered listing |
| Edge | "List /nonexistent/directory" | Clear error message |

**Test Prompts:**
```
"List the files in the src directory"
"Show me all Rust files in src/agent/ recursively"
"List the contents of /nonexistent_dir_12345"
```

---

### 5. `shell` [P0]
**Category:** Execution | **Risk:** High

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Run 'echo hello'" | Returns "hello" with exit code 0 |
| Realistic | "Run 'cargo --version'" | Returns cargo version info |
| Edge | "Run a command that takes too long" | Timeout handling |
| Edge | "Run 'exit 1'" | Captures non-zero exit code |

**Test Prompts:**
```
"Run the command: echo 'shell test passed'"
"Execute: pwd"
"Run: sleep 5" (test timeout if configured short)
"Run: exit 42" (test exit code capture)
```

---

### 6. `plan_create` [P1]
**Category:** Planning | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Create a plan called 'test-plan' with tasks: task1, task2" | Plan file created in plans/ |
| Realistic | "Create a deployment plan with steps for build, test, deploy" | Multi-task plan created |
| Edge | "Create a plan with empty task list" | Handled gracefully |

**Test Prompts:**
```
"Create a plan called 'test-plan' with tasks: 'Step 1', 'Step 2', 'Step 3'"
```

**Cleanup:** Delete plans/test-plan.md after testing.

---

### 7. `plan_list` [P1]
**Category:** Planning | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "List all plans" | Returns plan files with status |
| Edge | "List plans when none exist" | Empty result, no error |

**Test Prompts:**
```
"Show me all plans"
"List the available plans"
```

---

### 8. `plan_next` [P2]
**Category:** Planning | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Get next task from test-plan" | Returns first pending task |
| Edge | "Get next from completed plan" | No pending tasks message |

**Test Prompts:**
```
"What's the next task in test-plan?"
```

**Prerequisite:** Create a test plan first.

---

### 9. `plan_update` [P2]
**Category:** Planning | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Mark task 1 as done in test-plan" | Task status updated |
| Edge | "Mark invalid task index" | Error handled |

**Test Prompts:**
```
"Mark the first task in test-plan as complete"
```

---

### 10. `retrieve_output` [P1]
**Category:** RAG | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Retrieve output with ref_id from previous tool" | Returns stored data |
| Edge | "Retrieve with invalid ref_id" | Clear error message |

**Test Prompts:**
```
"Retrieve the output with reference ID [use actual ref_id from previous tool]"
```

**Prerequisite:** Run a tool that produces compressed output first (kubelint, k8s_optimize).

---

### 11. `list_stored_outputs` [P1]
**Category:** RAG | **Risk:** Low

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "List stored outputs" | Returns list of ref_ids |
| Edge | "List when no outputs stored" | Empty result, no error |

**Test Prompts:**
```
"What outputs are stored?"
"List all stored outputs"
```

---

### 12. `web_fetch` [P1]
**Category:** Network | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Fetch https://example.com" | Returns page content as markdown |
| Realistic | "Fetch a GitHub README" | Returns markdown content |
| Edge | "Fetch invalid URL" | Error handled gracefully |
| Edge | "Fetch very large page" | Content truncated at limit |

**Test Prompts:**
```
"Fetch the content from https://example.com"
"Get the page at https://httpstat.us/200"
"Fetch https://invalid-domain-12345.com"
```

---

## Category B: Require Project Context

### 13. `analyze_project` [P0]
**Category:** Analysis | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Analyze this project" | Returns language, framework detection |
| Realistic | "Analyze the syncable-cli codebase" | Detects Rust, shows structure |
| Edge | "Analyze empty directory" | Handles gracefully |

**Test Prompts:**
```
"Analyze this project"
"What languages and frameworks does this codebase use?"
"Analyze the project at /tmp/empty-dir"
```

---

### 14. `security_scan` [P1]
**Category:** Security | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Run a security scan" | Returns security findings |
| Realistic | "Scan src/ for security issues" | Returns categorized findings |
| Edge | "Scan path with no code" | Empty results, no error |

**Test Prompts:**
```
"Run a security scan on this project"
"Check src/ for security vulnerabilities"
```

---

### 15. `vulnerabilities` [P1]
**Category:** Security | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Check for vulnerable dependencies" | Returns vulnerability report |
| Edge | "Check project with no dependencies" | Empty results |

**Test Prompts:**
```
"Check this project for dependency vulnerabilities"
"Are there any known CVEs in our dependencies?"
```

---

### 16. `diagnostics` [P2]
**Category:** IDE | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Get diagnostics for src/" | Returns errors/warnings |
| Realistic | "Check src/main.rs for issues" | Returns Rust diagnostics |
| Edge | "Diagnostics for non-code file" | Handled gracefully |

**Test Prompts:**
```
"Get code diagnostics for this project"
"Check src/main.rs for errors"
```

**Note:** May require IDE connection or fallback to CLI tools (cargo check).

---

### 17. `dclint` [P1]
**Category:** Linting | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Lint docker-compose.yml" | Returns lint results |
| Edge | "Lint when no compose file exists" | Clear error message |

**Test Prompts:**
```
"Lint the docker-compose file"
"Check docker-compose.yml for issues"
```

**Prerequisite:** Project needs docker-compose.yml. Create test file if needed:
```yaml
version: '3.8'
services:
  app:
    image: nginx
```

---

### 18. `kubelint` [P1]
**Category:** Linting | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Lint Kubernetes manifests" | Returns security/best practice findings |
| Edge | "Lint invalid YAML" | Parse error handled |

**Test Prompts:**
```
"Lint the Kubernetes manifests in k8s/"
"Check my K8s deployment for issues"
```

**Prerequisite:** Create test manifest:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test
  template:
    metadata:
      labels:
        app: test
    spec:
      containers:
      - name: test
        image: nginx
```

---

### 19. `helmlint` [P2]
**Category:** Linting | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Lint Helm chart" | Returns lint findings |
| Edge | "Lint directory without Chart.yaml" | Clear error |

**Test Prompts:**
```
"Lint the Helm chart in charts/myapp"
```

**Prerequisite:** Needs Helm chart structure (Chart.yaml, templates/).

---

## Category C: Require Kubernetes

### 20. `k8s_optimize` [P0]
**Category:** Kubernetes | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Optimize K8s manifests" | Returns resource recommendations |
| Realistic | "Analyze K8s deployment for waste" | Returns CPU/memory suggestions |
| Edge | "Optimize with invalid YAML" | Error handled |

**Test Prompts:**
```
"Analyze the Kubernetes manifests for optimization opportunities"
"Check K8s resources for waste"
```

**Note:** Can work with static manifests (no cluster needed) or live cluster.

---

### 21. `k8s_drift` [P2]
**Category:** Kubernetes | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Check for K8s drift" | Returns drift detection results |
| Edge | "Check when no cluster access" | Clear error about cluster |

**Test Prompts:**
```
"Check for configuration drift between manifests and cluster"
```

**Prerequisite:** kubectl configured with cluster access.

---

### 22. `k8s_costs` [P1]
**Category:** Kubernetes | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Estimate K8s costs" | Returns cost breakdown |
| Realistic | "Calculate costs for AWS us-east-1" | Returns cloud-specific costs |
| Edge | "Costs for empty manifests" | Zero/empty results |

**Test Prompts:**
```
"Estimate the Kubernetes costs for this deployment"
"Calculate K8s costs assuming AWS us-east-1"
```

---

### 23. `prometheus_discover` [P2]
**Category:** Prometheus | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Discover Prometheus in cluster" | Returns list of Prometheus services |
| Edge | "Discover when no Prometheus exists" | Empty results |

**Test Prompts:**
```
"Find Prometheus services in the cluster"
"Discover Prometheus endpoints"
```

**Prerequisite:** kubectl access to cluster with Prometheus.

---

### 24. `prometheus_connect` [P2]
**Category:** Prometheus | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Connect to Prometheus" | Establishes connection, returns URL |
| Edge | "Connect to invalid endpoint" | Connection error handled |

**Test Prompts:**
```
"Connect to Prometheus at prometheus-server in monitoring namespace"
"Connect to Prometheus at http://localhost:9090"
```

**Prerequisite:** Prometheus endpoint available.

---

## Category D: Require External Binaries

### 25. `hadolint` [P1]
**Category:** Linting | **Risk:** High (external binary)

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Lint Dockerfile" | Returns hadolint findings |
| Edge | "Lint when hadolint not installed" | Clear error about missing binary |

**Test Prompts:**
```
"Lint the Dockerfile"
"Check Dockerfile for best practices"
```

**Prerequisite:**
- `hadolint` binary installed (`brew install hadolint` or download)
- Dockerfile in project

**Test Dockerfile:**
```dockerfile
FROM ubuntu:latest
RUN apt-get update && apt-get install -y curl
```

---

### 26. `terraform_fmt` [P2]
**Category:** Terraform | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Format Terraform files" | Returns formatting results |
| Edge | "Format when terraform not installed" | Error about missing binary |

**Test Prompts:**
```
"Format the Terraform configuration"
"Run terraform fmt on the infra directory"
```

**Prerequisite:** `terraform` CLI installed.

---

### 27. `terraform_validate` [P2]
**Category:** Terraform | **Risk:** Medium

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Validate Terraform config" | Returns validation results |
| Edge | "Validate invalid HCL" | Parse/validation errors |

**Test Prompts:**
```
"Validate the Terraform configuration"
"Check if Terraform files are valid"
```

**Prerequisite:** `terraform` CLI installed, may need `terraform init` first.

---

### 28. `terraform_install` [P3]
**Category:** Terraform | **Risk:** High (downloads binary)

| Test Type | Test Case | Expected Result |
|-----------|-----------|-----------------|
| Basic | "Install Terraform" | Downloads and installs terraform |
| Edge | "Install specific version" | Installs requested version |

**Test Prompts:**
```
"Install Terraform"
"Install Terraform version 1.5.0"
```

**Warning:** This downloads and installs software. Test with caution.

---

## Test Execution Checklist

### Pre-Test Setup

1. [ ] Build the agent: `cargo build --release`
2. [ ] Navigate to syncable-cli directory
3. [ ] Create test files if needed (Dockerfile, docker-compose.yml, K8s manifest)
4. [ ] Verify external tools if testing Category E:
   - [ ] `hadolint --version`
   - [ ] `terraform --version`
   - [ ] `kubectl version`

### Test Recording Format

For each tool tested, record:

```
Tool: [name]
Test Type: [basic/realistic/edge]
Prompt Used: "[exact prompt]"
Result: [PASS/FAIL/PARTIAL]
Output: [brief description or error message]
Notes: [any observations]
```

### Post-Test Cleanup

1. [ ] Delete test files created during testing
2. [ ] Remove test plans from plans/
3. [ ] Clear any stored outputs if needed

---

## Quick Reference: Test Priority

| Priority | Tools |
|----------|-------|
| P0 (Must) | read_file, write_file, list_directory, shell, analyze_project, k8s_optimize |
| P1 (Should) | write_files, plan_create, plan_list, retrieve_output, list_stored_outputs, web_fetch, security_scan, vulnerabilities, dclint, kubelint, k8s_costs, hadolint |
| P2 (Could) | plan_next, plan_update, diagnostics, helmlint, k8s_drift, prometheus_discover, prometheus_connect, terraform_fmt, terraform_validate |
| P3 (Won't) | terraform_install |

---

*Generated: Phase 1, Plan 02 - Testing Protocol*
