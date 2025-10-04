# Bulk Download Deletion Feature

## Overview
Added a new feature to delete all downloaded episodes and clean up the downloads folder with a single command. This provides a convenient way to free up disk space and reset the downloads state.

## Keybindings

### Primary Keybinding
- **Ctrl+X** - Delete all downloaded episodes and clean up downloads folder
  - Shows a confirmation prompt before execution
  - Requires explicit "y" or "yes" confirmation to proceed

### Alternative Commands
You can also access this feature through the command interface:
- `:delete-all-downloads` 
- `:clean-downloads`

Both commands show the same confirmation prompt.

## How It Works

### Deletion Process
1. **Confirmation Required**: Shows a clear warning prompt requiring explicit confirmation
2. **File Deletion**: Removes all downloaded episode files from the filesystem
3. **Status Update**: Updates episode status in storage from "Downloaded" back to "New"
4. **Directory Cleanup**: Removes empty podcast directories in the downloads folder
5. **UI Refresh**: Refreshes all episode lists and downloads buffer to reflect changes

### What Gets Deleted
- All episode files with status "Downloaded"
- Only actual downloaded files (not episodes marked as downloading or failed)
- Empty podcast directories in the downloads folder

### What Doesn't Get Deleted
- Podcast subscription data
- Episode metadata and descriptions
- The main downloads directory structure
- Episodes that are currently downloading
- Failed download entries (status is just cleaned up)

## Safety Features

### Confirmation Prompt
```
Delete ALL downloaded episodes? This will remove all downloaded files! (y/n)
```

### Error Handling
- Reports the number of successfully deleted files
- Shows specific error messages if any deletions fail
- Continues processing even if individual file deletions fail
- Provides feedback on partial success/failure scenarios

### Status Cleanup
The feature also cleans up inconsistent states:
- Episodes marked as "Downloaded" but with missing files
- Updates episode status appropriately after deletion

## Implementation Details

### New UI Action
- Added `DeleteAllDownloads` action to the UI action system
- Integrated with existing confirmation system using minibuffer

### Download Manager Enhancement
- New `delete_all_downloads()` method in `DownloadManager`
- Returns count of successfully deleted files
- Includes `cleanup_empty_directories()` helper method
- Proper error handling and transaction-like behavior

### Event System
- Added `AllDownloadsDeleted` and `AllDownloadsDeletionFailed` events
- Integrated with existing async event handling system
- Triggers appropriate UI refreshes after completion

## Usage Examples

### Via Keybinding
1. Press `Ctrl+X` in any buffer
2. See confirmation prompt: "Delete ALL downloaded episodes? This will remove all downloaded files! (y/n)"
3. Press `y` and `Enter` to confirm, or `n` to cancel
4. See progress message: "Deleting all downloaded episodes..."
5. See completion message: "Successfully deleted X downloaded episodes and cleaned up downloads folder"

### Via Command
1. Press `:` to open command prompt
2. Type `delete-all-downloads` or `clean-downloads`
3. Press `Enter`
4. Follow the same confirmation process as above

## Help Text Updates

The Downloads buffer help text now includes:
```
Actions:
  r         Refresh downloads list
  X         Delete selected download
  Ctrl+X    Delete ALL downloads and clean up
  c         Cancel/retry download
  o         Open downloads folder
  C         Clear completed downloads
```

## Benefits

1. **Disk Space Management**: Quick way to free up storage space
2. **Clean State**: Reset download state for fresh start
3. **Bulk Operations**: No need to delete episodes one by one
4. **Safe Operation**: Requires explicit confirmation
5. **Complete Cleanup**: Removes both files and directory structure
6. **Status Consistency**: Ensures episode statuses match filesystem state

## Technical Architecture

### Code Organization
- UI action definition in `src/ui/mod.rs`
- Keybinding in `src/ui/keybindings.rs`
- Business logic in `src/download/manager.rs`
- Event handling in `src/ui/app.rs`
- Help text in `src/ui/buffers/downloads.rs`

### Error Handling Pattern
Following the project's error handling guidelines:
- Custom error types with descriptive messages
- Proper error propagation through `Result` types
- User-friendly error messages in the UI
- Graceful degradation on partial failures

### Async Design
- Non-blocking operation using tokio spawn
- Progress feedback during operation
- Event-driven UI updates
- Maintains responsive interface during deletion

This feature enhances the podcast TUI's usability for managing downloaded content while maintaining the application's focus on safety and user experience.