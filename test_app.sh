#!/bin/bash

echo "Starting Podcast TUI..."
echo ""
echo "Testing key bindings:"
echo "1. The app should start showing the podcast list"
echo "2. Press 'a' to add a podcast - you should see 'Add podcast URL: ' prompt at the bottom"
echo "3. Type a URL like: https://feeds.npr.org/1001/podcast.xml"
echo "4. Press Enter to submit"
echo "5. Press 'q' to quit"
echo ""
echo "If the above works, then the issue was user expectation, not code."
echo ""
echo "Starting app in 3 seconds..."
sleep 3

cd /workspaces/podcast-tui
./target/debug/podcast-tui