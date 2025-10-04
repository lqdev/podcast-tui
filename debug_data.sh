#!/bin/bash

# Debug script to analyze podcast data and identify duplication issues

echo "=== Podcast TUI Debug Analysis ==="
echo ""

DATA_DIR="$HOME/.local/share/podcast-tui"

if [ ! -d "$DATA_DIR" ]; then
    echo "No podcast data found at $DATA_DIR"
    exit 1
fi

echo "Data directory: $DATA_DIR"
echo ""

# Check podcasts
echo "=== Podcasts ==="
PODCAST_COUNT=$(find "$DATA_DIR/podcasts" -name "*.json" 2>/dev/null | wc -l)
echo "Total podcasts: $PODCAST_COUNT"

if [ $PODCAST_COUNT -gt 0 ]; then
    echo ""
    echo "Podcast list:"
    for podcast_file in "$DATA_DIR/podcasts"/*.json; do
        if [ -f "$podcast_file" ]; then
            TITLE=$(jq -r '.title // "Unknown"' "$podcast_file" 2>/dev/null)
            EPISODE_COUNT=$(jq -r '.episodes | length' "$podcast_file" 2>/dev/null)
            echo "  - $TITLE ($EPISODE_COUNT episodes)"
        fi
    done
fi

echo ""
echo "=== Episodes ==="

for podcast_dir in "$DATA_DIR/episodes"/*/; do
    if [ -d "$podcast_dir" ]; then
        PODCAST_ID=$(basename "$podcast_dir")
        EPISODE_COUNT=$(find "$podcast_dir" -name "*.json" 2>/dev/null | wc -l)
        
        echo ""
        echo "Podcast ID: $PODCAST_ID"
        echo "Episodes stored: $EPISODE_COUNT"
        
        # Check for episodes without audio URLs
        NO_AUDIO_COUNT=0
        DUPLICATE_TITLES=()
        
        if [ $EPISODE_COUNT -gt 0 ]; then
            echo "Analyzing episodes..."
            
            # Create temp file for titles
            TEMP_TITLES=$(mktemp)
            
            for episode_file in "$podcast_dir"*.json; do
                if [ -f "$episode_file" ]; then
                    AUDIO_URL=$(jq -r '.audio_url // ""' "$episode_file" 2>/dev/null)
                    TITLE=$(jq -r '.title // "Unknown"' "$episode_file" 2>/dev/null)
                    GUID=$(jq -r '.guid // ""' "$episode_file" 2>/dev/null)
                    
                    if [ -z "$AUDIO_URL" ] || [ "$AUDIO_URL" = "null" ]; then
                        ((NO_AUDIO_COUNT++))
                    fi
                    
                    echo "$TITLE" >> "$TEMP_TITLES"
                fi
            done
            
            # Check for duplicate titles
            DUPLICATE_COUNT=$(sort "$TEMP_TITLES" | uniq -d | wc -l)
            
            echo "  - Episodes without audio URL: $NO_AUDIO_COUNT"
            echo "  - Episodes with duplicate titles: $DUPLICATE_COUNT"
            
            if [ $DUPLICATE_COUNT -gt 0 ]; then
                echo "  - Duplicate titles found:"
                sort "$TEMP_TITLES" | uniq -d | head -5 | sed 's/^/    /'
                if [ $DUPLICATE_COUNT -gt 5 ]; then
                    echo "    ... and $((DUPLICATE_COUNT - 5)) more"
                fi
            fi
            
            rm -f "$TEMP_TITLES"
        fi
    fi
done

echo ""
echo "=== Summary ==="
TOTAL_EPISODES=$(find "$DATA_DIR/episodes" -name "*.json" 2>/dev/null | wc -l)
echo "Total episodes across all podcasts: $TOTAL_EPISODES"

echo ""
echo "=== Recommendations ==="
echo "1. If you see many duplicate titles, the deduplication isn't working properly"
echo "2. High counts of 'no audio URL' episodes suggest RSS parsing issues"
echo "3. Run the app and check if the improvements reduce these numbers"