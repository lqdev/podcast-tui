#!/bin/bash

# Simple script to run podcast-tui outside VS Code
# to avoid keybinding conflicts

echo "Starting Podcast TUI..."
echo "Simple keybindings:"
echo "  F1 = Help, F2 = Podcasts, F3 = Help, F5 = Refresh, F10 = Quit"
echo "  a = Add podcast, d = Delete, r = Refresh, q = Quit"
echo "  Arrow keys = Navigate, Tab/Shift+Tab = Switch buffers"
echo "  : = Command prompt"
echo ""

cd /workspaces/podcast-tui
./target/debug/podcast-tui