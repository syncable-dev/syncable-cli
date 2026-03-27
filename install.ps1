# PowerShell Installation Script for Syncable CLI on Windows
# Usage: powershell -ExecutionPolicy Bypass -File install.ps1

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.local\bin",
    [switch]$Force = $false,
    [switch]$Help = $false
)

# Color functions for better output
function Write-Success {
    param([string]$Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host "ℹ️  $Message" -ForegroundColor Blue
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠️  $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "❌ $Message" -ForegroundColor Red
}

function Write-Step {
    param([string]$Message)
    Write-Host "🔧 $Message" -ForegroundColor Cyan
}

# Help function
function Show-Help {
    Write-Host @"
Syncable CLI Installer for Windows

Usage: powershell -ExecutionPolicy Bypass -File install.ps1 [OPTIONS]

Options:
  -Version <version>     Install specific version (default: latest)
  -InstallDir <path>     Installation directory (default: %USERPROFILE%\.local\bin)
  -Force                 Force installation even if already installed
  -Help                  Show this help message

Examples:
  .\install.ps1                          # Install latest version
  .\install.ps1 -Version "0.9.0"         # Install specific version
  .\install.ps1 -Force                   # Force reinstall
  .\install.ps1 -InstallDir "C:\tools"   # Custom installation directory

"@
}

# Check if help is requested
if ($Help) {
    Show-Help
    exit 0
}

Write-Host @"
🚀 Syncable CLI Installer for Windows
====================================
"@ -ForegroundColor Magenta

# Check if running as administrator (optional, for system-wide installs)
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")
if ($isAdmin) {
    Write-Info "Running as Administrator - can install system-wide"
} else {
    Write-Info "Running as regular user - installing to user directory"
}

# Check if cargo is available
Write-Step "Checking for Rust/Cargo installation..."
try {
    $cargoVersion = cargo --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Found Cargo: $cargoVersion"
        $hasRust = $true
    } else {
        $hasRust = $false
    }
} catch {
    $hasRust = $false
}

if (-not $hasRust) {
    Write-Warning "Rust/Cargo not found. Installing via cargo is not available."
    Write-Info "To install Rust, visit: https://rustup.rs/"
    Write-Info "Or download pre-built binaries from: https://github.com/syncable-dev/syncable-cli/releases"
    
    # Offer to open browser
    $response = Read-Host "Would you like to open the Rust installation page? (y/N)"
    if ($response -eq "y" -or $response -eq "Y") {
        Start-Process "https://rustup.rs/"
    }
    exit 1
}

# Check if sync-ctl is already installed
Write-Step "Checking for existing installation..."
try {
    $existingVersion = sync-ctl --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Info "Found existing installation: $existingVersion"
        if (-not $Force) {
            $response = Read-Host "sync-ctl is already installed. Reinstall? (y/N)"
            if ($response -ne "y" -and $response -ne "Y") {
                Write-Info "Installation cancelled."
                exit 0
            }
        }
    }
} catch {
    Write-Info "No existing installation found."
}

# Install via cargo
Write-Step "Installing Syncable CLI via Cargo..."
Write-Info "This may take a few minutes..."

try {
    if ($Version -eq "latest") {
        Write-Info "Installing latest version from crates.io..."
        $installResult = cargo install syncable-cli 2>&1
    } else {
        Write-Info "Installing version $Version from crates.io..."
        $installResult = cargo install syncable-cli --version $Version 2>&1
    }
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Syncable CLI installed successfully!"
    } else {
        Write-Error "Installation failed. Cargo output:"
        Write-Host $installResult -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Error "Installation failed: $_"
    exit 1
}

# Verify installation
Write-Step "Verifying installation..."
try {
    $version = sync-ctl --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Installation verified: $version"
    } else {
        Write-Warning "Installation may have issues. sync-ctl command not found."
    }
} catch {
    Write-Warning "Could not verify installation."
}

# Check PATH
Write-Step "Checking PATH configuration..."
$cargoPath = "$env:USERPROFILE\.cargo\bin"
$currentPath = $env:PATH
if ($currentPath -like "*$cargoPath*") {
    Write-Success "Cargo bin directory is already in PATH"
} else {
    Write-Warning "Cargo bin directory ($cargoPath) is not in your PATH"
    Write-Info "To add it permanently:"
    Write-Info "1. Open System Properties > Advanced > Environment Variables"
    Write-Info "2. Add '$cargoPath' to your PATH variable"
    Write-Info "3. Restart your terminal/PowerShell session"
    Write-Info ""
    Write-Info "Or run this command in an elevated PowerShell:"
    Write-Info "[Environment]::SetEnvironmentVariable('PATH', `$env:PATH + ';$cargoPath', 'User')"
    
    # Offer to add to PATH automatically
    $response = Read-Host "Would you like to add it to PATH now? (y/N)"
    if ($response -eq "y" -or $response -eq "Y") {
        try {
            [Environment]::SetEnvironmentVariable('PATH', $env:PATH + ";$cargoPath", 'User')
            $env:PATH += ";$cargoPath"  # Update current session
            Write-Success "Added to PATH. Restart PowerShell to ensure it takes effect."
        } catch {
            Write-Error "Failed to add to PATH: $_"
            Write-Info "Please add manually as described above."
        }
    }
}

# Install vulnerability scanning tools
Write-Step "Setting up vulnerability scanning tools..."
Write-Info "Installing common security tools for better analysis..."

# Install tools that work well on Windows
$tools = @(
    @{Name="cargo-audit"; Command="cargo install cargo-audit"; Check="cargo audit --version"},
    @{Name="pip-audit"; Command="pip install --user pip-audit"; Check="pip-audit --version"}
)

foreach ($tool in $tools) {
    Write-Info "Installing $($tool.Name)..."
    try {
        # Check if already installed
        $checkResult = Invoke-Expression $tool.Check 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Success "$($tool.Name) is already installed"
            continue
        }
        
        # Install the tool
        $installResult = Invoke-Expression $tool.Command 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Success "$($tool.Name) installed successfully"
        } else {
            Write-Warning "Failed to install $($tool.Name): $installResult"
        }
    } catch {
        Write-Warning "Error installing $($tool.Name): $_"
    }
}

# Additional Windows-specific tools
Write-Info "For additional security tools on Windows, consider:"
Write-Info "  • Scoop: scoop install grype"
Write-Info "  • Chocolatey: choco install grype"
Write-Info "  • Manual downloads from GitHub releases"

# Final instructions
Write-Host @"

🎉 Installation Complete!
========================
"@ -ForegroundColor Green

Write-Success "Syncable CLI is now installed and ready to use!"
Write-Info ""
Write-Info "Quick Start:"
Write-Info "  sync-ctl analyze .              # Analyze current directory"
Write-Info "  sync-ctl generate --all .       # Generate all IaC files"
Write-Info "  sync-ctl security .             # Run security analysis"
Write-Info "  sync-ctl tools status           # Check security tools"
Write-Info ""
Write-Info "For help: sync-ctl --help"
Write-Info "Documentation: https://github.com/syncable-dev/syncable-cli"
Write-Info ""
Write-Warning "Remember to restart your PowerShell session if PATH was modified!" 