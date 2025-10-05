#!/bin/bash
# Quick test script to verify the build system works
# Builds a single target to test the setup without building all platforms

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[TEST]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_status "Testing build system with native target..."

# Detect current platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Determine appropriate target
if [[ "$OS" == "linux" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        TARGET="x86_64-unknown-linux-gnu"
    elif [[ "$ARCH" == "aarch64" ]]; then
        TARGET="aarch64-unknown-linux-gnu"
    else
        print_warning "Unsupported architecture: $ARCH"
        TARGET="x86_64-unknown-linux-gnu"
    fi
elif [[ "$OS" == "darwin" ]]; then
    if [[ "$ARCH" == "arm64" ]]; then
        TARGET="aarch64-apple-darwin"
    else
        TARGET="x86_64-apple-darwin"
    fi
else
    print_warning "Unsupported OS: $OS"
    TARGET="x86_64-unknown-linux-gnu"
fi

print_status "Building for target: $TARGET"

# Build
if cargo zigbuild --release --target "$TARGET"; then
    print_status "✓ Build successful!"
    
    BINARY_PATH="target/${TARGET}/release/podcast-tui"
    if [[ "$OS" == "mingw"* ]] || [[ "$OS" == "msys"* ]]; then
        BINARY_PATH="${BINARY_PATH}.exe"
    fi
    
    if [ -f "$BINARY_PATH" ]; then
        print_status "✓ Binary exists at: $BINARY_PATH"
        
        SIZE=$(du -h "$BINARY_PATH" | cut -f1)
        print_status "Binary size: $SIZE"
        
        if [ -x "$BINARY_PATH" ]; then
            print_status "✓ Binary is executable"
            
            # Try to get version info
            if "$BINARY_PATH" --version 2>/dev/null; then
                print_status "✓ Binary runs successfully"
            else
                print_warning "Binary exists but failed version check (this may be expected if not implemented)"
            fi
        fi
    else
        print_warning "Binary not found at expected location"
    fi
else
    echo "❌ Build failed"
    exit 1
fi

print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "Build system test completed successfully! 🎉"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
print_status "You can now run: ./scripts/build-releases.sh to build all platforms"
