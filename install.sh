#!/bin/bash
# Syncable CLI Installation Script

set -e

echo "🚀 Installing Syncable IaC CLI..."
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check Rust version
RUST_VERSION=$(rustc --version | cut -d' ' -f2)
MIN_VERSION="1.70.0"

if [ "$(printf '%s\n' "$MIN_VERSION" "$RUST_VERSION" | sort -V | head -n1)" != "$MIN_VERSION" ]; then
    echo "❌ Rust version $RUST_VERSION is too old. Please update to at least $MIN_VERSION"
    echo "   rustup update"
    exit 1
fi

echo "✅ Rust $RUST_VERSION detected"
echo ""

# Clone repository if not already in it
if [ ! -f "Cargo.toml" ] || [ ! -d "src" ]; then
    echo "📦 Cloning Syncable CLI repository..."
    git clone https://github.com/yourusername/syncable-cli.git
    cd syncable-cli
fi

echo "🔨 Building Syncable CLI (this may take a few minutes)..."
cargo build --release

echo ""
echo "📦 Installing Syncable CLI..."
cargo install --path .

echo ""
echo "✅ Installation complete!"
echo ""
echo "🎯 Quick Start:"
echo "   sync-ctl --help                # Show help"
echo "   sync-ctl analyze .             # Analyze current directory"
echo "   sync-ctl vuln-check .          # Check for vulnerabilities"
echo ""
echo "📚 For more information, see TUTORIAL.md"
echo "" 