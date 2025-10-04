#!/bin/bash

echo "🎧 Podcast TUI - Download Configuration Test"
echo "============================================="
echo

# Check if config exists
CONFIG_FILE="$HOME/.config/podcast-tui/config.json"
DOWNLOAD_DIR="$HOME/Downloads/Podcasts"

echo "📁 Download Configuration:"
echo "   Default download folder: $DOWNLOAD_DIR"
echo "   Config file location:    $CONFIG_FILE"
echo

# Check if config file exists
if [ -f "$CONFIG_FILE" ]; then
    echo "✅ Config file exists:"
    echo "   $(ls -la "$CONFIG_FILE")"
    echo
    echo "📄 Download settings in config:"
    if command -v jq >/dev/null 2>&1; then
        jq '.downloads' "$CONFIG_FILE" 2>/dev/null || echo "   (Install 'jq' to view JSON config nicely)"
    else
        grep -A 10 '"downloads"' "$CONFIG_FILE" || echo "   (Raw config - install 'jq' for better formatting)"
    fi
else
    echo "ℹ️  Config file doesn't exist yet - will be created on first run"
    echo "   Default settings:"
    echo '   {
     "directory": "~/Downloads/Podcasts",
     "concurrent_downloads": 3,
     "cleanup_after_days": 30,
     "auto_download_new": false,
     "max_download_size_mb": 500
   }'
fi

echo
echo "📂 Download folder status:"
if [ -d "$DOWNLOAD_DIR" ]; then
    echo "✅ Download folder exists: $DOWNLOAD_DIR"
    echo "   Contents:"
    ls -la "$DOWNLOAD_DIR" 2>/dev/null || echo "   (Empty)"
else
    echo "ℹ️  Download folder doesn't exist yet - will be created when downloading episodes"
    echo "   Folder will be: $DOWNLOAD_DIR"
fi

echo
echo "⌨️  Download Keybindings:"
echo "   D (Shift+D)  - Download selected episode"
echo "   X (Shift+X)  - Delete downloaded episode file"
echo "   r            - Refresh podcast feed"
echo "   R (Shift+R)  - Refresh all podcast feeds"
echo
echo "🚀 Quick Start:"
echo "   1. Run: ./podcast-tui"
echo "   2. Press 'a' to add a podcast"
echo "   3. Enter a podcast RSS URL"
echo "   4. Press Enter to view episodes"
echo "   5. Press 'D' to download an episode"
echo "   6. Check $DOWNLOAD_DIR for downloaded files"
echo
echo "💡 Tip: Press F1 or 'h' in the app for help"