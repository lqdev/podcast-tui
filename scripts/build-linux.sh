#!/bin/bash
# Simple release build script for local development
# Builds for the current platform only

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Detect current architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)
        TARGET="x86_64-unknown-linux-gnu"
        ARCH_NAME="x86_64"
        ;;
    aarch64)
        TARGET="aarch64-unknown-linux-gnu"
        ARCH_NAME="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Get version
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
print_status "Building podcast-tui version: $VERSION for Linux $ARCH_NAME"

# Create release directory
RELEASE_DIR="releases/v${VERSION}"
mkdir -p "$RELEASE_DIR"

# Build for current platform
print_status "Building release binary..."
cargo build --release

# Package
print_status "Packaging binary..."
ARCHIVE_NAME="podcast-tui-v${VERSION}-linux-${ARCH_NAME}"
ARCHIVE_DIR="$RELEASE_DIR/$ARCHIVE_NAME"
mkdir -p "$ARCHIVE_DIR"
cp target/release/podcast-tui "$ARCHIVE_DIR/"
cp README.md "$ARCHIVE_DIR/" 2>/dev/null || true
[ -f LICENSE ] && cp LICENSE "$ARCHIVE_DIR/"
[ -f CHANGELOG.md ] && cp CHANGELOG.md "$ARCHIVE_DIR/"

cd "$RELEASE_DIR"
tar -czf "${ARCHIVE_NAME}.tar.gz" "$ARCHIVE_NAME"
sha256sum "${ARCHIVE_NAME}.tar.gz" > "${ARCHIVE_NAME}.tar.gz.sha256"
cd - > /dev/null

print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "Linux build complete! 🎉"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "Archive: $RELEASE_DIR/${ARCHIVE_NAME}.tar.gz"
ls -lh "$RELEASE_DIR"/${ARCHIVE_NAME}.tar.gz
echo ""
print_warning "Note: This builds only for your current platform ($ARCH_NAME)"
print_warning "For multi-platform releases (Linux x86_64/ARM64 + Windows), use GitHub Actions:"
print_warning "  git tag v${VERSION}"
print_warning "  git push origin v${VERSION}"
