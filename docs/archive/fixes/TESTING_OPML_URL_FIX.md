# Testing Instructions for OPML Import URL Fix

## Bug That Was Fixed
When importing OPML from URLs, the app was treating the URL as a podcast feed instead of an OPML file.

## How to Test the Fix

### Test 1: Import OPML from URL (The Original Issue)

1. **Start the application**:
   ```powershell
   cargo run --release
   ```

2. **Press `Shift+A`** (or type `:import-opml`)
   - You should see: "Import OPML from (file path or URL): "

3. **Enter this URL**:
   ```
   https://www.lqdev.me/collections/podroll/index.opml
   ```

4. **Expected behavior** (FIXED):
   - ✅ Shows "Starting OPML import from: https://www.lqdev.me/collections/podroll/index.opml..."
   - ✅ Shows "Validating OPML file..."
   - ✅ Shows "Found X feeds in OPML"
   - ✅ Shows progress: "Importing [1/X]: ..."
   - ✅ Shows final summary with imported/skipped/failed counts

5. **Wrong behavior** (BEFORE FIX):
   - ❌ Shows "Adding podcast: https://www.lqdev.me/collections/podroll/index.opml..."
   - ❌ Shows "Error: Failed to subscribe to https://..."

### Test 2: Import OPML from Local File

1. **Press `Shift+A`**

2. **Enter**:
   ```
   examples/sample-subscriptions.opml
   ```

3. **Expected behavior**:
   - ✅ Validates and imports from local file
   - ✅ Shows progress for 6 feeds in sample file

### Test 3: Regular Podcast Subscription Still Works

1. **Press `a`** (not Shift+A)

2. **Enter a valid RSS feed URL**:
   ```
   https://feed.syntax.fm/rss
   ```

3. **Expected behavior**:
   - ✅ Shows "Adding podcast: https://feed.syntax.fm/rss..."
   - ✅ Successfully subscribes to the podcast

### Test 4: Export OPML

1. **Press `Shift+E`**

2. **Press Enter** (use default location)

3. **Expected behavior**:
   - ✅ Shows export progress
   - ✅ Shows "Successfully exported X feeds to ..."
   - ✅ File created with timestamped name

### Test 5: Context Switching

1. **Press `Shift+A`** (OPML import prompt)
2. **Press `Esc`** (cancel)
3. **Press `a`** (regular add podcast)
4. **Enter an RSS URL**

Expected: Should add as podcast, not try to import as OPML

## Success Criteria

✅ All 5 tests pass  
✅ No compilation errors  
✅ No runtime crashes  
✅ Clear progress feedback  
✅ Proper error messages

## Quick One-Liner Test

```powershell
# Build and run
cargo run --release

# Then in the app:
# Press Shift+A, paste this, press Enter:
https://www.lqdev.me/collections/podroll/index.opml
```

If it shows "Importing OPML..." instead of "Adding podcast...", the fix works! 🎉

---

**Status**: Ready for testing  
**Date**: October 6, 2025
