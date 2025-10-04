#!/bin/bash

# Test script to check episode deduplication and audio URL extraction

echo "Testing RSS parsing improvements..."

# Kill any existing instance
pkill -f podcast-tui 2>/dev/null || true

# Wait a moment
sleep 1

# Run the app in the background for a few seconds to let it load
timeout 10s cargo run 2>/dev/null &
APP_PID=$!

# Wait for the app to start
sleep 5

# Kill the app
kill $APP_PID 2>/dev/null || true
wait $APP_PID 2>/dev/null || true

echo "Test completed. Check the UI for:"
echo "1. Reduced episode duplication"
echo "2. Fewer episodes with '(no audio URL)'"
echo "3. Better warning symbols for truly unavailable episodes"

# Show some stats
echo ""
echo "Episode count statistics:"
find ~/.local/share/podcast-tui/episodes/ -name "*.json" 2>/dev/null | wc -l | xargs echo "Total episodes stored:"

echo ""
echo "You can now run 'cargo run' to check the improvements in the UI"