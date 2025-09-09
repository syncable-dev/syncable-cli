# 🧄 Bun Integration Guide

This document covers the new Bun runtime and package manager integration in Syncable CLI.

## Overview

Syncable CLI now fully supports Bun, the all-in-one JavaScript runtime & toolkit. The integration includes:

- **Runtime Detection**: Automatically detects Bun projects via lock files, package.json configuration, and Bun-specific files
- **Vulnerability Scanning**: Uses `bun audit` to check for vulnerabilities in Bun projects  
- **Tool Installation**: Auto-installs Bun when needed across all platforms
- **Multi-Runtime Support**: Prioritizes Bun when multiple package managers are present

## How Bun Projects Are Detected

Syncable CLI uses a priority-based detection system:

### 1. Lock File Detection (Highest Priority)
```bash
# If bun.lockb exists, project is detected as Bun
bun.lockb
```

### 2. Package.json Configuration
```json
{
  "name": "my-app",
  "packageManager": "bun@1.0.0",
  "engines": {
    "bun": ">=1.0.0"
  }
}
```

### 3. Bun Configuration Files
```bash
bunfig.toml       # Bun configuration file
.bunfig.toml      # Alternative config name
```

### 4. Bun Scripts in package.json
```json
{
  "scripts": {
    "start": "bun run index.js",
    "dev": "bun --watch server.ts"
  }
}
```

## Priority Order

When multiple package managers are detected, Syncable CLI uses this priority:

1. **Bun** (bun.lockb, packageManager: "bun@*")
2. **pnpm** (pnpm-lock.yaml, packageManager: "pnpm@*") 
3. **Yarn** (yarn.lock, packageManager: "yarn@*")
4. **npm** (package-lock.json, packageManager: "npm@*")

## Vulnerability Scanning

### Automatic Runtime Detection
```bash
# Automatically detects Bun and uses 'bun audit'
sync-ctl vulnerabilities /path/to/bun-project

# Shows runtime detection in output
Runtime: Bun
Package Manager: bun
Audit Command: bun audit
```

### Example Output
```bash
$ sync-ctl vulnerabilities ./my-bun-app

🔍 Vulnerability Analysis Report
═══════════════════════════════════════════════════════════════════════

┌─ Project Information ────────────────────────────────────────────────┐
│ Runtime: Bun                                                         │
│ Package Manager: bun                                                 │  
│ Dependencies: 42 total (38 production, 4 development)               │
│ Lock File: bun.lockb                                                 │
└──────────────────────────────────────────────────────────────────────┘

┌─ Vulnerability Summary ──────────────────────────────────────────────┐
│ Total Vulnerabilities: 3                                            │
│ Critical: 1 | High: 1 | Medium: 1 | Low: 0                         │
│ Checked at: 2024-01-15 14:30:22 UTC                                 │
└──────────────────────────────────────────────────────────────────────┘
```

## Installation Integration

### Automatic Installation
If Bun is not installed but detected as the project's package manager:

```bash
$ sync-ctl vulnerabilities ./bun-project

⚙️  Bun not found but required for this project
🔧 Installing Bun automatically...

# On Windows
> powershell -c "irm bun.sh/install.ps1 | iex"

# On Unix/Linux/macOS  
> curl -fsSL https://bun.sh/install | bash

✅ Bun v1.0.3 installed successfully
🔍 Running vulnerability scan with bun audit...
```

### Manual Installation
```bash
# Check tool status
sync-ctl tools status

# Install all missing tools (including Bun if needed)
sync-ctl tools install

# Get installation guide
sync-ctl tools guide --bun
```

## Cross-Platform Support

### Windows Installation
```powershell
# PowerShell (Administrator recommended)
irm bun.sh/install.ps1 | iex

# Or via Scoop
scoop install bun
```

### Unix/Linux/macOS Installation  
```bash
# Official installer
curl -fsSL https://bun.sh/install | bash

# Homebrew (macOS)
brew install bun

# Manual download
wget https://github.com/oven-sh/bun/releases/latest/download/bun-linux-x64.zip
```

## Multi-Runtime Projects

For projects with multiple package managers, Bun takes priority:

```bash
# Project structure
my-project/
├── package.json          # Shared dependencies
├── bun.lockb             # Bun lock file (highest priority)
├── yarn.lock             # Yarn lock file  
├── package-lock.json     # npm lock file
└── pnpm-lock.yaml        # pnpm lock file

# Result: Detected as Bun project
Runtime: Bun
Package Manager: bun
Confidence: High
```

## Configuration Options

### .syncable.toml Configuration
```toml
[javascript]
# Force specific package manager
preferred_package_manager = "bun"

# Skip auto-installation  
auto_install_tools = false

[vulnerability]
# Custom audit commands
bun_audit_command = "bun audit --json"
```

### Command Line Options
```bash
# Force specific package manager for vulnerability scanning
sync-ctl vulnerabilities . --package-manager bun

# Skip missing tool installation
sync-ctl vulnerabilities . --no-install
```

## Troubleshooting

### Common Issues

1. **Bun not found in PATH**
   ```bash
   # Add Bun to PATH (Unix/Linux/macOS)
   echo 'export PATH="$HOME/.bun/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc
   
   # Windows: Add %USERPROFILE%\.bun\bin to PATH
   ```

2. **Permission issues during installation**
   ```bash
   # Run with elevated permissions or use package manager
   sudo curl -fsSL https://bun.sh/install | bash
   ```

3. **Lock file conflicts**
   ```bash
   # Clean conflicting lock files
   rm package-lock.json yarn.lock pnpm-lock.yaml
   bun install  # Recreate bun.lockb
   ```

### Debug Information
```bash
# Enable debug logging
RUST_LOG=debug sync-ctl vulnerabilities .

# View runtime detection details
sync-ctl analyze . --display detailed
```

## Best Practices

1. **Use explicit packageManager field** in package.json for clarity
2. **Remove conflicting lock files** when switching to Bun
3. **Keep bunfig.toml** for project-specific Bun configuration
4. **Use bun scripts** in package.json for consistency

## Migration from Other Package Managers

### From npm
```bash
# Remove npm artifacts
rm package-lock.json node_modules/ -rf

# Install with Bun
bun install

# Update package.json
{
  "packageManager": "bun@1.0.0"
}
```

### From Yarn
```bash
# Remove Yarn artifacts  
rm yarn.lock node_modules/ -rf

# Install with Bun
bun install

# Update scripts if needed
{
  "scripts": {
    "start": "bun run index.js"
  }
}
```

## Examples

See `examples/` directory for sample Bun projects and usage patterns.

---

For more information, see the [main documentation](../README.md) or [file an issue](https://github.com/syncable-dev/syncable-cli/issues).