#!/bin/bash
# Install dependencies for cross-compilation
# This script installs cargo-zigbuild and zig for cross-platform builds

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_status "Installing cross-compilation dependencies..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed. Please install Rust first."
    exit 1
fi

# Install cargo-zigbuild
print_status "Installing cargo-zigbuild..."
if cargo install cargo-zigbuild; then
    print_status "✓ cargo-zigbuild installed successfully"
else
    print_error "Failed to install cargo-zigbuild"
    exit 1
fi

# Add Rust targets
print_status "Adding compilation targets..."
rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
print_status "✓ Compilation targets added"

# Check if zig is installed
if command -v zig &> /dev/null; then
    ZIG_VERSION=$(zig version)
    print_status "✓ Zig is already installed (version: $ZIG_VERSION)"
else
    print_status "Installing Zig..."
    
    # Detect OS and architecture
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$ARCH" in
        x86_64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        armv7l)
            ARCH="armv7a"
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    # Use pip to install ziglang (cross-platform method)
    if command -v pip3 &> /dev/null; then
        print_status "Installing Zig via pip3..."
        pip3 install ziglang
        
        # Add Python user bin directory to PATH if not already there
        PYTHON_USER_BASE=$(python3 -m site --user-base 2>/dev/null || echo "$HOME/.local")
        PYTHON_BIN_DIR="$PYTHON_USER_BASE/bin"
        if [[ -d "$PYTHON_BIN_DIR" ]] && [[ ":$PATH:" != *":$PYTHON_BIN_DIR:"* ]]; then
            export PATH="$PYTHON_BIN_DIR:$PATH"
            print_status "Added $PYTHON_BIN_DIR to PATH"
            
            # Also add to GitHub Actions PATH if running in CI
            if [[ -n "$GITHUB_PATH" ]]; then
                echo "$PYTHON_BIN_DIR" >> "$GITHUB_PATH"
                print_status "Added $PYTHON_BIN_DIR to GITHUB_PATH for future steps"
            fi
        fi
        
        print_status "✓ Zig installed successfully via pip"
    elif command -v pip &> /dev/null; then
        print_status "Installing Zig via pip..."
        pip install ziglang
        
        # Add Python user bin directory to PATH if not already there
        PYTHON_USER_BASE=$(python -m site --user-base 2>/dev/null || echo "$HOME/.local")
        PYTHON_BIN_DIR="$PYTHON_USER_BASE/bin"
        if [[ -d "$PYTHON_BIN_DIR" ]] && [[ ":$PATH:" != *":$PYTHON_BIN_DIR:"* ]]; then
            export PATH="$PYTHON_BIN_DIR:$PATH"
            print_status "Added $PYTHON_BIN_DIR to PATH"
            
            # Also add to GitHub Actions PATH if running in CI
            if [[ -n "$GITHUB_PATH" ]]; then
                echo "$PYTHON_BIN_DIR" >> "$GITHUB_PATH"
                print_status "Added $PYTHON_BIN_DIR to GITHUB_PATH for future steps"
            fi
        fi
        
        print_status "✓ Zig installed successfully via pip"
    else
        print_warning "pip not found. Attempting to install via package manager..."
        
        # Try package managers based on OS
        case "$OS" in
            linux)
                if command -v apt-get &> /dev/null; then
                    print_status "Installing via apt..."
                    sudo apt-get update
                    sudo apt-get install -y wget xz-utils
                    
                    # Download and install Zig manually
                    ZIG_VERSION="0.11.0"
                    ZIG_TARBALL="zig-linux-${ARCH}-${ZIG_VERSION}.tar.xz"
                    ZIG_URL="https://ziglang.org/download/${ZIG_VERSION}/${ZIG_TARBALL}"
                    
                    print_status "Downloading Zig from $ZIG_URL..."
                    wget "$ZIG_URL" -O "/tmp/${ZIG_TARBALL}"
                    
                    print_status "Extracting Zig..."
                    sudo tar -xf "/tmp/${ZIG_TARBALL}" -C /usr/local/
                    sudo ln -sf "/usr/local/zig-linux-${ARCH}-${ZIG_VERSION}/zig" /usr/local/bin/zig
                    
                    rm "/tmp/${ZIG_TARBALL}"
                    print_status "✓ Zig installed successfully"
                else
                    print_error "Unable to install Zig automatically. Please install manually from https://ziglang.org/download/"
                    exit 1
                fi
                ;;
            darwin)
                if command -v brew &> /dev/null; then
                    print_status "Installing via Homebrew..."
                    brew install zig
                    print_status "✓ Zig installed successfully"
                else
                    print_error "Homebrew not found. Please install Zig manually from https://ziglang.org/download/"
                    exit 1
                fi
                ;;
            *)
                print_error "Unsupported OS: $OS"
                print_error "Please install Zig manually from https://ziglang.org/download/"
                exit 1
                ;;
        esac
    fi
fi

# Verify installation
print_status "Verifying installations..."

if command -v cargo-zigbuild &> /dev/null; then
    print_status "✓ cargo-zigbuild is available"
else
    print_error "cargo-zigbuild not found in PATH"
    exit 1
fi

if command -v zig &> /dev/null; then
    ZIG_VERSION=$(zig version)
    print_status "✓ Zig is available (version: $ZIG_VERSION)"
else
    print_error "zig not found in PATH"
    exit 1
fi

# Install additional development dependencies for Linux
if [[ "$OS" == "linux" ]]; then
    print_status "Checking for additional Linux build dependencies..."
    
    if command -v apt-get &> /dev/null; then
        print_status "Installing Linux build dependencies..."
        sudo apt-get update
        sudo apt-get install -y \
            build-essential \
            pkg-config \
            libssl-dev \
            libasound2-dev \
            zip \
            || print_warning "Some dependencies may have failed to install"
    fi
fi

echo ""
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "All dependencies installed successfully! 🎉"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
print_status "You can now run: ./scripts/build-releases.sh"
echo ""
