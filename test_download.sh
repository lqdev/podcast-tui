#!/bin/bash

# Test download functionality
echo "Testing podcast TUI download functionality..."

# Build first
echo "Building the application..."
cargo build --release

# Test with a sample podcast feed
echo "Testing with Python Talk podcast feed..."
echo "You can add this feed URL to test downloads: https://feeds.transistor.fm/python-podcast"
echo ""
echo "Instructions to test download functionality:"
echo "1. Run the app with './target/release/podcast-tui'"
echo "2. Press 'a' to add a podcast"
echo "3. Enter the URL: https://feeds.transistor.fm/python-podcast"
echo "4. Wait for the feed to load"
echo "5. Press Enter to view episodes"
echo "6. Select an episode and press 'D' to download"
echo "7. Press 'X' to delete a downloaded episode"
echo ""
echo "The download directory is configured as: ~/Downloads/Podcasts/"
echo ""

# Show current downloads directory status
DOWNLOADS_DIR="$HOME/Downloads/Podcasts"
if [ -d "$DOWNLOADS_DIR" ]; then
    echo "Downloads directory exists at: $DOWNLOADS_DIR"
    echo "Current contents:"
    ls -la "$DOWNLOADS_DIR" 2>/dev/null || echo "Directory is empty"
else
    echo "Downloads directory will be created at: $DOWNLOADS_DIR"
fi

echo ""
echo "Run './target/release/podcast-tui' to start testing!"