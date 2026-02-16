# Search & Filter — Design Document

**Created**: February 16, 2026  
**Status**: Design Phase  
**Sprint**: 5 (Enhanced Features)  
**Priority**: P1 (Episode filtering), P2 (Text search)  
**Branch**: `feat/search-and-filter`

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Research Summary](#research-summary)
3. [Requirements](#requirements)
4. [Architecture & Design](#architecture--design)
5. [Detailed Implementation Plan](#detailed-implementation-plan)
6. [Keybinding Design](#keybinding-design)
7. [UI/UX Design](#uiux-design)
8. [Data Flow](#data-flow)
9. [Testing Strategy](#testing-strategy)
10. [Migration & Compatibility](#migration--compatibility)
11. [Decision Log](#decision-log)

---

## Executive Summary

This document describes the design for search and filter functionality in Podcast TUI. The feature allows users to:

1. **Filter episodes** by status (new/downloaded/played), date range, and duration within any episode list buffer
2. **Search episodes** by text (title, description) across a single podcast or across all subscribed podcasts
3. **Filter the podcast list** by text search

The design follows the existing Emacs-style buffer architecture, leverages the minibuffer for input, and introduces **inline filtering** (narrowing the view within existing buffers) rather than creating separate search-result buffers. This approach is simpler, more discoverable, and consistent with the Emacs philosophy of narrowing/widening views.

---

## Research Summary

### Approach Evaluation

Two architectural approaches were evaluated:

#### Option A: Dedicated Search Buffer (Rejected)
- Creates a new `SearchBuffer` that displays cross-podcast search results
- Pros: Clean separation, no modification of existing buffers
- **Cons**: Duplicates episode rendering logic; breaks mental model ("where did my episode list go?"); harder to act on results (download, open detail) without re-wiring all actions; doesn't handle per-buffer filtering naturally

#### Option B: Inline Filtering (Selected ✅)
- Adds filter state to existing buffers (`EpisodeListBuffer`, `WhatsNewBuffer`, `PodcastListBuffer`)
- User activates filter/search mode, types query, list narrows in-place
- Pros: No new buffer type needed; all existing actions (download, detail, etc.) work unchanged on filtered results; consistent with Emacs narrowing; simpler implementation
- **Cons**: Requires modifying existing buffer structs (but changes are additive and non-breaking)

### TUI Search Pattern Research

Research into popular Rust TUI applications (spotify-tui, spotify-player) and Ratatui documentation revealed common patterns:

1. **Input Mode Toggle**: Applications switch between "normal" mode and "search/input" mode via a keybinding (typically `/`). In search mode, keystrokes go to the search input rather than navigation.

2. **Incremental Filtering**: Preferred UX is to filter the list as the user types (incremental/live search), not requiring Enter to submit. This gives immediate visual feedback.

3. **Filter Indicators**: When a filter is active, the UI should clearly indicate this — typically in the buffer title/border or a status line — so users know they're seeing a subset.

4. **Escape to Clear**: `Esc` closes the search input and clears the filter, returning to the full list. This is universal across TUI apps.

5. **Minibuffer Integration**: This project already has a sophisticated minibuffer with prompt, input, history, and completion support. The search input should use the minibuffer for text entry to maintain consistency.

### Existing Codebase Integration Points

The codebase is well-structured for this feature:

- **Minibuffer** already handles text input, cursor, history, and prompt display
- **`handle_minibuffer_key`** in `UIApp` intercepts all keyboard input when minibuffer is in input mode
- **`handle_minibuffer_input_with_context`** dispatches submitted input based on the prompt context string
- **Each buffer's `handle_action`** already returns `UIAction` for chaining
- **Buffer rendering** is self-contained — each buffer controls its own list display
- **`UIAction` enum** is easily extensible for new action variants
- **Data is already in-memory** in buffers after loading — filtering is a simple `Vec::iter().filter()` operation

---

## Requirements

### From PRD (docs/PRD.md)

| Priority | Feature | PRD Reference |
|----------|---------|---------------|
| P1 | Episode filtering by status (downloaded/played/new) | Should Have |
| P1 | Date range filtering (last 7d, 30d, etc.) | Should Have |
| P1 | Duration-based filtering (short/medium/long) | Should Have |
| P2 | Basic text search across episodes (title + description) | Could Have |
| P2 | Note search functionality | Could Have |

### From Implementation Plan (docs/IMPLEMENTATION_PLAN.md)

Sprint 5, Days 3-4:
- Episode filtering by status (downloaded/played)
- Date range filtering
- Duration-based filtering
- Basic text search across episodes

### Functional Requirements

1. **FR-1**: Users can filter episodes by status (`new`, `downloaded`, `played`, `downloading`, `failed`) in `EpisodeListBuffer` and `WhatsNewBuffer`
2. **FR-2**: Users can filter episodes by date range (`today`, `7d`, `30d`, `90d`, `all`) in any episode view
3. **FR-3**: Users can filter episodes by duration category (`short` <15min, `medium` 15-45min, `long` >45min) in any episode view
4. **FR-4**: Users can search by text (case-insensitive substring match against title and description) in `EpisodeListBuffer`, `WhatsNewBuffer`, and `PodcastListBuffer`
5. **FR-5**: Filters can be combined (e.g., status=downloaded AND date=7d AND text="interview")
6. **FR-6**: Active filters are clearly indicated in the buffer title/chrome
7. **FR-7**: Filters can be cleared individually or all at once
8. **FR-8**: Cursor position resets to the first item when a filter is applied
9. **FR-9**: All existing actions (download, delete, open detail, etc.) work correctly on filtered results

### Non-Functional Requirements

1. **NFR-1**: Filtering is instantaneous (client-side, no I/O) — all data is already in memory
2. **NFR-2**: No regressions — existing keybindings, navigation, and buffer behavior unchanged
3. **NFR-3**: Filter state survives buffer refresh (when episodes are reloaded, active filters are re-applied)
4. **NFR-4**: Cross-platform — no platform-specific input handling

---

## Architecture & Design

### Core Concept: FilterState

A new `FilterState` struct encapsulates all filter criteria. It is added to each buffer that supports filtering.

```rust
/// Filter criteria for episode lists
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EpisodeFilter {
    /// Text search query (matches title, description, notes)
    pub text_query: Option<String>,
    
    /// Filter by episode status
    pub status: Option<EpisodeStatusFilter>,
    
    /// Filter by date range
    pub date_range: Option<DateRangeFilter>,
    
    /// Filter by duration category
    pub duration: Option<DurationFilter>,
}

/// Status filter options
#[derive(Debug, Clone, PartialEq)]
pub enum EpisodeStatusFilter {
    New,
    Downloaded,
    Played,
    Downloading,
    DownloadFailed,
}

/// Date range filter options
#[derive(Debug, Clone, PartialEq)]
pub enum DateRangeFilter {
    Today,
    Last7Days,
    Last30Days,
    Last90Days,
    LastYear,
}

/// Duration filter categories
#[derive(Debug, Clone, PartialEq)]
pub enum DurationFilter {
    /// Under 15 minutes
    Short,
    /// 15 to 45 minutes
    Medium,
    /// Over 45 minutes
    Long,
}

/// Filter for podcast list (simpler — text only)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PodcastFilter {
    pub text_query: Option<String>,
}
```

### Filter Application Strategy

Buffers maintain two lists:
1. `episodes: Vec<Episode>` — the **full** unfiltered dataset (source of truth)
2. `filtered_indices: Vec<usize>` — indices into the full list that match current filters

When filters change, `filtered_indices` is recomputed. All rendering and navigation operate on `filtered_indices`, but the underlying data remains untouched.

This approach:
- Preserves the full dataset for instant filter changes
- Avoids cloning episodes for filtered views
- Keeps index mapping simple for actions (download, detail, etc.)
- Re-applies filters automatically when `set_episodes()` is called during refresh

```rust
impl EpisodeListBuffer {
    /// Recompute filtered indices based on current filter state
    fn apply_filters(&mut self) {
        self.filtered_indices = self.episodes
            .iter()
            .enumerate()
            .filter(|(_, ep)| self.filter.matches(ep))
            .map(|(i, _)| i)
            .collect();
        
        // Reset cursor to first filtered item
        self.selected_index = if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        };
        self.scroll_offset = 0;
    }
}
```

### Integration with Existing Buffer Pattern

The changes to each buffer are **additive**:

```rust
pub struct EpisodeListBuffer {
    // ... existing fields unchanged ...
    
    // NEW: Filter support
    filter: EpisodeFilter,
    filtered_indices: Vec<usize>,
}
```

Navigation methods (`select_next`, `select_previous`) are updated to use `filtered_indices` instead of direct `episodes` indexing. The `render` method iterates `filtered_indices` to build the visible list.

### New Module: `src/ui/filters.rs`

A new module contains the filter types, matching logic, and display helpers:

```
src/ui/
├── filters.rs          # NEW: Filter types, matching, display
├── mod.rs              # Updated: pub mod filters
├── app.rs              # Updated: handle filter actions
├── keybindings.rs      # Updated: add / keybinding
└── buffers/
    ├── episode_list.rs # Updated: add filter state + filtered rendering
    ├── whats_new.rs    # Updated: add filter state + filtered rendering
    └── podcast_list.rs # Updated: add text search filter
```

### New UIAction Variants

```rust
pub enum UIAction {
    // ... existing variants ...
    
    /// Activate text search in the current buffer (opens minibuffer for input)
    SearchInBuffer,
    
    /// Apply a text search query to the active buffer
    ApplySearch { query: String },
    
    /// Clear search/filter in the active buffer  
    ClearFilters,
    
    /// Cycle through status filters in the active buffer
    FilterByStatus,
    
    /// Set a specific status filter
    SetStatusFilter { status: Option<EpisodeStatusFilter> },
    
    /// Cycle through date range filters
    FilterByDateRange,
    
    /// Set a specific date range filter
    SetDateRangeFilter { range: Option<DateRangeFilter> },
    
    /// Cycle through duration filters
    FilterByDuration,
    
    /// Set a specific duration filter
    SetDurationFilter { duration: Option<DurationFilter> },
}
```

### New Commands (Minibuffer / `:` prompt)

| Command | Description | Example |
|---------|-------------|---------|
| `search <query>` | Text search in current buffer | `search interview` |
| `filter-status <status>` | Filter by status | `filter-status downloaded` |
| `filter-date <range>` | Filter by date range | `filter-date 7d` |
| `filter-duration <category>` | Filter by duration | `filter-duration short` |
| `clear-filters` / `widen` | Remove all filters | `clear-filters` |

---

## Detailed Implementation Plan

### Phase 1: Core Filter Infrastructure (Estimated: ~200 LOC)

**File: `src/ui/filters.rs` (new)**

1. Define `EpisodeFilter`, `PodcastFilter`, `EpisodeStatusFilter`, `DateRangeFilter`, `DurationFilter` types
2. Implement `EpisodeFilter::matches(&self, episode: &Episode) -> bool`
3. Implement `EpisodeFilter::is_active(&self) -> bool`
4. Implement `EpisodeFilter::description(&self) -> String` (for status bar / title display)
5. Implement `EpisodeFilter::clear(&mut self)`
6. Implement `Display` for filter enums
7. Implement parsing functions: `parse_status_filter(s: &str)`, `parse_date_range(s: &str)`, `parse_duration_filter(s: &str)`
8. Unit tests for all matching logic

### Phase 2: UIAction Extensions (~30 LOC)

**File: `src/ui/mod.rs`**

1. Add new `UIAction` variants: `SearchInBuffer`, `ApplySearch`, `ClearFilters`, `FilterByStatus`, `FilterByDateRange`, `FilterByDuration`, `SetStatusFilter`, `SetDateRangeFilter`, `SetDurationFilter`

### Phase 3: EpisodeListBuffer Filter Integration (~150 LOC)

**File: `src/ui/buffers/episode_list.rs`**

1. Add `filter: EpisodeFilter` and `filtered_indices: Vec<usize>` fields
2. Add `apply_filters()` method
3. Update `set_episodes()` to call `apply_filters()` after setting data
4. Update `select_next()` / `select_previous()` to navigate `filtered_indices`
5. Update `selected_episode()` to use `filtered_indices` for index mapping
6. Update `render()` to iterate `filtered_indices` and show filter indicator in title
7. Add `handle_action` arms for filter actions
8. Update `help_text()` with filter keybindings
9. Update status line to show "X of Y episodes (Z matching)" when filtered
10. Update existing tests; add filter-specific tests

### Phase 4: WhatsNewBuffer Filter Integration (~120 LOC)

**File: `src/ui/buffers/whats_new.rs`**

1. Same pattern as EpisodeListBuffer but adapted for `AggregatedEpisode`
2. Filter matches against `aggregated_episode.episode` fields
3. Update rendering to show filter indicator
4. Add `handle_action` arms for filter actions

### Phase 5: PodcastListBuffer Search Integration (~80 LOC)

**File: `src/ui/buffers/podcast_list.rs`**

1. Add `filter: PodcastFilter` and `filtered_indices: Vec<usize>` fields
2. Implement text search matching against podcast title, author, description
3. Update navigation and rendering to use `filtered_indices`
4. Show search indicator in title bar

### Phase 6: Keybinding & Command Wiring (~100 LOC)

**File: `src/ui/keybindings.rs`**

1. Bind `/` → `UIAction::SearchInBuffer`
2. Bind `Ctrl+/` or `F6` → `UIAction::ClearFilters` (fallback if Ctrl+/ doesn't work in some terminals)

**File: `src/ui/app.rs`**

1. Handle `UIAction::SearchInBuffer`: show minibuffer prompt "Search: " and set context
2. Handle `UIAction::ApplySearch`: dispatch to active buffer
3. Handle `UIAction::ClearFilters`: dispatch to active buffer
4. Handle `UIAction::FilterByStatus` / `FilterByDateRange` / `FilterByDuration`: show minibuffer with completion candidates
5. Add `search`, `filter-status`, `filter-date`, `filter-duration`, `clear-filters`, `widen` to `execute_command_direct`
6. Add these commands to `get_available_commands()` for tab completion
7. Handle search prompt context in `handle_minibuffer_input_with_context`

### Phase 7: Integration Tests (~100 LOC)

1. Test filter application and clearing in `EpisodeListBuffer`
2. Test combined filters (status + text)
3. Test that filtered navigation works (up/down on filtered list)
4. Test that actions work on filtered items (download correct episode)
5. Test filter survives `set_episodes()` refresh
6. Test filter display strings

---

## Keybinding Design

### New Keybindings

| Key | Action | Context | Rationale |
|-----|--------|---------|-----------|
| `/` | Open search/text filter input | Any filterable buffer | Universal "search" key (vim, less, man, etc.) |
| `F6` | Clear all filters | Any filterable buffer | Available F-key, safe across terminals |

### Filter Commands via `:` prompt

Structured filters (status, date, duration) are accessed via the `:` command prompt to avoid consuming too many single-key bindings. This keeps the keybinding space clean for Sprint 4 playback controls.

| Command | Tab-Completable | Behavior |
|---------|-----------------|----------|
| `:search <text>` | Yes | Text search (same as `/`) |
| `:filter-status` | Yes, cycles: `new`, `downloaded`, `played`, `downloading`, `failed` | Set status filter |
| `:filter-date` | Yes, cycles: `today`, `7d`, `30d`, `90d`, `year` | Set date range filter |
| `:filter-duration` | Yes, cycles: `short`, `medium`, `long` | Set duration filter |
| `:clear-filters` / `:widen` | Yes | Remove all active filters |

### Keybinding Conflict Analysis

- `/` is currently **unbound** — safe to use
- `F6` through `F9` are currently **unbound** — safe to use
- No conflict with planned Sprint 4 playback keys (`Space`, `s`, `[`, `]`, `+`, `-`, `n`, `p`)
- No conflict with existing navigation, buffer, or podcast management keys

### Search Input Flow

```
User presses '/' in EpisodeListBuffer:
  1. UIApp receives UIAction::SearchInBuffer
  2. UIApp calls minibuffer.show_prompt("Search: ", vec![])
  3. Minibuffer enters input mode → keystrokes go to minibuffer
  4. User types search text, sees it in minibuffer
  5a. User presses Enter → minibuffer.submit() → UIApp receives input
      → UIApp dispatches ApplySearch { query } to active buffer
      → Buffer applies text filter, re-renders
  5b. User presses Esc → minibuffer.clear() → search cancelled
  6. Filter indicator appears in buffer title: "Episodes: My Podcast [search: interview]"
  7. User presses F6 or runs :clear-filters → filters cleared, full list restored
```

---

## UI/UX Design

### Filter Indicator in Buffer Title

When filters are active, the buffer title is augmented:

```
┌─ Episodes: My Podcast [filtered: downloaded, 30d] ──────────────┐
│ ● Episode about Rust                                              │
│ ● Episode about Go                                                │
│                                                                    │
│                                                                    │
└──────────────── 2 of 45 episodes (2 matching) ───────────────────┘
```

For text search:
```
┌─ Episodes: My Podcast [search: "rust"] ──────────────────────────┐
```

For combined filters:
```
┌─ Episodes: My Podcast [search: "rust", status: downloaded] ──────┐
```

### Status Line Update

The bottom-right status indicator changes from:
- Unfiltered: `3 of 45 episodes`
- Filtered: `1 of 5 matching (45 total)`

### Empty State

When filters match no episodes:
```
┌─ Episodes: My Podcast [search: "xyzzy"] ─────────────────────────┐
│                                                                    │
│              No episodes match the current filter.                 │
│           Press F6 or run :clear-filters to reset.                 │
│                                                                    │
└──────────────── 0 matching (45 total) ────────────────────────────┘
```

### Podcast List Search

```
┌─ Podcasts [search: "rust"] ──────────────────────────────────────┐
│   Rustacean Station - Allen Wyma                                  │
│   New Rustacean - Chris Krycho                                    │
│                                                                   │
└──────────────── 2 of 15 podcasts (2 matching) ───────────────────┘
```

---

## Data Flow

### Filter Application Flow

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  User Input      │────>│  UIApp           │────>│  Active Buffer   │
│  / or :search    │     │  handle_action() │     │  apply_filters() │
└──────────────────┘     └──────────────────┘     └──────────────────┘
                                                          │
                                                          ▼
                                                  ┌──────────────────┐
                                                  │ filtered_indices │
                                                  │ recomputed       │
                                                  │ cursor reset     │
                                                  │ render() called  │
                                                  └──────────────────┘
```

### Filter Persistence Through Refresh

```
Background refresh triggers set_episodes(new_data):
  1. self.episodes = new_data (sorted)
  2. if self.filter.is_active():
       self.apply_filters()  // re-compute filtered_indices
     else:
       self.filtered_indices = (0..self.episodes.len()).collect()
  3. Cursor position adjusted to stay in bounds
```

### Index Mapping for Actions

```
User selects item at display position 2 in filtered list:
  → filtered_indices[2] = 15  (actual index in self.episodes)
  → self.episodes[15] = the correct Episode
  → Actions (download, detail, etc.) get the right episode
```

---

## Testing Strategy

### Unit Tests (in `src/ui/filters.rs`)

1. **Text matching**: case-insensitive substring match on title, description, notes
2. **Status filter**: each status variant matches/rejects correctly
3. **Date range filter**: episodes within/outside each range
4. **Duration filter**: short/medium/long boundary conditions
5. **Combined filters**: AND logic across multiple criteria
6. **Empty query**: returns all items
7. **Filter description**: correct human-readable strings
8. **Parse functions**: valid and invalid input handling

### Integration Tests (in buffer test modules)

1. **Filter + navigation**: up/down on filtered list skips correct items
2. **Filter + actions**: download/detail on filtered item targets correct episode
3. **Filter + refresh**: filter re-applied after `set_episodes()`
4. **Filter + cursor**: cursor resets to 0 on new filter, adjusts if filtered results shrink
5. **Clear filter**: restores full list, resets cursor
6. **Empty results**: proper empty state rendering
7. **WhatsNew filter**: works across aggregated episodes from multiple podcasts
8. **PodcastList filter**: text search matches title, author

### Manual Testing Checklist

- [ ] Press `/`, type text, see list narrow in real-time (on Enter)
- [ ] Press `Esc` during search to cancel
- [ ] Run `:filter-status downloaded`, see only downloaded episodes
- [ ] Combine text search + status filter
- [ ] Press `F6` to clear all filters
- [ ] Filter in WhatsNew buffer
- [ ] Filter in PodcastList buffer
- [ ] Download an episode from filtered results (correct episode downloads)
- [ ] Open episode detail from filtered results (correct episode opens)
- [ ] Refresh podcast while filter is active (filter re-applies to new data)
- [ ] Navigate filtered list (up/down, Home/End, Page Up/Down)
- [ ] Check filter indicator in title bar
- [ ] Check status line shows "X matching (Y total)"

---

## Migration & Compatibility

### Breaking Changes: None

- All changes are additive
- No existing struct fields removed
- No existing functions signatures changed
- No existing keybindings moved or removed
- Default filter state is empty (inactive) — same as current behavior

### Backward Compatibility

- Users who never use `/` or filter commands see no change
- All existing episode rendering, navigation, and actions work identically when no filter is active
- `filtered_indices` defaults to all indices when no filter is set

---

## Decision Log

| # | Decision | Rationale | Date |
|---|----------|-----------|------|
| 1 | Inline filtering over dedicated SearchBuffer | Simpler UX, no action re-wiring, Emacs narrowing philosophy, preserves existing buffer actions | 2026-02-16 |
| 2 | `/` for search keybinding | Universal convention (vim, less, man, etc.), currently unbound, single keystroke | 2026-02-16 |
| 3 | Submit-on-Enter (not incremental live search) | Simpler implementation; consistent with existing minibuffer input model; avoids complexity of dispatching per-keystroke to buffer while minibuffer owns input | 2026-02-16 |
| 4 | Status/date/duration filters via `:` commands | Preserves single-key space for playback (Sprint 4); provides tab-completion UX; consistent with existing command system | 2026-02-16 |
| 5 | Filter indices approach over cloning filtered data | Memory efficient; preserves original data for instant filter changes; simpler action dispatch (index mapping) | 2026-02-16 |
| 6 | `F6` for clear-filters | Available F-key; works across all terminals; single-keystroke convenience for common action | 2026-02-16 |
| 7 | AND logic for combined filters | Most intuitive for narrowing results; consistent with Emacs narrowing | 2026-02-16 |
| 8 | `src/ui/filters.rs` as new module | Keeps filter logic separate from buffer/component code; reusable across buffers; testable in isolation | 2026-02-16 |

---

## Appendix: File Change Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `src/ui/filters.rs` | **NEW** | Filter types, matching logic, display helpers |
| `src/ui/mod.rs` | Modified | Add `pub mod filters`, new `UIAction` variants |
| `src/ui/keybindings.rs` | Modified | Bind `/` → `SearchInBuffer`, `F6` → `ClearFilters` |
| `src/ui/app.rs` | Modified | Handle new actions, add commands, minibuffer integration |
| `src/ui/buffers/episode_list.rs` | Modified | Add filter state, filtered rendering, filter actions |
| `src/ui/buffers/whats_new.rs` | Modified | Add filter state, filtered rendering, filter actions |
| `src/ui/buffers/podcast_list.rs` | Modified | Add text search filter |
| `src/ui/buffers/mod.rs` | Modified | (If needed) trait extension for filter support |
| `docs/KEYBINDINGS.md` | Modified | Document new keybindings and commands |
| `docs/SEARCH_AND_FILTER.md` | **NEW** | This document |

**Estimated total lines of code**: ~800-1000 (including tests)  
**Estimated implementation time**: 2-3 days

---

**Document Version**: 1.0  
**Author**: GitHub Copilot  
**Reviewers**: Project maintainer
