#!/bin/bash

echo "🎉 DOWNLOAD FUNCTIONALITY TEST"
echo "============================="
echo ""
echo "📁 Downloads will be saved to: ~/Downloads/Podcasts/"
echo ""
echo "⌨️  KEYBINDINGS:"
echo "  - Shift+D (D) = Download episode"
echo "  - Shift+X (X) = Delete downloaded episode"
echo ""
echo "🔄 Starting the application..."
echo "Navigate to an episode and press Shift+D to test!"
echo ""

# Create downloads directory if it doesn't exist
mkdir -p ~/Downloads/Podcasts/

# Run the app
./target/debug/podcast-tui