#!/bin/bash
# Syncable CLI Installation Script

set -e

echo "🚀 Installing Syncable IaC CLI..."
echo ""

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

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    print_error "Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check Rust version
RUST_VERSION=$(rustc --version | cut -d' ' -f2)
MIN_VERSION="1.70.0"

if [ "$(printf '%s\n' "$MIN_VERSION" "$RUST_VERSION" | sort -V | head -n1)" != "$MIN_VERSION" ]; then
    print_error "Rust version $RUST_VERSION is too old. Please update to at least $MIN_VERSION"
    echo "   rustup update"
    exit 1
fi

print_success "Rust $RUST_VERSION detected"
echo ""

# Clone repository if not already in it
if [ ! -f "Cargo.toml" ] || [ ! -d "src" ]; then
    print_step "Cloning Syncable CLI repository..."
    git clone https://github.com/syncable-dev/syncable-cli.git
    cd syncable-cli
fi

print_step "Building Syncable CLI (this may take a few minutes)..."
cargo build --release

echo ""
print_step "Installing Syncable CLI..."
cargo install --path .

echo ""
print_success "Syncable CLI installed successfully!"

# Now install vulnerability scanning tools
echo ""
echo "🛡️  Setting up vulnerability scanning tools..."
echo "================================================"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to install tools based on platform
install_vulnerability_tools() {
    print_step "Checking and installing vulnerability scanning tools..."
    
    # 1. Rust - cargo-audit
    if command_exists cargo; then
        if ! cargo audit --version >/dev/null 2>&1; then
            print_step "Installing cargo-audit for Rust vulnerability scanning..."
            if cargo install cargo-audit; then
                print_success "cargo-audit installed"
            else
                print_warning "Failed to install cargo-audit"
            fi
        else
            print_success "cargo-audit already installed"
        fi
    fi
    
    # 2. Node.js/JavaScript - npm (comes with Node.js)
    if command_exists npm; then
        print_success "npm detected (Node.js vulnerability scanning available)"
    else
        print_warning "npm not found. Install Node.js for JavaScript/TypeScript vulnerability scanning:"
        echo "   • Download from: https://nodejs.org/"
        echo "   • Or use package manager:"
        echo "     - macOS: brew install node"
        echo "     - Ubuntu/Debian: sudo apt install nodejs npm"
        echo "     - CentOS/RHEL: sudo yum install nodejs npm"
    fi
    
    # 3. Python - pip-audit
    if command_exists python3 || command_exists python; then
        if ! command_exists pip-audit; then
            print_step "Installing pip-audit for Python vulnerability scanning..."
            
            # Try different installation methods
            if command_exists pipx; then
                if pipx install pip-audit; then
                    print_success "pip-audit installed via pipx"
                fi
            elif command_exists pip3; then
                if pip3 install --user pip-audit; then
                    print_success "pip-audit installed via pip3"
                fi
            elif command_exists pip; then
                if pip install --user pip-audit; then
                    print_success "pip-audit installed via pip"
                fi
            else
                print_warning "Could not install pip-audit automatically. Install manually:"
                echo "   • pipx install pip-audit (recommended)"
                echo "   • pip3 install --user pip-audit"
            fi
        else
            print_success "pip-audit already installed"
        fi
    else
        print_warning "Python not found. Install Python for Python vulnerability scanning:"
        echo "   • Download from: https://python.org/"
        echo "   • Or use package manager:"
        echo "     - macOS: brew install python"
        echo "     - Ubuntu/Debian: sudo apt install python3 python3-pip"
    fi
    
    # 4. Go - govulncheck
    if command_exists go; then
        if ! command_exists govulncheck && ! test -f "$HOME/go/bin/govulncheck"; then
            print_step "Installing govulncheck for Go vulnerability scanning..."
            if go install golang.org/x/vuln/cmd/govulncheck@latest; then
                print_success "govulncheck installed"
                print_info "Make sure ~/go/bin is in your PATH"
            else
                print_warning "Failed to install govulncheck"
            fi
        else
            print_success "govulncheck already installed"
        fi
    else
        print_warning "Go not found. Install Go for Go vulnerability scanning:"
        echo "   • Download from: https://golang.org/"
        echo "   • Or use package manager:"
        echo "     - macOS: brew install go"
        echo "     - Ubuntu/Debian: sudo apt install golang-go"
    fi
    
    # 5. Java/Kotlin - grype (universal vulnerability scanner)
    if ! command_exists grype && ! test -f "$HOME/.local/bin/grype"; then
        print_step "Installing grype for universal vulnerability scanning (Java, containers, etc.)..."
        
        case "$(uname -s)" in
            Darwin)  # macOS
                if command_exists brew; then
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
                print_info "Please install grype manually: https://github.com/anchore/grype"
                ;;
        esac
    else
        print_success "grype already installed"
    fi
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
            print_info "Make sure ~/.local/bin is in your PATH"
        else
            print_warning "Failed to download grype automatically"
            print_info "Please install manually: https://github.com/anchore/grype#installation"
        fi
    else
        print_warning "curl not found. Please install grype manually: https://github.com/anchore/grype#installation"
    fi
}

# Install vulnerability scanning tools
install_vulnerability_tools

echo ""
echo "🎯 Installation Complete!"
echo "========================"
print_success "Syncable CLI is ready to use!"

echo ""
echo "📚 Quick Start Guide:"
echo "  sync-ctl --help                    # Show all commands"
echo "  sync-ctl analyze .                 # Analyze current directory"
echo "  sync-ctl generate .                # Generate IaC files"
echo "  sync-ctl vuln-check .              # Check for vulnerabilities"
echo "  sync-ctl security-scan .           # Comprehensive security analysis"

echo ""
echo "🔧 Environment Setup:"

# Check if common directories are in PATH
PATH_ADDITIONS=""
if [ -d "$HOME/.local/bin" ] && [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    PATH_ADDITIONS="$PATH_ADDITIONS$HOME/.local/bin:"
fi
if [ -d "$HOME/go/bin" ] && [[ ":$PATH:" != *":$HOME/go/bin:"* ]]; then
    PATH_ADDITIONS="$PATH_ADDITIONS$HOME/go/bin:"
fi

if [ -n "$PATH_ADDITIONS" ]; then
    print_warning "Some tools may not be in your PATH. Add these to your shell profile:"
    echo "  export PATH=\"${PATH_ADDITIONS%:}:\$PATH\""
    echo ""
    echo "For current session, run:"
    echo "  export PATH=\"${PATH_ADDITIONS%:}:\$PATH\""
fi

echo ""
print_info "For more information and examples, see:"
echo "  • README.md - General usage and examples"
echo "  • CONTRIBUTING.md - Development guide"
echo "  • https://github.com/syncable-dev/syncable-cli"

echo ""
print_success "Happy coding! 🚀" 