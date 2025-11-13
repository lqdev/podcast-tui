#!/bin/bash
# Install icon and desktop file for Linux systems
# This script installs the podcast-tui icon and desktop entry for the current user

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Define installation paths
ICON_DIR="$HOME/.local/share/icons/hicolor"
DESKTOP_DIR="$HOME/.local/share/applications"

print_status "Installing Podcast TUI icon and desktop entry..."

# Create directories if they don't exist
mkdir -p "$ICON_DIR/16x16/apps"
mkdir -p "$ICON_DIR/32x32/apps"
mkdir -p "$ICON_DIR/48x48/apps"
mkdir -p "$ICON_DIR/64x64/apps"
mkdir -p "$ICON_DIR/128x128/apps"
mkdir -p "$ICON_DIR/256x256/apps"
mkdir -p "$ICON_DIR/scalable/apps"
mkdir -p "$DESKTOP_DIR"

# Copy icon files
print_status "Copying icon files..."
cp "$PROJECT_DIR/assets/icons/podcast-tui-16x16.png" "$ICON_DIR/16x16/apps/podcast-tui.png"
cp "$PROJECT_DIR/assets/icons/podcast-tui-32x32.png" "$ICON_DIR/32x32/apps/podcast-tui.png"
cp "$PROJECT_DIR/assets/icons/podcast-tui-48x48.png" "$ICON_DIR/48x48/apps/podcast-tui.png"
cp "$PROJECT_DIR/assets/icons/podcast-tui-64x64.png" "$ICON_DIR/64x64/apps/podcast-tui.png"
cp "$PROJECT_DIR/assets/icons/podcast-tui-128x128.png" "$ICON_DIR/128x128/apps/podcast-tui.png"
cp "$PROJECT_DIR/assets/icons/podcast-tui-256x256.png" "$ICON_DIR/256x256/apps/podcast-tui.png"
cp "$PROJECT_DIR/assets/icons/podcast-tui.svg" "$ICON_DIR/scalable/apps/podcast-tui.svg"

# Copy desktop file
print_status "Installing desktop entry..."
cp "$PROJECT_DIR/assets/linux/podcast-tui.desktop" "$DESKTOP_DIR/"

# Update icon cache if gtk-update-icon-cache is available
if command -v gtk-update-icon-cache &> /dev/null; then
    print_status "Updating icon cache..."
    gtk-update-icon-cache -f -t "$ICON_DIR" 2>/dev/null || print_warning "Failed to update icon cache (non-critical)"
fi

# Update desktop database if update-desktop-database is available
if command -v update-desktop-database &> /dev/null; then
    print_status "Updating desktop database..."
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || print_warning "Failed to update desktop database (non-critical)"
fi

print_status "Icon and desktop entry installed successfully!"
print_status "Icon location: $ICON_DIR"
print_status "Desktop entry: $DESKTOP_DIR/podcast-tui.desktop"
print_status ""
print_status "You may need to log out and back in for changes to take effect."
