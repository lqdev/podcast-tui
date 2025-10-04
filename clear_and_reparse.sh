#!/bin/bash

# Script to clear existing podcast data and re-parse with improved logic

echo "=== Clearing existing podcast data ==="

DATA_DIR="$HOME/.local/share/podcast-tui"

if [ -d "$DATA_DIR" ]; then
    echo "Backing up existing data..."
    cp -r "$DATA_DIR" "$DATA_DIR.backup.$(date +%Y%m%d_%H%M%S)"
    
    echo "Removing existing episodes to force re-parsing..."
    rm -rf "$DATA_DIR/episodes"
    
    echo "Existing data backed up and cleared."
else
    echo "No existing data found."
fi

echo ""
echo "Now run 'cargo run' to re-parse feeds with improved logic."
echo "The app will re-download and parse all episodes with the new algorithms."