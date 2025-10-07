# OPML Feature Improvements

## Issues Fixed

### 1. OPML Import Non-Destructive Behavior Enhanced

**Problem**: User reported that OPML imports were overwriting existing podcasts.

**Root Cause**: While the import logic correctly checked for duplicates, there was a potential edge case where the `subscribe()` method could return an `AlreadySubscribed` error that wasn't being properly handled as a skip.

**Solution**: 
- Enhanced the error handling in `import_opml()` to treat `AlreadySubscribed` errors as skips rather than failures
- Added defensive programming to catch any duplicate detection that happens at the `subscribe()` level
- The import flow now has two layers of duplicate protection:
  1. Pre-check with `is_subscribed()` → skips subscription attempt
  2. Subscribe-level check → if somehow reached, treats as skip

**Code Changes**: `src/podcast/subscription.rs`
- Modified the error handling in the import loop to check if an error message contains "already subscribed"
- If detected, increments `skipped` counter instead of adding to `failed` list
- Logs the skip appropriately

### 2. Help Buffer Updated with OPML Commands

**Problem**: The help buffer did not show the new OPML import/export commands and keybindings.

**Solution**: Updated the help buffer content to include:
- Command documentation for `:import-opml` and `:export-opml`
- Keybinding documentation for `Shift-A` (Import) and `Shift-E` (Export)
- New section "OPML IMPORT/EXPORT" with detailed information
- Note about non-destructive import behavior

**Code Changes**: `src/ui/buffers/help.rs`
- Added OPML commands to the COMMANDS section
- Added keybindings to the PODCAST MANAGEMENT section
- Added new OPML IMPORT/EXPORT section with full details
- Added note about import being non-destructive

### 3. Minibuffer Cancel Behavior Improved

**Problem**: The `:` keybinding might not have been working if there were pending operations.

**Solution**: 
- Enhanced the cancel operation (Esc/Ctrl-G) to clear all pending states
- Ensured that `pending_deletion` and `pending_bulk_deletion` flags are cleared when minibuffer is cancelled
- This prevents state leakage that could interfere with command prompt

**Code Changes**: `src/ui/app.rs`
- Updated `handle_minibuffer_key()` to clear `pending_deletion` and `pending_bulk_deletion` on cancel

## How OPML Import Works (For Reference)

### Non-Destructive Import Flow

1. **Parse OPML**: Validate and parse the OPML file/URL
2. **Iterate Feeds**: Process each feed sequentially
3. **Check Existing**: For each feed:
   - Call `is_subscribed(feed_url)` to check if podcast exists
   - If exists → Skip and increment `skipped` counter
   - If not exists → Proceed to subscribe
4. **Subscribe**: Attempt subscription:
   - Parse feed and download metadata
   - Save podcast and episodes
   - If fails with "already subscribed" → Treat as skip (defensive)
   - If fails with other error → Add to failed list
5. **Log Results**: Create detailed log file with:
   - Total feeds processed
   - Successfully imported count
   - Skipped (already subscribed) count
   - Failed imports with error details

### Keybindings Summary

- `Shift-A`: Show import prompt for file path or URL
- `Shift-E`: Show export prompt for output path
- `:import-opml <path>`: Import from command with path/URL
- `:export-opml <path>`: Export to command with path

### Commands Summary

- `import-opml <source>`: Import podcasts from OPML file or URL
  - Supports local file paths (with tilde expansion)
  - Supports HTTP/HTTPS URLs
  - Non-destructive (skips existing subscriptions)
  
- `export-opml <path>`: Export podcasts to OPML file
  - Defaults to configured export directory if no path specified
  - Creates timestamped filename if directory specified
  - Generates OPML 2.0 compliant XML

## Testing Recommendations

To verify the fixes work correctly:

1. **Test OPML Import Non-Destructive Behavior**:
   ```
   1. Subscribe to a podcast manually (e.g., via :add-podcast)
   2. Create an OPML file that includes this podcast
   3. Import the OPML file using Shift-A or :import-opml
   4. Verify the podcast is skipped (not re-imported or overwritten)
   5. Check the import log for "already subscribed" messages
   ```

2. **Test Help Buffer**:
   ```
   1. Press F1 or h or ? to show help
   2. Scroll down to verify OPML commands are documented
   3. Verify keybindings Shift-A and Shift-E are listed
   4. Verify the new OPML IMPORT/EXPORT section appears
   ```

3. **Test Command Prompt**:
   ```
   1. Press : (colon) key
   2. Verify minibuffer shows "M-x " prompt
   3. Try typing "import-opml" and verify it appears in suggestions
   4. Try typing "export-opml" and verify it appears
   5. Test cancelling with Esc and then using : again
   ```

## Files Modified

1. `src/podcast/subscription.rs`
   - Enhanced duplicate detection in `import_opml()`
   - Better error handling for "already subscribed" cases

2. `src/ui/buffers/help.rs`
   - Added OPML documentation to help content
   - New section with keybindings and command info

3. `src/ui/app.rs`
   - Clear pending operations on minibuffer cancel
   - Prevents state leakage

## Remaining Considerations

- Import is intentionally sequential (not parallel) to avoid overwhelming feeds and to provide clear progress feedback
- Log files are created in `~/.local/share/podcast-tui/logs/` (Linux/macOS) or `%LOCALAPPDATA%\podcast-tui\logs\` (Windows)
- Export defaults to configured directory from `config.json`
- Both import and export support tilde (`~`) expansion for home directory

## Documentation References

- OPML 2.0 Specification: http://opml.org/spec2.opml
- Implementation details: `docs/OPML_IMPLEMENTATION_SUMMARY.md`
- Feature documentation: `docs/FEATURE-OPML.md`
