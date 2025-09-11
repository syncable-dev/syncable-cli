# Contributing to Syncable CLI

Thank you for your interest in contributing to the Syncable Infrastructure-as-Code CLI! This document provides guidelines and instructions for contributing.

## 🤝 Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and constructive in all interactions.

## 🛡️ Telemetry and Privacy

Syncable CLI collects anonymous usage data to help us improve the product. This data includes:

- Command usage (which commands are run)
- System information (OS type, CLI version)
- Performance metrics (execution time, success/failure status)

We do NOT collect:
- Personal or sensitive information
- File contents or project data
- Environment variables or secrets
- Any personally identifiable information

### Opting Out of Telemetry

Users can opt out of telemetry collection through multiple methods:

1. **Command-Line Flag**: Add `--disable-telemetry` to any command
   ```bash
   sync-ctl --disable-telemetry analyze .
   ```

2. **Environment Variable**: Set `SYNCABLE_CLI_TELEMETRY=false`
   ```bash
   export SYNCABLE_CLI_TELEMETRY=false
   sync-ctl analyze .
   ```

3. **Configuration File**: Add the following to your `.syncable.toml` file
   ```toml
   [telemetry]
   enabled = false
   ```

The opt-out mechanisms follow this priority order:
1. `--disable-telemetry` CLI flag (highest priority)
2. `SYNCABLE_CLI_TELEMETRY` environment variable (medium priority)
3. `telemetry.enabled` in config file (lowest priority)

Our telemetry system is designed with user privacy in mind. All data is anonymized and collected in compliance with privacy regulations.

## 🚀 Getting Started

### Prerequisites

- Rust 1.70.0 or later
- Git
- A code editor (we recommend VS Code with rust-analyzer)

### Setting Up Development Environment

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR-USERNAME/syncable-cli.git
   cd syncable-cli
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/syncable/syncable-cli.git
   ```
4. Install development tools:
   ```bash
   rustup component add rustfmt clippy
   ```

## 📝 Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Make Your Changes

- Follow the existing code style and patterns
- Add tests for new functionality
- Update documentation as needed

### 3. Run Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### 4. Check Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check for security issues
cargo audit
```

### 5. Commit Your Changes

We follow conventional commit messages:

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Test additions or modifications
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Maintenance tasks

Example:
```bash
git commit -m "feat: add support for Ruby language detection"
```

## 🔍 Areas for Contribution

### High Priority

1. **Language Support**: Add detection for new languages (Ruby, PHP, C#)
2. **Framework Detection**: Expand framework detection patterns
3. **Security Scanning**: Integrate additional vulnerability databases
4. **Documentation**: Improve user guides and API documentation
5. **Test Coverage**: Add more unit and integration tests

### Feature Ideas

- Cloud provider integrations (AWS, GCP, Azure)
- Kubernetes manifest generation
- Interactive configuration wizard
- Performance optimizations
- New IaC output formats

## 📋 Pull Request Process

1. **Update Your Branch**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Push to Your Fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

3. **Create Pull Request**:
   - Go to the original repository on GitHub
   - Click "New Pull Request"
   - Select your branch
   - Fill out the PR template

4. **PR Requirements**:
   - Clear description of changes
   - Tests pass (`cargo test`)
   - Code is formatted (`cargo fmt`)
   - No clippy warnings (`cargo clippy`)
   - Documentation updated if needed

## 🧪 Testing Guidelines

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Test implementation
    }
}
```

### Integration Tests

Add integration tests in `tests/integration/`:

```rust
use assert_cmd::Command;

#[test]
fn test_cli_analyze() {
    let mut cmd = Command::cargo_bin("sync-ctl").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/sample_project")
        .assert()
        .success();
}
```

### Test Fixtures

Add test projects in `tests/fixtures/` with appropriate structure for testing.

## 📁 Project Structure

Key directories:

- `src/analyzer/`: Language and framework detection
- `src/generator/`: IaC file generation (Phase 2)
- `src/common/`: Shared utilities
- `templates/`: IaC templates
- `tests/`: Test suite
- `docs/`: Documentation

## 🐛 Reporting Issues

### Bug Reports

Include:
- Rust version (`rustc --version`)
- OS and version
- Steps to reproduce
- Expected vs actual behavior
- Error messages

### Feature Requests

Include:
- Use case description
- Expected behavior
- Example scenarios
- Alternative solutions considered

## 💡 Tips for Contributors

### Understanding the Codebase

1. Start with `src/main.rs` to understand the CLI structure
2. Review `src/analyzer/mod.rs` for the analysis pipeline
3. Check existing tests for usage examples

### Common Patterns

- Use `Result<T, E>` for error handling
- Implement traits for extensibility
- Use `log` crate for debugging
- Follow the builder pattern for complex structs

### Performance Considerations

- Use `rayon` for parallel processing
- Cache expensive computations
- Avoid unnecessary allocations
- Profile before optimizing

## 📞 Getting Help

- **Discord**: Join our community server
- **GitHub Discussions**: Ask questions
- **Issues**: Report bugs or request features

## 🎉 Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Given credit in relevant documentation

## 📄 License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Syncable CLI! 🚀 