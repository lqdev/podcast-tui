# Unsubscribe Downloads Deletion Feature

## Overview

This feature ensures that when a user unsubscribes from a podcast (deletes it from the podcast list), all downloaded episodes for that podcast are automatically deleted as well. This prevents orphaned downloaded files from consuming disk space after a podcast subscription is removed.

## Implementation Details

### Key Components Modified

1. **SubscriptionManager** (`src/podcast/subscription.rs`)
   - Added optional `download_manager` field for cleanup operations
   - New constructor `with_download_manager()` for automatic cleanup
   - Updated `unsubscribe()` method to delete downloads before removing podcast

2. **DownloadManager** (`src/download/manager.rs`)
   - New method `delete_podcast_downloads()` to remove all episodes for a specific podcast
   - New method `cleanup_podcast_directory_by_name()` for directory cleanup
   - Proper handling of folder cleanup even after podcast is deleted from storage

3. **App Integration** (`src/app.rs`)
   - Updated app initialization to use new `SubscriptionManager::with_download_manager()`
   - Automatic linking of download manager to subscription manager

4. **UI Events** (`src/ui/events.rs`)
   - Added `PodcastDownloadsDeleted` event for user feedback
   - Updated event handling in UI app

### Feature Behavior

**When unsubscribing from a podcast:**

1. **Download Cleanup**: 
   - Loads podcast info to determine download folder name
   - Finds all episodes marked as "Downloaded" 
   - Deletes physical files from disk
   - Updates episode status from "Downloaded" back to "New"
   - Saves updated episode metadata

2. **Directory Cleanup**:
   - Removes empty podcast-specific download directories
   - Only removes directories that are completely empty

3. **Storage Cleanup**:
   - Removes podcast from storage
   - Removes all episode metadata for the podcast

4. **Error Handling**:
   - Graceful handling when download manager is not available
   - Warning messages for partial failures (some files couldn't be deleted)
   - Unsubscribe operation continues even if download cleanup fails

### Configuration

The feature works with existing download configuration:
- Respects `use_readable_folders` setting for folder naming
- Uses podcast title or UUID for folder names as configured
- Follows existing sanitization rules for cross-platform compatibility

### User Experience

- **Transparent Operation**: Downloads are deleted automatically without user confirmation
- **Status Messages**: User receives feedback about successful deletion
- **No Interruption**: UI remains responsive during cleanup operations
- **Graceful Degradation**: Works even if download manager is unavailable

### Code Quality

- **Error Handling**: Comprehensive error handling with proper error types
- **Testing**: All existing tests pass, functionality tested via unit tests
- **Architecture**: Follows existing patterns and abstractions
- **Documentation**: Clear method documentation and inline comments
- **Safety**: Atomic operations and proper resource cleanup

## Usage

The feature activates automatically when using the standard unsubscribe functionality:

1. Select a podcast in the podcast list
2. Press 'd' to delete/unsubscribe
3. Confirm deletion when prompted
4. Downloads are automatically cleaned up

No additional configuration or user action required.

## Technical Notes

- Downloads are deleted before podcast metadata to ensure folder name can be determined
- Empty directories are cleaned up to prevent folder clutter
- Episode status is properly updated in storage for consistency
- Feature is backward compatible with existing installations