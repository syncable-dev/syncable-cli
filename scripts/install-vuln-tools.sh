#!/bin/bash
# Vulnerability Scanning Tools Installation Script
# This script installs the necessary tools for vulnerability scanning across different languages

set -e

# Color codes for better output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_step() {
    echo -e "${BLUE}🔧 $1${NC}"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check tool installation
check_tool_status() {
    local tool="$1"
    local description="$2"
    
    case "$tool" in
        "cargo-audit")
            if cargo audit --version >/dev/null 2>&1; then
                print_success "$description"
                return 0
            fi
            ;;
        "npm")
            if command_exists npm; then
                print_success "$description"
                return 0
            fi
            ;;
        "pip-audit")
            if command_exists pip-audit; then
                print_success "$description"
                return 0
            fi
            ;;
        "govulncheck")
            if command_exists govulncheck || test -f "$HOME/go/bin/govulncheck"; then
                print_success "$description"
                return 0
            fi
            ;;
        "grype")
            if command_exists grype || test -f "$HOME/.local/bin/grype"; then
                print_success "$description"
                return 0
            fi
            ;;
        "safety")
            if command_exists safety; then
                print_success "$description"
                return 0
            fi
            ;;
        "bandit")
            if command_exists bandit; then
                print_success "$description"
                return 0
            fi
            ;;
        "dependency-check")
            if command_exists dependency-check || test -f "$HOME/.local/bin/dependency-check"; then
                print_success "$description"
                return 0
            fi
            ;;
    esac
    
    print_warning "$description (missing)"
    return 1
}

# Function to manually install grype
install_grype_manually() {
    print_step "Installing grype manually..."
    
    # Create local bin directory
    mkdir -p "$HOME/.local/bin"
    
    # Detect platform
    case "$(uname -s)" in
        Darwin)
            case "$(uname -m)" in
                x86_64) PLATFORM="darwin_amd64" ;;
                arm64|aarch64) PLATFORM="darwin_arm64" ;;
                *) 
                    print_warning "Unsupported macOS architecture"
                    return 1
                    ;;
            esac
            ;;
        Linux)
            case "$(uname -m)" in
                x86_64) PLATFORM="linux_amd64" ;;
                aarch64|arm64) PLATFORM="linux_arm64" ;;
                *) 
                    print_warning "Unsupported Linux architecture"
                    return 1
                    ;;
            esac
            ;;
        *)
            print_warning "Unsupported operating system"
            return 1
            ;;
    esac
    
    # Download and install
    VERSION="0.92.2"
    URL="https://github.com/anchore/grype/releases/download/v${VERSION}/grype_${VERSION}_${PLATFORM}.tar.gz"
    
    if command_exists curl; then
        print_info "Downloading grype v${VERSION} for ${PLATFORM}..."
        if curl -L "$URL" | tar -xz -C "$HOME/.local/bin" grype; then
            chmod +x "$HOME/.local/bin/grype"
            print_success "grype installed to ~/.local/bin/grype"
            return 0
        else
            print_warning "Failed to download grype automatically"
            return 1
        fi
    else
        print_warning "curl not found"
        return 1
    fi
}

# Function to install OWASP Dependency Check
install_dependency_check() {
    print_step "Installing OWASP Dependency Check..."
    
    # Create installation directory
    mkdir -p "$HOME/.local/dependency-check"
    mkdir -p "$HOME/.local/bin"
    
    VERSION="10.0.4"
    URL="https://github.com/jeremylong/DependencyCheck/releases/download/v${VERSION}/dependency-check-${VERSION}-release.zip"
    
    if command_exists curl && command_exists unzip; then
        print_info "Downloading OWASP Dependency Check v${VERSION}..."
        
        # Download and extract
        if curl -L "$URL" -o "/tmp/dependency-check.zip" && \
           unzip -o "/tmp/dependency-check.zip" -d "$HOME/.local/" && \
           ln -sf "$HOME/.local/dependency-check/bin/dependency-check.sh" "$HOME/.local/bin/dependency-check"; then
            
            chmod +x "$HOME/.local/bin/dependency-check"
            print_success "OWASP Dependency Check installed"
            rm -f "/tmp/dependency-check.zip"
            return 0
        else
            print_warning "Failed to install OWASP Dependency Check"
            return 1
        fi
    else
        print_warning "curl or unzip not found"
        return 1
    fi
}

# Main installation function
install_vulnerability_tools() {
    echo "🛡️  Vulnerability Scanning Tools Installation"
    echo "=============================================="
    echo ""
    
    # Check current status first
    print_step "Checking current tool status..."
    echo ""
    
    # Language-specific tools
    echo "📋 Language-Specific Tools:"
    check_tool_status "cargo-audit" "Rust - cargo-audit"
    check_tool_status "npm" "JavaScript/TypeScript - npm audit"
    check_tool_status "pip-audit" "Python - pip-audit"
    check_tool_status "safety" "Python - safety"
    check_tool_status "bandit" "Python - bandit"
    check_tool_status "govulncheck" "Go - govulncheck"
    
    echo ""
    echo "🔍 Universal Scanners:"
    check_tool_status "grype" "Grype (universal vulnerability scanner)"
    check_tool_status "dependency-check" "OWASP Dependency Check"
    
    echo ""
    print_step "Installing missing tools..."
    
    # 1. Rust - cargo-audit
    if command_exists cargo; then
        if ! cargo audit --version >/dev/null 2>&1; then
            print_step "Installing cargo-audit..."
            if cargo install cargo-audit; then
                print_success "cargo-audit installed"
            else
                print_warning "Failed to install cargo-audit"
            fi
        fi
    else
        print_info "Rust not found - skipping cargo-audit"
    fi
    
    # 2. Node.js/JavaScript - npm (informational only)
    if ! command_exists npm; then
        print_info "npm not found. Install Node.js for JavaScript/TypeScript scanning:"
        echo "  • macOS: brew install node"
        echo "  • Ubuntu/Debian: sudo apt install nodejs npm"
        echo "  • Download: https://nodejs.org/"
    fi
    
    # 3. Python tools
    if command_exists python3 || command_exists python; then
        # Install pip-audit
        if ! command_exists pip-audit; then
            print_step "Installing pip-audit..."
            if command_exists pipx; then
                pipx install pip-audit >/dev/null 2>&1 && print_success "pip-audit installed via pipx"
            elif command_exists pip3; then
                pip3 install --user pip-audit >/dev/null 2>&1 && print_success "pip-audit installed via pip3"
            elif command_exists pip; then
                pip install --user pip-audit >/dev/null 2>&1 && print_success "pip-audit installed via pip"
            else
                print_warning "Could not install pip-audit - no pip found"
            fi
        fi
        
        # Install safety (alternative Python scanner)
        if ! command_exists safety; then
            print_step "Installing safety (Python vulnerability scanner)..."
            if command_exists pipx; then
                pipx install safety >/dev/null 2>&1 && print_success "safety installed via pipx"
            elif command_exists pip3; then
                pip3 install --user safety >/dev/null 2>&1 && print_success "safety installed via pip3"
            elif command_exists pip; then
                pip install --user safety >/dev/null 2>&1 && print_success "safety installed via pip"
            fi
        fi
        
        # Install bandit (Python security linter)
        if ! command_exists bandit; then
            print_step "Installing bandit (Python security linter)..."
            if command_exists pipx; then
                pipx install bandit >/dev/null 2>&1 && print_success "bandit installed via pipx"
            elif command_exists pip3; then
                pip3 install --user bandit >/dev/null 2>&1 && print_success "bandit installed via pip3"
            elif command_exists pip; then
                pip install --user bandit >/dev/null 2>&1 && print_success "bandit installed via pip"
            fi
        fi
    else
        print_info "Python not found - skipping Python security tools"
    fi
    
    # 4. Go - govulncheck
    if command_exists go; then
        if ! command_exists govulncheck && ! test -f "$HOME/go/bin/govulncheck"; then
            print_step "Installing govulncheck..."
            if go install golang.org/x/vuln/cmd/govulncheck@latest; then
                print_success "govulncheck installed"
                print_info "Added to ~/go/bin/govulncheck"
            else
                print_warning "Failed to install govulncheck"
            fi
        fi
    else
        print_info "Go not found - skipping govulncheck"
    fi
    
    # 5. Universal scanners
    # Install grype
    if ! command_exists grype && ! test -f "$HOME/.local/bin/grype"; then
        case "$(uname -s)" in
            Darwin)  # macOS
                if command_exists brew; then
                    print_step "Installing grype via Homebrew..."
                    if brew install anchore/grype/grype; then
                        print_success "grype installed via Homebrew"
                    else
                        install_grype_manually
                    fi
                else
                    install_grype_manually
                fi
                ;;
            Linux)
                install_grype_manually
                ;;
            *)
                print_warning "Platform not supported for automatic grype installation"
                ;;
        esac
    fi
    
    # Install OWASP Dependency Check (optional - heavy tool)
    if [ "${INSTALL_OWASP_DC:-}" = "true" ] && ! command_exists dependency-check && ! test -f "$HOME/.local/bin/dependency-check"; then
        install_dependency_check
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --check-only    Only check tool status, don't install"
    echo "  --owasp-dc      Also install OWASP Dependency Check (large download)"
    echo "  --help          Show this help message"
    echo ""
    echo "This script installs vulnerability scanning tools for:"
    echo "  • Rust: cargo-audit"
    echo "  • JavaScript/TypeScript: npm audit (requires Node.js)"
    echo "  • Python: pip-audit, safety, bandit"
    echo "  • Go: govulncheck"
    echo "  • Universal: grype"
    echo "  • Java (optional): OWASP Dependency Check"
}

# Parse command line arguments
CHECK_ONLY=false
INSTALL_OWASP_DC=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --check-only)
            CHECK_ONLY=true
            shift
            ;;
        --owasp-dc)
            INSTALL_OWASP_DC=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Export for use in functions
export INSTALL_OWASP_DC

# Main execution
if [ "$CHECK_ONLY" = true ]; then
    echo "🛡️  Checking Vulnerability Scanning Tools Status"
    echo "==============================================="
    echo ""
    
    # Check and display status only
    echo "📋 Language-Specific Tools:"
    check_tool_status "cargo-audit" "Rust - cargo-audit"
    check_tool_status "npm" "JavaScript/TypeScript - npm audit"
    check_tool_status "pip-audit" "Python - pip-audit"
    check_tool_status "safety" "Python - safety"
    check_tool_status "bandit" "Python - bandit"
    check_tool_status "govulncheck" "Go - govulncheck"
    
    echo ""
    echo "🔍 Universal Scanners:"
    check_tool_status "grype" "Grype (universal vulnerability scanner)"
    check_tool_status "dependency-check" "OWASP Dependency Check"
    
    echo ""
    print_info "Run without --check-only to install missing tools"
else
    # Install tools
    install_vulnerability_tools
    
    echo ""
    echo "🎯 Installation Summary"
    echo "======================"
    
    # Final status check
    echo ""
    echo "📋 Final Tool Status:"
    check_tool_status "cargo-audit" "Rust - cargo-audit"
    check_tool_status "npm" "JavaScript/TypeScript - npm audit"
    check_tool_status "pip-audit" "Python - pip-audit"
    check_tool_status "safety" "Python - safety"
    check_tool_status "bandit" "Python - bandit"
    check_tool_status "govulncheck" "Go - govulncheck"
    check_tool_status "grype" "Grype (universal scanner)"
    check_tool_status "dependency-check" "OWASP Dependency Check"
    
    # PATH recommendations
    echo ""
    print_info "PATH Configuration:"
    if [ -d "$HOME/.local/bin" ]; then
        echo "  • Add ~/.local/bin to your PATH for locally installed tools"
    fi
    if [ -d "$HOME/go/bin" ]; then
        echo "  • Add ~/go/bin to your PATH for Go tools"
    fi
    
    echo ""
    echo "Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo '  export PATH="$HOME/.local/bin:$HOME/go/bin:$PATH"'
    
    echo ""
    print_success "Vulnerability scanning tools setup complete!"
fi 