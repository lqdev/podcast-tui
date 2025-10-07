# What's New Buffer Implementation

## Overview
Implemented a new "What's New" buffer that aggregates the latest episodes from all subscribed podcasts, displaying them in reverse chronological order.

## Features Implemented

### 1. New Buffer: What's New (`whats-new`)
- **Location**: `src/ui/buffers/whats_new.rs`
- **Purpose**: Shows latest episodes across all podcasts
- **Sorting**: Episodes sorted by publication date (newest first)
- **Filtering**: Automatically excludes already-downloaded episodes
- **Deduplication**: Removes duplicate episodes that may appear in multiple feeds

### 2. Navigation & Actions
- **Scroll**: Use `C-n`/`↓` (next) and `C-p`/`↑` (previous) to navigate
- **Download**: Press `D` to download selected episode
- **Refresh**: Press `F5` to reload the episode list
- **Jump**: Use `C-a` (top) and `C-e` (bottom) for quick navigation

### 3. Episode Limit Configuration
- **Config Setting**: `ui.whats_new_episode_limit`
- **Default**: 100 episodes
- **Location**: Added to `src/config.rs` in `UiConfig` struct
- **Purpose**: Limits memory usage and improves performance

### 4. Download Integration
When an episode is downloaded from What's New:
- **Status Update**: Episode status updated in storage
- **Download Buffer**: Completed download appears in Downloads buffer
- **Auto-Refresh**: What's New buffer automatically refreshes to remove the downloaded episode
- **Episode List Sync**: All episode lists for that podcast are refreshed

### 5. Buffer Persistence
- **State Tracking**: Downloaded episodes are permanently filtered out
- **Refresh Behavior**: Refreshes when app starts and when user triggers refresh
- **Automatic Updates**: Updates when:
  - Podcasts are refreshed (single or all)
  - Episodes are downloaded
  - User presses F5 while viewing the buffer

## Implementation Details

### Core Components

#### 1. WhatsNewBuffer Structure
```rust
pub struct WhatsNewBuffer {
    id: String,                    // "whats-new"
    episodes: Vec<AggregatedEpisode>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
    subscription_manager: Option<Arc<SubscriptionManager<JsonStorage>>>,
    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,
    max_episodes: usize,
}
```

#### 2. AggregatedEpisode
```rust
struct AggregatedEpisode {
    pub podcast_id: PodcastId,
    pub podcast_title: String,
    pub episode: Episode,
}
```

### Key Methods

1. **`load_episodes()`**: Aggregates episodes from all podcasts
   - Loads all subscribed podcasts
   - Fetches episodes for each podcast
   - Filters out downloaded/downloading episodes
   - Sorts by publication date
   - Applies episode limit
   - Deduplicates by episode ID

2. **`handle_action()`**: Processes user actions
   - Navigation (up/down/top/bottom)
   - Download triggering with validation

3. **`render()`**: Displays the episode table
   - Shows: Podcast | Episode | Duration | Published
   - Highlights selected episode
   - Shows empty state with helpful message

### Integration Points

#### App Initialization (`src/ui/app.rs`)
```rust
// Create buffer during startup
self.buffer_manager.create_whats_new_buffer(
    self.subscription_manager.clone(),
    self.download_manager.clone(),
    self.config.ui.whats_new_episode_limit,
);

// Load initial data
if let Some(whats_new_buffer) = self.buffer_manager.get_whats_new_buffer_mut() {
    whats_new_buffer.load_episodes().await?;
}
```

#### Auto-Refresh Triggers
1. **On Episode Downloaded**: Removes from What's New
2. **On Podcast Refreshed**: Adds new episodes
3. **On All Podcasts Refreshed**: Updates entire list
4. **On F5 in Buffer**: Manual refresh

#### Buffer Manager Methods
- `create_whats_new_buffer()`: Initialize the buffer
- `get_whats_new_buffer_mut()`: Access for updates
- Integrated with buffer cycling

### Buffer Access

#### By ID
- Buffer ID: `"whats-new"`

#### By Aliases (in commands)
- `"new"` → `"whats-new"`
- `"whats-new"` → `"whats-new"`
- `"latest"` → `"whats-new"`

#### Command Examples
```
M-x switch-to-buffer whats-new
M-x b new
M-x buffer latest
```

## User Experience

### Display Format
```
┌─ What's New (45 episodes) ──────────────────────────────┐
│ Podcast      │ Episode                      │ Duration │ Published │
│────────────────────────────────────────────────────────────────────│
│ Tech News    │ AI Breakthroughs Today       │ 42:15    │ 2h ago    │
│ History Pod  │ Ancient Rome Part 5          │ 1:15:32  │ 5h ago    │
│ Science      │ Climate Change Update        │ 28:45    │ 1d ago    │
│ ...          │ ...                          │ ...      │ ...       │
└────────────────────────────────────────────────────────────────────┘
```

### Empty State
When no new episodes are available:
```
┌─ What's New (0 episodes) ───────────────────────────────┐
│                                                          │
│    No new episodes available.                           │
│                                                          │
│    Episodes will appear here after refreshing podcasts. │
│    Press 'R' to refresh all podcasts.                   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Status Messages
- `"Starting download: [Episode Title]"` - When download begins
- `"Episode download completed successfully"` - On completion
- `"What's New refreshed"` - After F5 refresh
- `"Found X new episode(s)"` - After podcast refresh

## Files Modified

### New Files
1. `src/ui/buffers/whats_new.rs` - Main buffer implementation

### Modified Files
1. `src/ui/buffers/mod.rs` - Added module and methods
2. `src/ui/app.rs` - Integration and event handling
3. `src/config.rs` - Added episode limit configuration
4. `src/utils/time.rs` - Added `format_relative_time()` helper

## Testing

### Manual Testing Steps
1. Start the app with subscribed podcasts
2. Navigate to What's New buffer (`C-x b whats-new`)
3. Verify episodes appear in reverse chronological order
4. Select an episode and press `D` to download
5. Verify episode disappears after download completes
6. Verify episode appears in Downloads buffer
7. Refresh a podcast and verify new episodes appear
8. Press F5 to manually refresh the buffer

### Edge Cases Handled
- No subscribed podcasts (empty state)
- All episodes already downloaded (empty state)
- Episode with no audio URL (shows error)
- Episode already downloading (shows message)
- Duplicate episodes across feeds (deduplicated)
- Large numbers of episodes (limited by config)

## Configuration

### Default Settings
```json
{
  "ui": {
    "whats_new_episode_limit": 100
  }
}
```

### Customization
Users can adjust the episode limit in their config file:
```json
{
  "ui": {
    "whats_new_episode_limit": 200
  }
}
```

## Architecture Decisions

### Why Aggregated Episodes?
- Combines podcast metadata with episode data for efficient rendering
- Avoids repeated podcast lookups during display
- Enables sorting across all podcasts

### Why Filter Downloaded Episodes?
- Focuses on new content discovery
- Reduces clutter in the view
- Matches user expectation: "What's New" = "What I Haven't Downloaded"

### Why Episode Limit?
- Performance: Prevents loading thousands of episodes
- Memory: Reduces RAM usage
- UX: Makes scrolling manageable
- Configurable: Power users can increase if needed

### Why Not Pagination?
- MVP focus: Simple implementation
- Episode limit provides sufficient discovery
- Can be added later if needed

## Future Enhancements (Not in MVP)

1. **Mark as Read**: Hide episodes without downloading
2. **Custom Filters**: By podcast, date range, or duration
3. **Search**: Find episodes by title or description
4. **Sorting Options**: Duration, podcast name, etc.
5. **Episode Details**: View full description in popup
6. **Bulk Download**: Select multiple episodes
7. **Pagination**: Load episodes in batches
8. **Date Range Filter**: Only show last N days

## Benefits

1. **Content Discovery**: See all new content in one place
2. **Efficiency**: Don't need to check each podcast individually
3. **Consistency**: Same `D` keybinding as episode lists
4. **Automatic Updates**: Always current with latest episodes
5. **Performance**: Limited episode count keeps UI responsive
6. **State Management**: Downloads properly tracked and synced

## Follows Project Guidelines

✅ **Storage Abstraction**: Uses Storage trait, not direct JSON  
✅ **Component Separation**: UI, business logic, and storage are separate  
✅ **Event-Driven**: Uses event system for async operations  
✅ **Buffer-Based UI**: Integrates with existing buffer system  
✅ **Async-First**: All I/O operations are async  
✅ **Error Handling**: Proper Result types, no unwraps  
✅ **MVP Focus**: Simple, working implementation  
✅ **Documentation**: Comprehensive comments and help text  

## Known Limitations (MVP)

1. No visual indication of which episodes are new since last view
2. No way to mark episodes as "seen" without downloading
3. Episode limit applies globally, not per-podcast
4. No custom sorting options
5. No filtering by podcast or date range

These are acceptable trade-offs for the MVP and can be addressed in future iterations.
