#!/bin/bash
# Regenerate PNG and ICO files from SVG source
# Requires: librsvg2-bin (for rsvg-convert) and imagemagick (for convert)

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check for required tools
if ! command -v rsvg-convert &> /dev/null; then
    print_error "rsvg-convert not found. Install librsvg2-bin:"
    echo "  Ubuntu/Debian: sudo apt-get install librsvg2-bin"
    echo "  Fedora: sudo dnf install librsvg2-tools"
    echo "  macOS: brew install librsvg"
    exit 1
fi

if ! command -v convert &> /dev/null; then
    print_error "convert (ImageMagick) not found. Install imagemagick:"
    echo "  Ubuntu/Debian: sudo apt-get install imagemagick"
    echo "  Fedora: sudo dnf install ImageMagick"
    echo "  macOS: brew install imagemagick"
    exit 1
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

SVG_FILE="podcast-tui.svg"

if [ ! -f "$SVG_FILE" ]; then
    print_error "Source SVG file not found: $SVG_FILE"
    exit 1
fi

print_status "Regenerating icons from $SVG_FILE..."

# Generate PNG files at various sizes
print_status "Generating PNG files..."
rsvg-convert -w 16 -h 16 "$SVG_FILE" -o podcast-tui-16x16.png
rsvg-convert -w 32 -h 32 "$SVG_FILE" -o podcast-tui-32x32.png
rsvg-convert -w 48 -h 48 "$SVG_FILE" -o podcast-tui-48x48.png
rsvg-convert -w 64 -h 64 "$SVG_FILE" -o podcast-tui-64x64.png
rsvg-convert -w 128 -h 128 "$SVG_FILE" -o podcast-tui-128x128.png
rsvg-convert -w 256 -h 256 "$SVG_FILE" -o podcast-tui-256x256.png

# Generate Windows ICO file
print_status "Generating Windows ICO file..."
convert podcast-tui-16x16.png podcast-tui-32x32.png podcast-tui-48x48.png \
        podcast-tui-64x64.png podcast-tui-128x128.png podcast-tui-256x256.png \
        podcast-tui.ico

print_status "Icon generation complete!"
print_status "Generated files:"
ls -lh podcast-tui*.png podcast-tui.ico

print_status ""
print_status "Next steps:"
print_status "  1. Rebuild the application to embed the new Windows icon"
print_status "  2. Run ./scripts/install-icon-linux.sh to update Linux system icons"
