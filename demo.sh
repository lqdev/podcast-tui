#!/bin/bash

# Simple demo script to show the minimal working features

echo "🎧 Podcast TUI MVP Demo"
echo "======================="
echo ""
echo "This demonstrates the core functionality:"
echo ""
echo "✅ Config-based setup:"
echo "   - Downloads folder: ~/Downloads/Podcasts"
echo "   - Feed list managed in storage"
echo ""
echo "✅ Add/Remove feeds:"
echo "   - Press 'a' to add a podcast feed"
echo "   - Press 'd' to delete a podcast subscription"
echo ""
echo "✅ Download/Delete episodes:"
echo "   - Press Enter on a podcast to view episodes"
echo "   - Press 'D' to download an episode"
echo "   - Press 'X' to delete downloaded file"
echo ""
echo "✅ Basic navigation:"
echo "   - Arrow keys to navigate"
echo "   - Tab to switch between buffers"
echo "   - F1 for help"
echo "   - 'q' to quit"
echo ""
echo "Starting the application..."
echo ""

cd /workspaces/podcast-tui
cargo run