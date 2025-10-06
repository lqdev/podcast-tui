# Buffer Scrolling Fix

## Issue Fixed

**Problem**: Navigation with arrow keys in the Podcast List buffer didn't scroll the view when the selected item went beyond the visible area. The cursor/selection indicator was not visible, and users couldn't see items at the bottom of long lists.

**Root Cause**: The `PodcastListBuffer` and `DownloadsBuffer` were rendering all items without implementing scroll offset tracking. They used Ratatui's `List` widget but didn't adjust which items were visible based on the selected index.

## Solution

Implemented proper scroll offset management for both affected buffers following the same pattern already used successfully in `EpisodeListBuffer` and `WhatsNewBuffer`.

### Changes Made

#### 1. PodcastListBuffer (`src/ui/buffers/podcast_list.rs`)

**Added scroll tracking**:
- Added `scroll_offset: usize` field to track which item is at the top of the visible area
- Implemented `adjust_scroll()` method to automatically keep selected item visible
- Modified `render()` to only display visible items within the viewport
- Added Page Up/Page Down support (moves by 10 items)

**How it works**:
```rust
// Calculate visible height (subtract borders)
let visible_height = chunks[0].height.saturating_sub(2) as usize;

// Adjust scroll to keep selected item visible
self.adjust_scroll(visible_height);

// Only render the visible range
let end_index = (self.scroll_offset + visible_height).min(self.podcasts.len());
let visible_podcasts = &self.podcasts[self.scroll_offset..end_index];
```

**Navigation improvements**:
- Arrow keys (↑/↓) now properly scroll the view
- Page Up/Down jumps by 10 items
- Home/End goes to first/last item and resets scroll
- Selection is always visible in the viewport

#### 2. DownloadsBuffer (`src/ui/buffers/downloads.rs`)

Applied the same scroll offset pattern to prevent the same issue in the Downloads buffer:
- Added `scroll_offset: usize` field
- Implemented `adjust_scroll()` method
- Modified `render()` to only display visible downloads
- Added Page Up/Page Down support

### Technical Details

**The adjust_scroll algorithm**:
```rust
fn adjust_scroll(&mut self, visible_height: usize) {
    if let Some(selected) = self.selected_index {
        // If selected item is above visible area, scroll up
        if selected < self.scroll_offset {
            self.scroll_offset = selected;
        }
        // If selected item is below visible area, scroll down
        else if selected >= self.scroll_offset + visible_height {
            self.scroll_offset = selected.saturating_sub(visible_height - 1);
        }
    }
}
```

This ensures:
1. Selected item is never off-screen
2. Smooth scrolling when navigating with arrow keys
3. No jarring jumps - only scrolls when necessary
4. Works with any terminal size (responsive)

### Buffers Status

| Buffer | Scroll Support | Status |
|--------|---------------|--------|
| PodcastListBuffer | ✅ Fixed | Previously broken, now working |
| DownloadsBuffer | ✅ Fixed | Preventively fixed (same issue) |
| EpisodeListBuffer | ✅ Working | Already had scroll support |
| WhatsNewBuffer | ✅ Working | Already had scroll support |
| HelpBuffer | ✅ Working | Has its own scroll implementation |
| BufferListBuffer | N/A | Typically short list, no scroll needed |

### Testing

✅ **Code compiles**: `cargo check` passes with only warnings (no errors)  
✅ **All tests pass**: 71/71 tests passing  
✅ **No breaking changes**: Existing functionality preserved

### User Experience Improvements

**Before**:
- Navigating past visible items was confusing
- Selected item indicator disappeared off-screen
- No way to see items at the bottom of long lists
- Page Up/Down not supported

**After**:
- Selected item always visible
- Smooth scrolling keeps selection on screen
- Can navigate through any length list
- Page Up/Down for faster navigation (10 items at a time)
- Terminal resizing handled correctly

### Navigation Reference

```
Basic Navigation:
  ↑/↓         Move selection up/down (with auto-scroll)
  Page Up     Jump up 10 items
  Page Down   Jump down 10 items
  Home        Jump to first item
  End         Jump to last item

Works in:
  - Podcast List (F2)
  - Downloads Buffer (F4)
  - Episode Lists
  - What's New Buffer
```

## Code Quality

- Follows existing patterns from `EpisodeListBuffer`
- Consistent with project coding guidelines
- Maintains separation of concerns
- No unwrap() or expect() in production code
- Proper bounds checking with saturating operations

## Files Modified

1. `src/ui/buffers/podcast_list.rs`
   - Added scroll_offset field
   - Added adjust_scroll() method
   - Modified render() for viewport-based rendering
   - Added Page Up/Down handlers

2. `src/ui/buffers/downloads.rs`
   - Added scroll_offset field
   - Added adjust_scroll() method
   - Modified render() for viewport-based rendering
   - Added Page Up/Down handlers

## Related Documentation

- Ratatui List widget: https://docs.rs/ratatui/latest/ratatui/widgets/struct.List.html
- Buffer abstraction: `src/ui/buffers/mod.rs`
- UI component trait: `src/ui/mod.rs`
