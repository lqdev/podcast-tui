#!/bin/bash
# Cross-platform release build script for podcast-tui
# This script builds release binaries for multiple platforms using cargo-zigbuild
#
# Usage:
#   ./scripts/build-releases.sh           # Build all available targets
#   ./scripts/build-releases.sh --linux-only  # Build only Linux targets

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
LINUX_ONLY=false
if [[ "$1" == "--linux-only" ]]; then
    LINUX_ONLY=true
fi

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Get the project version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
print_status "Building podcast-tui version: $VERSION"

# Create release directory
RELEASE_DIR="releases/v${VERSION}"
mkdir -p "$RELEASE_DIR"
print_status "Release directory: $RELEASE_DIR"

if [[ "$LINUX_ONLY" == "true" ]]; then
    print_status "Building Linux targets only"
else
    print_warning "Building all targets (including Windows)"
    print_warning "Note: Windows builds may fail on Linux hosts"
    print_warning "Use './scripts/build-releases.sh --linux-only' to build only Linux targets"
    print_warning "Or use GitHub Actions for reliable multi-platform builds"
    echo ""
fi

# Define target platforms
# Format: "target-triple:output-name:archive-name"
TARGETS=(
    # Windows x86_64
    "x86_64-pc-windows-msvc:podcast-tui.exe:podcast-tui-v${VERSION}-windows-x86_64"
    
    # Windows ARM64
    "aarch64-pc-windows-msvc:podcast-tui.exe:podcast-tui-v${VERSION}-windows-aarch64"
    
    # Linux x86_64
    "x86_64-unknown-linux-gnu:podcast-tui:podcast-tui-v${VERSION}-linux-x86_64"
    
    # Linux ARM64
    "aarch64-unknown-linux-gnu:podcast-tui:podcast-tui-v${VERSION}-linux-aarch64"
)

# Function to build for a specific target
build_target() {
    local target_triple="$1"
    local output_name="$2"
    local archive_name="$3"
    
    print_status "Building for target: $target_triple"
    
    # Determine build command based on target
    # Use cargo-zigbuild for Linux, skip Windows if running locally
    if [[ "$target_triple" == *"windows"* ]] && [[ "$LINUX_ONLY" == "true" ]]; then
        print_warning "Skipping Windows target $target_triple (use --linux-only flag or build in CI)"
        return 1
    elif [[ "$target_triple" == *"windows"* ]]; then
        print_warning "Windows cross-compilation from Linux may fail locally"
        print_warning "For reliable Windows builds, use GitHub Actions or a Windows machine"
        print_warning "Attempting build anyway..."
        BUILD_CMD="cargo zigbuild --release --target $target_triple"
    else
        BUILD_CMD="cargo zigbuild --release --target $target_triple"
    fi
    
    # Build
    if $BUILD_CMD; then
        print_status "✓ Build successful for $target_triple"
        
        # Create archive directory
        local archive_dir="$RELEASE_DIR/$archive_name"
        mkdir -p "$archive_dir"
        
        # Copy binary
        local binary_path="target/${target_triple}/release/${output_name}"
        if [ -f "$binary_path" ]; then
            cp "$binary_path" "$archive_dir/"
            print_status "✓ Binary copied to $archive_dir/"
            
            # Copy additional files
            cp README.md "$archive_dir/" 2>/dev/null || true
            cp LICENSE "$archive_dir/" 2>/dev/null || true
            cp CHANGELOG.md "$archive_dir/" 2>/dev/null || true
            
            # Create archive
            cd "$RELEASE_DIR"
            if [[ "$target_triple" == *"windows"* ]]; then
                # Create ZIP for Windows
                if command -v zip &> /dev/null; then
                    zip -r "${archive_name}.zip" "$archive_name"
                    print_status "✓ Created ${archive_name}.zip"
                else
                    print_warning "zip not found, skipping archive creation"
                fi
            else
                # Create tar.gz for Linux
                tar -czf "${archive_name}.tar.gz" "$archive_name"
                print_status "✓ Created ${archive_name}.tar.gz"
            fi
            cd - > /dev/null
            
            # Generate checksums
            if [[ "$target_triple" == *"windows"* ]]; then
                sha256sum "$RELEASE_DIR/${archive_name}.zip" > "$RELEASE_DIR/${archive_name}.zip.sha256" 2>/dev/null || true
            else
                sha256sum "$RELEASE_DIR/${archive_name}.tar.gz" > "$RELEASE_DIR/${archive_name}.tar.gz.sha256" 2>/dev/null || true
            fi
        else
            print_error "Binary not found at $binary_path"
            return 1
        fi
    else
        print_error "Build failed for $target_triple"
        return 1
    fi
}

# Main build loop
print_status "Starting cross-compilation builds..."
echo ""

FAILED_TARGETS=()
SUCCESSFUL_TARGETS=()

for target_info in "${TARGETS[@]}"; do
    IFS=':' read -r target_triple output_name archive_name <<< "$target_info"
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    if build_target "$target_triple" "$output_name" "$archive_name"; then
        SUCCESSFUL_TARGETS+=("$target_triple")
    else
        FAILED_TARGETS+=("$target_triple")
    fi
    echo ""
done

# Print summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "Build Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo -e "\n${GREEN}Successful builds (${#SUCCESSFUL_TARGETS[@]}):${NC}"
for target in "${SUCCESSFUL_TARGETS[@]}"; do
    echo "  ✓ $target"
done

if [ ${#FAILED_TARGETS[@]} -gt 0 ]; then
    echo -e "\n${RED}Failed builds (${#FAILED_TARGETS[@]}):${NC}"
    for target in "${FAILED_TARGETS[@]}"; do
        echo "  ✗ $target"
    done
fi

echo -e "\n${GREEN}Release artifacts available in: $RELEASE_DIR${NC}"
echo ""

# List all created archives
print_status "Created archives:"
ls -lh "$RELEASE_DIR"/*.{zip,tar.gz} 2>/dev/null || print_warning "No archives found"

# Exit with error if any builds failed
if [ ${#FAILED_TARGETS[@]} -gt 0 ]; then
    exit 1
fi

print_status "All builds completed successfully! 🎉"
